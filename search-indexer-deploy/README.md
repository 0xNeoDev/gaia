# Search Indexer Deployment

Kubernetes deployment configurations for the search indexer, OpenSearch, and monitoring stack.

## Directory Structure

```
search-indexer-deploy/
├── base/                    # Base Kubernetes manifests
│   ├── kustomization.yaml
│   ├── namespace.yaml
│   ├── opensearch.yaml      # OpenSearch StatefulSet + Service
│   ├── search-indexer.yaml  # Search Indexer Deployment
│   └── monitoring.yaml      # Prometheus + Grafana + OpenSearch Exporter
├── overlays/
│   └── local/               # Local development overrides (minikube)
│       ├── kustomization.yaml
│       ├── opensearch-patch.yaml
│       └── monitoring-patch.yaml
└── scripts/
    └── local.sh             # Local development helper script
```

## Resource Configuration

| Environment | OpenSearch RAM | OpenSearch Heap | Dashboards RAM | Grafana RAM | Prometheus RAM |
|-------------|----------------|-----------------|----------------|-------------|----------------|
| Production  | 6 GB           | 3 GB            | 1 GB           | 256 MB      | 512 MB         |
| Local       | 2 GB           | 1 GB            | 512 MB         | 128 MB      | 256 MB         |

## Quick Start

### Local Development (Minikube)

```bash
# Start minikube and deploy OpenSearch + monitoring
./search-indexer-deploy/scripts/local.sh start

# Forward all ports to localhost
./search-indexer-deploy/scripts/local.sh port-forward

# Run the search indexer locally
OPENSEARCH_URL=http://localhost:9200 cargo run -p search-indexer

# Check cluster health
./search-indexer-deploy/scripts/local.sh health

# View logs
./search-indexer-deploy/scripts/local.sh logs

# Open Grafana dashboards (admin/admin)
./search-indexer-deploy/scripts/local.sh grafana

# Open Prometheus
./search-indexer-deploy/scripts/local.sh prometheus

# Stop (preserves data)
./search-indexer-deploy/scripts/local.sh stop
```

### Production Deployment

```bash
# Apply base configuration
kubectl apply -k search-indexer-deploy/base/

# Or using kustomize directly
kustomize build search-indexer-deploy/base/ | kubectl apply -f -
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| OpenSearch REST API | 9200 | Search and indexing API |
| OpenSearch Transport | 9300 | Inter-node communication |
| OpenSearch Dashboards | 5601 | Web UI for OpenSearch queries |
| Grafana | 4040 | Metrics dashboards (admin/admin) |
| Prometheus | 9090 | Metrics collection and querying |
| OpenSearch Exporter | 9114 | Prometheus metrics exporter |

## Monitoring Stack

The monitoring stack includes:

- **OpenSearch Exporter**: Exports OpenSearch metrics in Prometheus format using the [prometheus-community/elasticsearch_exporter](https://github.com/prometheus-community/elasticsearch_exporter)
- **Prometheus**: Scrapes metrics from the exporter and stores time-series data
- **Grafana**: Pre-configured with an OpenSearch Overview dashboard showing:
  - Search QPS and latency
  - Document counts and indexing rates
  - Cluster health and shard status
  - JVM heap and GC metrics
  - CPU, memory, and filesystem utilization
  - Thread pool queues and rejections
  - Circuit breaker status

## Environment Variables

The search-indexer binary uses these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENSEARCH_URL` | OpenSearch REST endpoint | `http://localhost:9200` |
| `KAFKA_BROKER` | Kafka broker address | `localhost:9092` |
| `KAFKA_GROUP_ID` | Consumer group ID | `search-indexer` |
| `RUST_LOG` | Log level | `search_indexer=info` |
| `AXIOM_TOKEN` | Axiom API token (optional) | - |
| `AXIOM_DATASET` | Axiom dataset name | `gaia.search-indexer` |

## Security Notes

⚠️ **The default configuration disables OpenSearch security for development.**

For production, you should:
1. Enable the OpenSearch security plugin
2. Configure TLS certificates
3. Set up authentication
4. Use Kubernetes secrets for credentials

