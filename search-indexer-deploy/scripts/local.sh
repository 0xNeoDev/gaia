#!/bin/bash
# Local development script for the search stack using Minikube
#
# Usage:
#   ./search-indexer-deploy/scripts/local.sh start     # Start minikube and deploy OpenSearch
#   ./search-indexer-deploy/scripts/local.sh stop      # Stop the search stack
#   ./search-indexer-deploy/scripts/local.sh delete    # Delete the search namespace
#   ./search-indexer-deploy/scripts/local.sh status    # Check status
#   ./search-indexer-deploy/scripts/local.sh logs      # View OpenSearch logs
#   ./search-indexer-deploy/scripts/local.sh health    # Check cluster health
#   ./search-indexer-deploy/scripts/local.sh port-forward  # Forward ports to localhost
#   ./search-indexer-deploy/scripts/local.sh dashboard  # Open OpenSearch Dashboards
#   ./search-indexer-deploy/scripts/local.sh grafana   # Open Grafana dashboards (port 4040)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(dirname "$SCRIPT_DIR")"
OVERLAY_DIR="$DEPLOY_DIR/overlays/local"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

function print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

function print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

function print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

function print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

function check_minikube() {
    if ! command -v minikube &> /dev/null; then
        print_error "minikube is not installed"
        echo "Install with: brew install minikube"
        exit 1
    fi
}

function check_kubectl() {
    if ! command -v kubectl &> /dev/null; then
        print_error "kubectl is not installed"
        echo "Install with: brew install kubectl"
        exit 1
    fi
}

function ensure_minikube_running() {
    check_minikube
    check_kubectl
    
    if ! minikube status &> /dev/null; then
        print_info "Starting minikube..."
        # Start with enough memory for OpenSearch (2GB) + Kafka + other services
        minikube start --memory=6144 --cpus=4 --driver=docker
    else
        print_info "minikube is already running"
    fi
}

function start_search() {
    ensure_minikube_running
    
    print_step "Deploying OpenSearch stack + monitoring to minikube..."
    
    # Apply the local overlay which includes resource limits for local dev
    kubectl apply -k "$OVERLAY_DIR"
    
    print_step "Waiting for OpenSearch to be ready..."
    echo "This may take a few minutes on first start..."
    
    # Wait for the pod to be created
    sleep 5
    
    # Wait for OpenSearch to be ready
    if kubectl -n search wait --for=condition=ready pod -l app=opensearch --timeout=300s; then
        print_info "OpenSearch is ready!"
    else
        print_warn "OpenSearch may still be starting. Check logs with: $0 logs"
    fi
    
    # Wait for monitoring stack
    print_step "Waiting for monitoring stack..."
    kubectl -n search wait --for=condition=ready pod -l app=grafana --timeout=120s 2>/dev/null || true
    kubectl -n search wait --for=condition=ready pod -l app=prometheus --timeout=120s 2>/dev/null || true
    kubectl -n search wait --for=condition=ready pod -l app=opensearch-exporter --timeout=120s 2>/dev/null || true
    
    echo ""
    print_info "To access services locally, run:"
    echo "  $0 port-forward"
    echo ""
    print_info "Services available:"
    echo "  OpenSearch:  http://localhost:9200"
    echo "  Dashboards:  http://localhost:5601"
    echo "  Grafana:     http://localhost:4040 (admin/admin)"
    echo "  Prometheus:  http://localhost:9090"
    echo ""
    print_info "Then run the search indexer with:"
    echo "  OPENSEARCH_URL=http://localhost:9200 cargo run -p search-indexer"
}

function stop_search() {
    check_kubectl
    print_info "Scaling down search stack..."
    kubectl -n search scale statefulset opensearch --replicas=0 2>/dev/null || true
    kubectl -n search scale deployment opensearch-dashboards --replicas=0 2>/dev/null || true
    kubectl -n search scale deployment search-indexer --replicas=0 2>/dev/null || true
    kubectl -n search scale deployment grafana --replicas=0 2>/dev/null || true
    kubectl -n search scale deployment prometheus --replicas=0 2>/dev/null || true
    kubectl -n search scale deployment opensearch-exporter --replicas=0 2>/dev/null || true
    print_info "Search stack stopped. Data is preserved."
    print_info "Use 'delete' to remove completely, or 'start' to restart."
}

function delete_search() {
    check_kubectl
    print_warn "This will delete all search data!"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Deleting search namespace..."
        kubectl delete namespace search --ignore-not-found=true
        print_info "Search stack deleted."
    else
        print_info "Cancelled."
    fi
}

function show_status() {
    check_kubectl
    echo ""
    print_info "Minikube status:"
    minikube status 2>/dev/null || echo "Not running"
    echo ""
    print_info "Search namespace pods:"
    kubectl -n search get pods 2>/dev/null || echo "Namespace not found"
    echo ""
    print_info "Search namespace services:"
    kubectl -n search get svc 2>/dev/null || echo "Namespace not found"
}

function show_logs() {
    check_kubectl
    print_info "Streaming OpenSearch logs (Ctrl+C to stop)..."
    kubectl -n search logs -f -l app=opensearch
}

