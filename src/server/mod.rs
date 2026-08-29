//! Mock IGD server implementation.

mod http;
mod ssdp;

use crate::Result;
use crate::action::Action;
use crate::mock::{Mock, MockRegistry, ReceivedRequest, ReceivedSsdpRequest};
use crate::responder::Responder;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

/// UPnP IGD specification version the mock server emulates.
///
/// The version determines the device/service types advertised in SSDP
/// responses and the device description, as well as the actions listed in
/// the WANIPConnection SCPD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IgdVersion {
    /// InternetGatewayDevice:1 with WANIPConnection:1 (default).
    #[default]
    V1,
    /// InternetGatewayDevice:2 with WANIPConnection:2.
    ///
    /// Adds the v2-only actions `AddAnyPortMapping`,
    /// `DeletePortMappingRange` and `GetListOfPortMappings`.
    V2,
}

impl IgdVersion {
    /// The numeric device/service version suffix (1 or 2).
    pub fn number(&self) -> u8 {
        match self {
            IgdVersion::V1 => 1,
            IgdVersion::V2 => 2,
        }
    }
}

/// The WAN connection service(s) the mock server exposes.
///
/// A real gateway offers `WANIPConnection` for a routed WAN interface and
/// `WANPPPConnection` for a PPP based one (PPPoE/PPPoA); some devices
/// advertise both. `WANPPPConnection` is only defined in version 1, so it
/// is advertised as `urn:schemas-upnp-org:service:WANPPPConnection:1` for
/// both IGD v1 and IGD v2 devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ConnectionService {
    /// Only `WANIPConnection` (default).
    #[default]
    Ip,
    /// Only `WANPPPConnection:1`.
    Ppp,
    /// Both `WANIPConnection` and `WANPPPConnection:1`.
    Both,
}

impl ConnectionService {
    /// Whether `WANIPConnection` is advertised.
    pub fn has_ip(&self) -> bool {
        matches!(self, ConnectionService::Ip | ConnectionService::Both)
    }

    /// Whether `WANPPPConnection` is advertised.
    pub fn has_ppp(&self) -> bool {
        matches!(self, ConnectionService::Ppp | ConnectionService::Both)
    }
}

/// The device configuration shared by the HTTP and SSDP servers.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DeviceConfig {
    /// IGD version the server emulates.
    pub(crate) igd_version: IgdVersion,
    /// WAN connection service(s) the server exposes.
    pub(crate) connection_service: ConnectionService,
}

/// Control URL path of the `WANIPConnection` service.
pub(crate) const IP_CONNECTION_CONTROL_PATH: &str = "/ctl/IPConn";

/// Control URL path of the `WANPPPConnection` service.
pub(crate) const PPP_CONNECTION_CONTROL_PATH: &str = "/ctl/PPPConn";

/// SCPD URL path of the `WANIPConnection` service.
pub(crate) const IP_CONNECTION_SCPD_PATH: &str = "/WANIPCn.xml";

/// SCPD URL path of the `WANPPPConnection` service.
pub(crate) const PPP_CONNECTION_SCPD_PATH: &str = "/WANPPPCn.xml";

/// A mock UPnP IGD server for testing.
pub struct MockIgdServer {
    /// HTTP server address.
    http_addr: SocketAddr,
    /// SSDP server address (if enabled).
    ssdp_addr: Option<SocketAddr>,
    /// Device configuration (IGD version and connection service).
    config: DeviceConfig,
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
    ///
    /// Returns the `WANPPPConnection` control URL when the server only
    /// exposes that service, and the `WANIPConnection` control URL
    /// otherwise. Use [`ip_control_url`](Self::ip_control_url) or
    /// [`ppp_control_url`](Self::ppp_control_url) to address a specific
    /// service on a server exposing both.
    pub fn control_url(&self) -> String {
        match self.config.connection_service {
            ConnectionService::Ppp => self.ppp_control_url(),
            _ => self.ip_control_url(),
        }
    }

    /// Get the `WANIPConnection` control URL for SOAP actions.
    pub fn ip_control_url(&self) -> String {
        format!("http://{}{}", self.http_addr, IP_CONNECTION_CONTROL_PATH)
    }

    /// Get the `WANPPPConnection` control URL for SOAP actions.
    pub fn ppp_control_url(&self) -> String {
        format!("http://{}{}", self.http_addr, PPP_CONNECTION_CONTROL_PATH)
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

    /// Get the IGD version the server emulates.
    pub fn igd_version(&self) -> IgdVersion {
        self.config.igd_version
    }

    /// Get the WAN connection service(s) the server exposes.
    pub fn connection_service(&self) -> ConnectionService {
        self.config.connection_service
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

    /// Get all received requests.
    ///
    /// Returns a list of all SOAP requests received by the server.
    /// Useful for verifying that expected requests were made.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let requests = server.received_requests().await;
    /// assert_eq!(requests.len(), 1);
    /// assert_eq!(requests[0].action_name, "GetExternalIPAddress");
    /// ```
    pub async fn received_requests(&self) -> Vec<ReceivedRequest> {
        self.registry.received_requests().await
    }

    /// Clear all received requests.
    pub async fn clear_received_requests(&self) {
        self.registry.clear_received_requests().await;
    }

    /// Get all received SSDP requests (M-SEARCH).
    ///
    /// Returns a list of all SSDP M-SEARCH requests received by the server.
    /// Useful for verifying device discovery behavior.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let requests = server.received_ssdp_requests().await;
    /// assert_eq!(requests.len(), 1);
    /// assert_eq!(requests[0].search_target, "ssdp:all");
    /// ```
    pub async fn received_ssdp_requests(&self) -> Vec<ReceivedSsdpRequest> {
        self.registry.received_ssdp_requests().await
    }

    /// Clear all received SSDP requests.
    pub async fn clear_received_ssdp_requests(&self) {
        self.registry.clear_received_ssdp_requests().await;
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
    igd_version: IgdVersion,
    connection_service: ConnectionService,
}

impl MockIgdServerBuilder {
    /// Set a specific port for the HTTP server.
    pub fn http_port(mut self, port: u16) -> Self {
        self.http_port = Some(port);
        self
    }

    /// Set the IGD version to emulate (default: [`IgdVersion::V1`]).
    pub fn igd_version(mut self, version: IgdVersion) -> Self {
        self.igd_version = version;
        self
    }

    /// Set the WAN connection service(s) to expose
    /// (default: [`ConnectionService::Ip`]).
    ///
    /// Use [`ConnectionService::Ppp`] to emulate a PPP based gateway that
    /// advertises `urn:schemas-upnp-org:service:WANPPPConnection:1`, or
    /// [`ConnectionService::Both`] to advertise both connection services.
    pub fn connection_service(mut self, service: ConnectionService) -> Self {
        self.connection_service = service;
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

        let config = DeviceConfig {
            igd_version: self.igd_version,
            connection_service: self.connection_service,
        };
        let http_registry = registry.clone();
        tokio::spawn(async move {
            http::run_http_server(listener, http_registry, config, shutdown_rx).await;
        });

        // Start SSDP server if enabled
        let ssdp_addr = if self.enable_ssdp {
            let port = self.ssdp_port.unwrap_or(1900);
            match ssdp::start_ssdp_server(http_addr, port, config, registry.clone()).await {
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
            config,
            registry,
            shutdown_tx: Some(shutdown_tx),
        })
    }
}
