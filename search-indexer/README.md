# Search Indexer

Main binary for the Geo Knowledge Graph search indexer.

## Overview

The search indexer consumes entity events from Kafka and indexes them into OpenSearch
for fast full-text search across the Knowledge Graph.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        search-indexer                            │
│  (main binary - configuration, tracing, dependency wiring)      │
└─────────────────────────────────────────────────────────────────┘
                                │
                   ┌────────────┴────────────┐
                   ▼                         ▼
        ┌──────────────────┐      ┌──────────────────┐
        │      Kafka       │      │    OpenSearch    │
        │  (knowledge.edits)│      │  (geo_entities)  │
        └──────────────────┘      └──────────────────┘
```

## Configuration

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `OPENSEARCH_URL` | OpenSearch server URL | `http://localhost:9200` |
| `KAFKA_BROKER` | Kafka broker address | `localhost:9092` |
| `KAFKA_GROUP_ID` | Consumer group ID | `search-indexer` |
| `AXIOM_TOKEN` | Axiom API token (optional) | - |
| `AXIOM_DATASET` | Axiom dataset name | `gaia.search-indexer` |
| `RUST_LOG` | Log level filter | `search_indexer=info` |

## Running

### Prerequisites

1. OpenSearch running at `OPENSEARCH_URL`
2. Kafka broker running at `KAFKA_BROKER`
3. `knowledge.edits` topic exists in Kafka

### Start the indexer

```bash
# With environment variables
OPENSEARCH_URL=http://localhost:9200 \
KAFKA_BROKER=localhost:9092 \
cargo run --release

# Or with .env file
cp .env.example .env
# Edit .env with your configuration
cargo run --release
```

### Docker

```bash
docker build -t search-indexer .
docker run -e OPENSEARCH_URL=http://opensearch:9200 \
           -e KAFKA_BROKER=kafka:9092 \
           search-indexer
```

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running locally

```bash
# Start dependencies
docker-compose -f ../hermes/docker-compose.yml up -d

# Run the indexer
cargo run
```

## Monitoring

The indexer logs to stdout in JSON format. When `AXIOM_TOKEN` is set, logs are
also sent to Axiom for centralized monitoring.

Key metrics to monitor:
- Documents indexed per second
- Index latency (ms)
- Kafka consumer lag
- Error rates

## Troubleshooting

### Common issues

**Cannot connect to OpenSearch**
- Check `OPENSEARCH_URL` is correct
- Verify OpenSearch is running: `curl http://localhost:9200`

**Cannot connect to Kafka**
- Check `KAFKA_BROKER` is correct
- Verify Kafka is running and `knowledge.edits` topic exists

**High latency**
- Check OpenSearch cluster health
- Monitor Kafka consumer lag
- Consider increasing batch size in loader config

