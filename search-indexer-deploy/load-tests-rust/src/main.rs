use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use tracing::{error, info, warn};
use tracing_subscriber;

mod config;
mod clients;
mod generators;
mod loaders;
mod metrics;
mod reporter;
mod scenarios;

use config::TestConfig;
use scenarios::{run_indexing, run_querying, run_mixed, run_sustained, run_burst};

#[derive(Parser)]
#[command(name = "load-test")]
#[command(about = "Load testing scripts for the search index system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// OpenSearch URL
    #[arg(long, default_value = "http://localhost:9200", global = true)]
    opensearch_url: String,

    /// API server URL (optional, for end-to-end testing)
    #[arg(long, global = true)]
    api_url: Option<String>,

    /// Index name
    #[arg(long, default_value = "entities", global = true)]
    index_name: String,

    /// Test duration in seconds
    #[arg(long, default_value = "300", global = true)]
    duration: u64,

    /// Number of indexing workers
    #[arg(long, global = true)]
    indexing_workers: Option<usize>,

    /// Number of query workers
    #[arg(long, global = true)]
    query_workers: Option<usize>,

    /// Batch size for indexing
    #[arg(long, default_value = "100", global = true)]
    batch_size: usize,

    /// Output directory for results
    #[arg(long, default_value = "./results", global = true)]
    output_dir: String,

    /// Deployment type (local|cloud)
    #[arg(long, global = true)]
    deployment_type: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run pure indexing load test
    Indexing,
    /// Run pure querying load test
    Querying,
    /// Run mixed workload load test (simultaneous indexing and querying)
    Mixed,
    /// Run sustained load test (extended duration)
    Sustained,
    /// Run burst load test (sudden spike in traffic)
    Burst,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Set deployment type in environment if provided
    if let Some(ref deployment_type) = cli.deployment_type {
        env::set_var("DEPLOYMENT_TYPE", deployment_type);
    }
    
    // Set OpenSearch URL from environment if not provided via CLI
    if env::var("OPENSEARCH_URL").is_ok() && cli.opensearch_url == "http://localhost:9200" {
        // Use env var if default was used
    } else {
        env::set_var("OPENSEARCH_URL", &cli.opensearch_url);
    }
    
    // Set API URL from environment if not provided via CLI
    if let Some(ref api_url) = cli.api_url {
        env::set_var("API_URL", api_url);
    } else if env::var("API_URL").is_ok() {
        // Keep env var if set
    }

    let result = match cli.command {
        Commands::Indexing => {
            info!("Starting indexing load test");
            run_indexing_test(&cli).await
        }
        Commands::Querying => {
            info!("Starting querying load test");
            run_querying_test(&cli).await
        }
        Commands::Mixed => {
            info!("Starting mixed workload load test");
            run_mixed_test(&cli).await
        }
        Commands::Sustained => {
            info!("Starting sustained load test");
            run_sustained_test(&cli).await
        }
        Commands::Burst => {
            info!("Starting burst load test");
            run_burst_test(&cli).await
        }
    };

    if let Err(e) = result {
        error!("Test failed: {}", e);
        eprintln!("\n❌ Error: {}", e);
        
        // Print error chain for debugging
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("  Caused by: {}", err);
            source = err.source();
        }
        
        std::process::exit(1);
    }
}

async fn run_indexing_test(cli: &Cli) -> Result<()> {
    let limits = config::get_test_limits(None);
    let indexing_workers = cli.indexing_workers.unwrap_or(limits.max_indexing_workers);
    let batch_size = cli.batch_size;

    // Validate configuration
    let validation = config::validate_test_config(indexing_workers, 0, batch_size);
    if !validation.valid {
        warn!("Configuration warnings:");
        for warning in &validation.warnings {
            warn!("  - {}", warning);
        }
    }

    let config = TestConfig {
        scenario: "indexing".to_string(),
        duration_seconds: cli.duration,
        indexing_workers: Some(indexing_workers),
        query_workers: None,
        batch_size: Some(batch_size),
        opensearch_url: cli.opensearch_url.clone(),
        api_url: cli.api_url.clone(),
        index_name: cli.index_name.clone(),
        output_dir: cli.output_dir.clone(),
    };

    run_indexing(config).await
}

