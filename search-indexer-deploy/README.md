# Search Indexer Deployment

Kubernetes deployment configurations for the search indexer, OpenSearch, and monitoring stack.

## Local Development

Use docker-compose for local development:

```bash
cd search-indexer-deploy
docker-compose up
```

Services:
- **OpenSearch REST API**: `http://localhost:9200`
- **OpenSearch Dashboards**: `http://localhost:5601`
- **Grafana**: `http://localhost:4040` (admin/admin)
- **Prometheus**: `http://localhost:9090`
- **OpenSearch Exporter**: `http://localhost:9114`

Run the search indexer locally:
```bash
OPENSEARCH_URL=http://localhost:9200 cargo run -p search-indexer
```

Check cluster health:
```bash
curl http://localhost:9200/_cluster/health?pretty
```

## Production

Production runs on Kubernetes and is deployed via GitHub Actions.

The Kubernetes manifests are in `k8s/`.

### Manual Deployment

```bash
# Apply base configuration
kubectl apply -k search-indexer-deploy/k8s/

# Or using kustomize directly
kustomize build search-indexer-deploy/k8s/ | kubectl apply -f -
```

## Directory Structure

```
search-indexer-deploy/
├── docker-compose.yaml  # Local development
├── prometheus.yml       # Prometheus config for docker-compose
├── grafana/             # Grafana provisioning configs
│   ├── datasources.yml
│   ├── dashboard-providers.yml
│   └── dashboards/     # Dashboard JSON files
└── k8s/                 # Kubernetes manifests (production)
    ├── kustomization.yaml
    ├── namespace.yaml
    ├── opensearch.yaml      # OpenSearch StatefulSet + Service
    ├── search-indexer.yaml  # Search Indexer Deployment
    └── monitoring.yaml      # Prometheus + Grafana + OpenSearch Exporter
```

## Resource Configuration

| Environment | OpenSearch RAM | OpenSearch Heap | Dashboards RAM | Grafana RAM | Prometheus RAM |
|-------------|----------------|-----------------|----------------|-------------|----------------|
| Production  | 6 GB           | 3 GB            | 1 GB           | 2 GB        | 512 MB         |
| Local       | 2 GB           | 1 GB            | 512 MB         | 1 GB        | 256 MB         |

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