function port_forward() {
    check_kubectl
    print_info "Starting port forwarding..."
    echo "OpenSearch REST API:    http://localhost:9200"
    echo "OpenSearch Dashboards:  http://localhost:5601"
    echo "Grafana:                http://localhost:4040  (admin/admin)"
    echo "Prometheus:             http://localhost:9090"
    echo ""
    print_info "Press Ctrl+C to stop port forwarding"
    echo ""
    
    # Run all port-forwards in background
    kubectl -n search port-forward svc/opensearch 9200:9200 &
    PF1_PID=$!
    kubectl -n search port-forward svc/opensearch-dashboards 5601:5601 &
    PF2_PID=$!
    kubectl -n search port-forward svc/grafana 4040:4040 &
    PF3_PID=$!
    kubectl -n search port-forward svc/prometheus 9090:9090 &
    PF4_PID=$!
    
    # Wait and cleanup on exit
    trap "kill $PF1_PID $PF2_PID $PF3_PID $PF4_PID 2>/dev/null" EXIT
    wait
}

function check_health() {
    print_info "Checking OpenSearch cluster health..."
    echo ""
    
    # Try localhost first (if port-forward is running)
    if curl -s http://localhost:9200/_cluster/health?pretty 2>/dev/null; then
        echo ""
        return 0
    fi
    
    # Otherwise use kubectl exec
    check_kubectl
    print_info "Port-forward not active, querying via kubectl..."
    kubectl -n search exec -it statefulset/opensearch -- curl -s http://localhost:9200/_cluster/health?pretty
}

function open_dashboard() {
    print_info "Opening OpenSearch Dashboards..."
    
    # Start port-forward in background if not running
    if ! curl -s http://localhost:5601 &>/dev/null; then
        print_info "Starting port-forward..."
        kubectl -n search port-forward svc/opensearch-dashboards 5601:5601 &
        sleep 2
    fi
    
    # Open browser
    if command -v open &> /dev/null; then
        open http://localhost:5601
    elif command -v xdg-open &> /dev/null; then
        xdg-open http://localhost:5601
    else
        print_info "Open http://localhost:5601 in your browser"
    fi
}

function open_grafana() {
    print_info "Opening Grafana dashboards..."
    
    # Start port-forward in background if not running
    if ! curl -s http://localhost:4040/api/health &>/dev/null; then
        print_info "Starting port-forward..."
        kubectl -n search port-forward svc/grafana 4040:4040 &
        sleep 2
    fi
    
    print_info "Grafana credentials: admin / admin"
    echo ""
    
    # Open browser directly to the OpenSearch dashboard
    if command -v open &> /dev/null; then
        open "http://localhost:4040/d/opensearch-overview/opensearch-overview"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "http://localhost:4040/d/opensearch-overview/opensearch-overview"
    else
        print_info "Open http://localhost:4040/d/opensearch-overview/opensearch-overview in your browser"
    fi
}

function open_prometheus() {
    print_info "Opening Prometheus..."
    
    # Start port-forward in background if not running
    if ! curl -s http://localhost:9090/-/ready &>/dev/null; then
        print_info "Starting port-forward..."
        kubectl -n search port-forward svc/prometheus 9090:9090 &
        sleep 2
    fi
    
    # Open browser
    if command -v open &> /dev/null; then
        open http://localhost:9090
    elif command -v xdg-open &> /dev/null; then
        xdg-open http://localhost:9090
    else
        print_info "Open http://localhost:9090 in your browser"
    fi
}

function show_help() {
    echo "OpenSearch Local Development Script (Minikube)"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  start         Start minikube and deploy OpenSearch + monitoring stack"
    echo "  stop          Scale down the search stack (preserves data)"
    echo "  delete        Delete the search namespace (removes all data)"
    echo "  status        Show minikube and pod status"
    echo "  logs          Stream OpenSearch logs"
    echo "  health        Check cluster health"
    echo "  port-forward  Forward all ports to localhost"
    echo "  dashboard     Open OpenSearch Dashboards in browser"
    echo "  grafana       Open Grafana dashboards in browser"
    echo "  prometheus    Open Prometheus in browser"
    echo "  help          Show this help message"
    echo ""
    echo "Resource Configuration (Local):"
    echo "  OpenSearch:  2GB RAM limit, 1GB heap"
    echo "  Dashboards:  512MB RAM limit"
    echo "  Grafana:     128MB RAM limit"
    echo "  Prometheus:  256MB RAM limit"
    echo ""
    echo "Port Forwarding:"
    echo "  OpenSearch:  http://localhost:9200"
    echo "  Dashboards:  http://localhost:5601"
    echo "  Grafana:     http://localhost:4040 (admin/admin)"
    echo "  Prometheus:  http://localhost:9090"
    echo ""
    echo "Environment Variables for search-indexer:"
    echo "  OPENSEARCH_URL     OpenSearch URL (default: http://localhost:9200)"
    echo "  KAFKA_BROKER       Kafka broker (default: localhost:9092)"
    echo "  KAFKA_GROUP_ID     Consumer group (default: search-indexer)"
}

case "${1:-help}" in
    start)
        start_search
        ;;
    stop)
        stop_search
        ;;
    delete)
        delete_search
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs
        ;;
    health)
        check_health
        ;;
    port-forward|pf)
        port_forward
        ;;
    dashboard|dash)
        open_dashboard
        ;;
    grafana)
        open_grafana
        ;;
    prometheus|prom)
        open_prometheus
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac

