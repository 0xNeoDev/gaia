# Search Index Load Testing (Rust)

Load testing suite for benchmarking the search index system, written in Rust for performance and reliability.

## Features

- **Comprehensive Error Handling**: Detailed logging and error messages for connection issues and failures
- **Multiple Test Scenarios**: Pure indexing, pure querying, mixed workload, sustained load, and burst load
- **Resource Awareness**: Automatically adapts to local (16GB MacBook) vs cloud (K8s) deployments
- **Multiple Output Formats**: Human-readable reports, JSON, and CSV (including time-series data)
- **Real-time Progress**: Live progress updates during test execution
- **Realistic Query Generation**: Queries search for text that actually exists in indexed documents, simulating real-world usage patterns

## Installation

```bash
cd load-tests-rust
cargo build --release
```

## Usage

### Pure Indexing Test

```bash
cargo run --release --bin load-test -- indexing \
  --duration 300 \
  --indexing-workers 10 \
  --batch-size 100
```

### Pure Querying Test

```bash
cargo run --release --bin load-test -- querying \
  --duration 300 \
  --query-workers 50
```

### Mixed Workload Test

```bash
cargo run --release --bin load-test -- mixed \
  --duration 600 \
  --indexing-workers 10 \
  --query-workers 50 \
  --batch-size 100
```

### Sustained Load Test

```bash
cargo run --release --bin load-test -- sustained \
  --duration 3600 \
  --indexing-workers 5 \
  --query-workers 25
```

### Burst Load Test

```bash
cargo run --release --bin load-test -- burst \
  --duration 300 \
  --indexing-workers 10 \
  --query-workers 50
```

## Query Generation

The load test generates **realistic queries** that simulate real-world search patterns:

### Query Characteristics

- **Text Matching**: Queries search for text that **actually exists** in the indexed documents. The query generator extracts words from document names and descriptions, ensuring queries will find matching results.

- **Query Patterns**:
  - **50% normal single-word queries**: Simple searches like "entity", "knowledge", "graph"
  - **20% word prefixes**: Autocomplete/search-as-you-type queries like "ent" (for "entity"), "doc" (for "document"), "kno" (for "knowledge")
  - **15% misspelled words**: Typo tolerance testing with queries like "cbloc" (for "block"), "knoledge" (for "knowledge"), "documant" (for "document")
  - **15% multi-word queries**: More complex searches like "important document", "knowledge graph system"

- **Search Fields**: Queries use OpenSearch `multi_match` with:
  - `name` field (boosted 2x for relevance)
  - `description` field
  - Automatic fuzziness for typo tolerance

- **Query Scopes**: Randomly selects from realistic scopes:
  - `GLOBAL`: Search across all documents
  - `GLOBAL_BY_SPACE_SCORE`: Global search with space-based scoring
  - `SPACE_SINGLE`: Search within a specific space
  - `SPACE`: Search within space context

- **Result Limits**: Varies between 10, 20, 50, and 100 results to simulate different use cases

- **Realistic Distribution**: In mixed workload tests, queries are generated from seed documents that match the indexed content, ensuring queries are meaningful and will return results.

This approach ensures that:
1. ✅ Queries will find matching documents (not empty result sets)
2. ✅ Search patterns reflect real user behavior
3. ✅ Performance metrics represent actual search performance
4. ✅ The index is tested under realistic query loads
5. ✅ Autocomplete/search-as-you-type features are tested with prefix queries
6. ✅ Typo tolerance and fuzziness are tested with misspelled words

## Configuration Options

- `--opensearch-url <url>`: OpenSearch URL (default: `http://localhost:9200`)
- `--api-url <url>`: API server URL (optional, for end-to-end testing)
- `--index-name <name>`: Index name (default: `entities`)
- `--duration <seconds>`: Test duration in seconds (default: `300`)
- `--indexing-workers <count>`: Number of indexing workers
- `--query-workers <count>`: Number of query workers
- `--batch-size <size>`: Batch size for indexing (default: `100`)
- `--output-dir <dir>`: Output directory for results (default: `./results`)
- `--deployment-type <type>`: Deployment type (`local`|`cloud`)

## Environment Variables

- `OPENSEARCH_URL`: OpenSearch server URL
- `API_URL`: API server URL
- `DEPLOYMENT_TYPE`: Deployment type (`local`|`cloud`)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`, `warn`)

## Output Files

After running a test, the following files are generated in the output directory:

1. **`{test-name}-report.txt`**: Human-readable summary report
2. **`{test-name}-results.json`**: Detailed JSON results
3. **`{test-name}-results.csv`**: CSV format with key metrics
4. **`{test-name}-timeseries.csv`**: Time-series data for graphing (if available)

## Error Handling

The tool provides comprehensive error handling and logging:

- **Connection Errors**: Clear messages if OpenSearch or API server cannot be reached
- **Health Check Failures**: Warnings and fallback behavior when services are unhealthy
- **Operation Errors**: Detailed error tracking with error type breakdowns
- **Configuration Validation**: Warnings when test parameters exceed recommended limits

All errors are logged with full context and error chains for debugging.

## Examples

### Local Testing (MacBook)

```bash
# Test indexing performance
cargo run --release --bin load-test -- indexing \
  --opensearch-url http://localhost:9200 \
  --duration 300 \
  --indexing-workers 10 \
  --batch-size 100

# Test querying performance
cargo run --release --bin load-test -- querying \
  --opensearch-url http://localhost:9200 \
  --duration 300 \
  --query-workers 50

# Test mixed workload
cargo run --release --bin load-test -- mixed \
  --opensearch-url http://localhost:9200 \
  --api-url http://localhost:3000 \
  --duration 600 \
  --indexing-workers 10 \
  --query-workers 50
```

### Cloud Testing (K8s)

```bash
# Test against cloud deployment
cargo run --release --bin load-test -- mixed \
  --opensearch-url http://opensearch.search.svc.cluster.local:9200 \
  --api-url https://api-testnet.geobrowser.io \
  --duration 1800 \
  --indexing-workers 20 \
  --query-workers 100 \
  --deployment-type cloud
```

## Troubleshooting

### Connection Issues

If you see connection errors:

1. **Check OpenSearch is running**:
   ```bash
   curl http://localhost:9200/_cluster/health
   ```

2. **Check API server is running** (if using):
   ```bash
   curl http://localhost:3000/health
   ```

3. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run --release --bin load-test -- querying ...
   ```

### High Error Rates

If you see high error rates:

1. Reduce the number of workers
2. Increase batch size (for indexing)
3. Check OpenSearch cluster health
4. Monitor resource usage (CPU, memory)

### No Output Files

If output files are not generated:

1. Check the output directory exists and is writable
2. Look for error messages in the console output
3. Check file permissions
4. Ensure the test completed successfully (check exit code)

## Performance Targets

These are initial targets to validate (adjust based on actual hardware):

- **Indexing**: > 1,000 docs/sec (local), > 5,000 docs/sec (cloud)
- **Querying**: > 5,000 QPS (local), > 20,000 QPS (cloud)
- **Latency**: p95 < 100ms (indexing), p95 < 50ms (querying)
- **Error Rate**: < 0.1% under normal load

