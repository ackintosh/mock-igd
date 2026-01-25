//! Builder for success responses.

use super::{Responder, ResponderInner, SuccessResponse};
use std::net::IpAddr;
use std::sync::Arc;

/// Builder for creating successful responses.
#[derive(Debug, Clone, Default)]
pub struct SuccessResponseBuilder {
    response: SuccessResponse,
}

impl SuccessResponseBuilder {
    /// Set the external IP address (for GetExternalIPAddress).
    pub fn with_external_ip(mut self, ip: IpAddr) -> Self {
        self.response.external_ip = Some(ip);
        self
    }

    /// Set the remote host (for port mapping responses).
    pub fn with_remote_host(mut self, host: impl Into<String>) -> Self {
        self.response.remote_host = Some(host.into());
        self
    }

    /// Set the external port (for port mapping responses).
    pub fn with_external_port(mut self, port: u16) -> Self {
        self.response.external_port = Some(port);
        self
    }

    /// Set the protocol (for port mapping responses).
    pub fn with_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.response.protocol = Some(protocol.into());
        self
    }

    /// Set the internal port (for port mapping responses).
    pub fn with_internal_port(mut self, port: u16) -> Self {
        self.response.internal_port = Some(port);
        self
    }

    /// Set the internal client (for port mapping responses).
    pub fn with_internal_client(mut self, client: impl Into<String>) -> Self {
        self.response.internal_client = Some(client.into());
        self
    }

    /// Set whether the mapping is enabled (for port mapping responses).
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.response.enabled = Some(enabled);
        self
    }

    /// Set the description (for port mapping responses).
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.response.description = Some(description.into());
        self
    }

    /// Set the lease duration (for port mapping responses).
    pub fn with_lease_duration(mut self, duration: u32) -> Self {
        self.response.lease_duration = Some(duration);
        self
    }

    /// Set the WAN access type (for GetCommonLinkProperties).
    pub fn with_wan_access_type(mut self, access_type: impl Into<String>) -> Self {
        self.response.wan_access_type = Some(access_type.into());
        self
    }

    /// Set upstream max bit rate (for GetCommonLinkProperties).
    pub fn with_layer1_upstream_max_bit_rate(mut self, rate: u32) -> Self {
        self.response.layer1_upstream_max_bit_rate = Some(rate);
        self
    }

    /// Set downstream max bit rate (for GetCommonLinkProperties).
    pub fn with_layer1_downstream_max_bit_rate(mut self, rate: u32) -> Self {
        self.response.layer1_downstream_max_bit_rate = Some(rate);
        self
    }

    /// Set physical link status (for GetCommonLinkProperties).
    pub fn with_physical_link_status(mut self, status: impl Into<String>) -> Self {
        self.response.physical_link_status = Some(status.into());
        self
    }

    /// Set total bytes (for GetTotalBytesReceived/Sent).
    pub fn with_total_bytes(mut self, bytes: u64) -> Self {
        self.response.total_bytes = Some(bytes);
        self
    }

    /// Build the responder.
    pub fn build(self) -> Responder {
        Responder {
            inner: Arc::new(ResponderInner::Success(self.response)),
        }
    }
}

impl From<SuccessResponseBuilder> for Responder {
    fn from(builder: SuccessResponseBuilder) -> Self {
        builder.build()
    }
}
