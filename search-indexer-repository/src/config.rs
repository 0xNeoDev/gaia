//! Configuration types for the SearchIndexClient.

/// Configuration for the SearchIndexClient.
///
/// This struct allows customization of client behavior, particularly around batch
/// operation limits. Use this to control resource usage and prevent accidentally
/// sending overly large batches to the search index backend.
#[derive(Debug, Clone)]
pub struct SearchIndexConfig {
    /// Maximum number of documents allowed in a single batch operation.
    ///
    /// Set to `None` to disable the limit (not recommended for production).
    /// Defaults to 1000 if not specified.
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
    /// Create a config with no batch size limit.
    ///
    /// # Warning
    ///
    /// Use with caution. Removing batch size limits can lead to memory issues
    /// and timeouts when processing very large batches. Not recommended for production.
    ///
    /// # Returns
    ///
    /// A `SearchIndexConfig` with `max_batch_size` set to `None`.
    pub fn unlimited() -> Self {
        Self {
            max_batch_size: None,
        }
    }

    /// Create a config with a custom batch size limit.
    ///
    /// # Arguments
    ///
    /// * `max_batch_size` - Maximum number of documents allowed in a single batch operation
    ///
    /// # Returns
    ///
    /// A `SearchIndexConfig` with the specified batch size limit.
    pub fn with_max_batch_size(max_batch_size: usize) -> Self {
        Self {
            max_batch_size: Some(max_batch_size),
        }
    }
}
