use thiserror::Error;

/// Errors produced by the scanscan core.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("scan failed: {0}")]
    Scan(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unsupported index format: {0}")]
    Format(String),
}

/// Convenience alias used throughout the core.
pub type Result<T> = std::result::Result<T, CoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_convert() {
        let err: CoreError = std::io::Error::new(std::io::ErrorKind::NotFound, "nope").into();
        assert!(matches!(err, CoreError::Io(_)));
    }
}
