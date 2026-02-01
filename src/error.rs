//! Error types for mock-igd.

use thiserror::Error;

/// Result type alias for mock-igd operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in mock-igd.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to bind to address.
    #[error("failed to bind to address: {0}")]
    Bind(#[from] std::io::Error),

    /// Invalid SOAP action.
    #[error("invalid SOAP action: {0}")]
    InvalidAction(String),

    /// Server is not running.
    #[error("server is not running")]
    ServerNotRunning,
}
