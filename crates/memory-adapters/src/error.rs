//! Error types for adapter operations.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during adapter operations.
#[derive(Error, Debug)]
pub enum AdapterError {
    /// Configuration file not found or invalid.
    #[error("Configuration error at {path:?}: {message}")]
    Config {
        path: Option<PathBuf>,
        message: String,
    },

    /// Failed to normalize event from raw format.
    #[error("Normalization error: {0}")]
    Normalize(String),

    /// IO error during adapter operation.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse event data.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Agent detection failed.
    #[error("Detection error: {0}")]
    Detection(String),
}

impl AdapterError {
    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            path: None,
            message: message.into(),
        }
    }

    /// Create a configuration error with path context.
    pub fn config_at(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Config {
            path: Some(path.into()),
            message: message.into(),
        }
    }

    /// Create a normalization error.
    pub fn normalize(message: impl Into<String>) -> Self {
        Self::Normalize(message.into())
    }

    /// Create a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(message.into())
    }

    /// Create a detection error.
    pub fn detection(message: impl Into<String>) -> Self {
        Self::Detection(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AdapterError::config("invalid format");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("invalid format"));
    }

    #[test]
    fn test_error_with_path() {
        let err = AdapterError::config_at("/path/to/config.toml", "missing field");
        assert!(err.to_string().contains("/path/to/config.toml"));
    }

    #[test]
    fn test_normalize_error() {
        let err = AdapterError::normalize("missing timestamp");
        assert!(err.to_string().contains("Normalization error"));
        assert!(err.to_string().contains("missing timestamp"));
    }

    #[test]
    fn test_parse_error() {
        let err = AdapterError::parse("invalid JSON");
        assert!(err.to_string().contains("Parse error"));
    }

    #[test]
    fn test_detection_error() {
        let err = AdapterError::detection("no agent found");
        assert!(err.to_string().contains("Detection error"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: AdapterError = io_err.into();
        assert!(err.to_string().contains("IO error"));
    }
}