async fn run_querying_test(cli: &Cli) -> Result<()> {
    let limits = config::get_test_limits(None);
    let query_workers = cli.query_workers.unwrap_or(limits.max_query_workers);

    // Validate configuration
    let validation = config::validate_test_config(0, query_workers, 0);
    if !validation.valid {
        warn!("Configuration warnings:");
        for warning in &validation.warnings {
            warn!("  - {}", warning);
        }
    }

    let config = TestConfig {
        scenario: "querying".to_string(),
        duration_seconds: cli.duration,
        indexing_workers: None,
        query_workers: Some(query_workers),
        batch_size: None,
        opensearch_url: cli.opensearch_url.clone(),
        api_url: cli.api_url.clone(),
        index_name: cli.index_name.clone(),
        output_dir: cli.output_dir.clone(),
    };

    run_querying(config).await
}

async fn run_mixed_test(cli: &Cli) -> Result<()> {
    let limits = config::get_test_limits(None);
    let indexing_workers = cli.indexing_workers.unwrap_or(limits.max_indexing_workers / 2);
    let query_workers = cli.query_workers.unwrap_or(limits.max_query_workers / 2);
    let batch_size = cli.batch_size;

    // Validate configuration
    let validation = config::validate_test_config(indexing_workers, query_workers, batch_size);
    if !validation.valid {
        warn!("Configuration warnings:");
        for warning in &validation.warnings {
            warn!("  - {}", warning);
        }
    }

    let config = TestConfig {
        scenario: "mixed".to_string(),
        duration_seconds: cli.duration,
        indexing_workers: Some(indexing_workers),
        query_workers: Some(query_workers),
        batch_size: Some(batch_size),
        opensearch_url: cli.opensearch_url.clone(),
        api_url: cli.api_url.clone(),
        index_name: cli.index_name.clone(),
        output_dir: cli.output_dir.clone(),
    };

    run_mixed(config).await
}

async fn run_sustained_test(cli: &Cli) -> Result<()> {
    let limits = config::get_test_limits(None);
    let indexing_workers = cli.indexing_workers.unwrap_or(limits.max_indexing_workers / 2);
    let query_workers = cli.query_workers.unwrap_or(limits.max_query_workers / 2);
    let batch_size = cli.batch_size;
    let duration = cli.duration.max(3600); // At least 1 hour

    // Validate configuration
    let validation = config::validate_test_config(indexing_workers, query_workers, batch_size);
    if !validation.valid {
        warn!("Configuration warnings:");
        for warning in &validation.warnings {
            warn!("  - {}", warning);
        }
    }

    let config = TestConfig {
        scenario: "sustained".to_string(),
        duration_seconds: duration,
        indexing_workers: Some(indexing_workers),
        query_workers: Some(query_workers),
        batch_size: Some(batch_size),
        opensearch_url: cli.opensearch_url.clone(),
        api_url: cli.api_url.clone(),
        index_name: cli.index_name.clone(),
        output_dir: cli.output_dir.clone(),
    };

    run_sustained(config).await
}

async fn run_burst_test(cli: &Cli) -> Result<()> {
    let limits = config::get_test_limits(None);
    let base_indexing_workers = cli.indexing_workers.unwrap_or(limits.max_indexing_workers);
    let base_query_workers = cli.query_workers.unwrap_or(limits.max_query_workers);
    let indexing_workers = base_indexing_workers * 3; // 3x for burst
    let query_workers = base_query_workers * 5; // 5x for burst
    let batch_size = cli.batch_size;
    let duration = cli.duration.min(300); // Max 5 minutes

    // Validate configuration
    let validation = config::validate_test_config(indexing_workers, query_workers, batch_size);
    if !validation.valid {
        warn!("Configuration warnings:");
        for warning in &validation.warnings {
            warn!("  - {}", warning);
        }
    }

    let config = TestConfig {
        scenario: "burst".to_string(),
        duration_seconds: duration,
        indexing_workers: Some(indexing_workers),
        query_workers: Some(query_workers),
        batch_size: Some(batch_size),
        opensearch_url: cli.opensearch_url.clone(),
        api_url: cli.api_url.clone(),
        index_name: cli.index_name.clone(),
        output_dir: cli.output_dir.clone(),
    };

    run_burst(config).await
}

