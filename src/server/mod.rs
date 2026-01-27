//! Mock IGD server implementation.

mod http;
mod ssdp;

use crate::action::Action;
use crate::mock::{Mock, MockRegistry};
use crate::responder::Responder;
use crate::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

/// A mock UPnP IGD server for testing.
pub struct MockIgdServer {
    /// HTTP server address.
    http_addr: SocketAddr,
    /// SSDP server address (if enabled).
    ssdp_addr: Option<SocketAddr>,
    /// Mock registry.
    registry: Arc<MockRegistry>,
    /// Shutdown signal sender.
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MockIgdServer {
    /// Start a new mock IGD server on a random available port.
    pub async fn start() -> Result<Self> {
        Self::builder().start().await
    }

    /// Create a builder for configuring the server.
    pub fn builder() -> MockIgdServerBuilder {
        MockIgdServerBuilder::default()
    }

    /// Get the URL of the HTTP server (for SOAP requests).
    pub fn url(&self) -> String {
        format!("http://{}", self.http_addr)
    }

    /// Get the control URL for SOAP actions.
    pub fn control_url(&self) -> String {
        format!("http://{}/ctl/IPConn", self.http_addr)
    }

    /// Get the device description URL.
    pub fn description_url(&self) -> String {
        format!("http://{}/rootDesc.xml", self.http_addr)
    }

    /// Get the HTTP server address.
    pub fn http_addr(&self) -> SocketAddr {
        self.http_addr
    }

    /// Get the SSDP server address (if enabled).
    pub fn ssdp_addr(&self) -> Option<SocketAddr> {
        self.ssdp_addr
    }

    /// Register a mock for the given action.
    pub async fn mock(&self, action: impl Into<Action>, responder: impl Into<Responder>) {
        let mock = Mock::new(action, responder);
        self.registry.register(mock).await;
    }

    /// Register a mock with a specific priority (higher = checked first).
    pub async fn mock_with_priority(
        &self,
        action: impl Into<Action>,
        responder: impl Into<Responder>,
        priority: u32,
    ) {
        let mock = Mock::new(action, responder).with_priority(priority);
        self.registry.register(mock).await;
    }

    /// Register a mock that only matches a limited number of times.
    pub async fn mock_with_times(
        &self,
        action: impl Into<Action>,
        responder: impl Into<Responder>,
        times: u32,
    ) {
        let mock = Mock::new(action, responder).times(times);
        self.registry.register(mock).await;
    }

    /// Clear all registered mocks.
    pub async fn clear_mocks(&self) {
        self.registry.clear().await;
    }

    /// Shutdown the server.
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for MockIgdServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Builder for configuring a mock IGD server.
#[derive(Default)]
pub struct MockIgdServerBuilder {
    http_port: Option<u16>,
    enable_ssdp: bool,
    ssdp_port: Option<u16>,
}

impl MockIgdServerBuilder {
    /// Set a specific port for the HTTP server.
    pub fn http_port(mut self, port: u16) -> Self {
        self.http_port = Some(port);
        self
    }

    /// Enable SSDP discovery responses.
    pub fn with_ssdp(mut self) -> Self {
        self.enable_ssdp = true;
        self
    }

    /// Set a specific port for SSDP (default: 1900).
    pub fn ssdp_port(mut self, port: u16) -> Self {
        self.ssdp_port = Some(port);
        self.enable_ssdp = true;
        self
    }

    /// Start the server with the configured options.
    pub async fn start(self) -> Result<MockIgdServer> {
        let registry = Arc::new(MockRegistry::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Start HTTP server
        let http_addr = format!("127.0.0.1:{}", self.http_port.unwrap_or(0));
        let listener = tokio::net::TcpListener::bind(&http_addr).await?;
        let http_addr = listener.local_addr()?;

        let http_registry = registry.clone();
        tokio::spawn(async move {
            http::run_http_server(listener, http_registry, shutdown_rx).await;
        });

        // Start SSDP server if enabled
        let ssdp_addr = if self.enable_ssdp {
            let port = self.ssdp_port.unwrap_or(1900);
            match ssdp::start_ssdp_server(http_addr, port).await {
                Ok(addr) => Some(addr),
                Err(e) => {
                    tracing::warn!("Failed to start SSDP server: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(MockIgdServer {
            http_addr,
            ssdp_addr,
            registry,
            shutdown_tx: Some(shutdown_tx),
        })
    }
}
