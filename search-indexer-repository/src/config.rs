//! Configuration types for the SearchIndexClient.

/// Configuration for the SearchIndexClient.
#[derive(Debug, Clone)]
pub struct SearchIndexConfig {
    /// Maximum number of documents allowed in a single batch operation.
    /// Set to None to disable the limit (not recommended for production).
    pub max_batch_size: Option<usize>,
}

impl Default for SearchIndexConfig {
    fn default() -> Self {
        Self {
            max_batch_size: Some(1000),
        }
    }
}

impl SearchIndexConfig {
    /// Create a config with no batch size limit (use with caution).
    pub fn unlimited() -> Self {
        Self {
            max_batch_size: None,
        }
    }

    /// Create a config with a custom batch size limit.
    pub fn with_max_batch_size(max_batch_size: usize) -> Self {
        Self {
            max_batch_size: Some(max_batch_size),
        }
    }
}

