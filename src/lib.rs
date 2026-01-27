//! # mock-igd
//!
//! A mock UPnP Internet Gateway Device (IGD) server for testing client implementations.
//!
//! ## Example
//!
//! ```no_run
//! use mock_igd::{MockIgdServer, Action, Responder};
//!
//! #[tokio::test]
//! async fn test_get_external_ip() {
//!     let server = MockIgdServer::start().await.unwrap();
//!
//!     server.mock(
//!         Action::GetExternalIPAddress,
//!         Responder::success()
//!             .with_external_ip("203.0.113.1".parse().unwrap())
//!     ).await;
//!
//!     // Use server.url() to connect your IGD client
//!     let url = server.url();
//! }
//! ```

pub mod action;
pub mod error;
pub mod matcher;
pub mod mock;
pub mod responder;
pub mod server;

// Re-exports for convenience
pub use action::{Action, Protocol};
pub use error::{Error, Result};
pub use matcher::Matcher;
pub use responder::Responder;
pub use server::MockIgdServer;
