//! Response generation for UPnP IGD actions.

mod builder;
mod templates;

pub use builder::SuccessResponseBuilder;
pub(crate) use templates::generate_soap_fault;
use templates::generate_success_response;

use crate::matcher::SoapRequest;
use std::net::IpAddr;
use std::sync::Arc;

/// A responder that generates responses for matched requests.
#[derive(Clone)]
pub struct Responder {
    inner: Arc<ResponderInner>,
}

enum ResponderInner {
    Success(SuccessResponse),
    Error { code: u16, description: String },
    Custom(Arc<dyn Fn(&SoapRequest) -> ResponseBody + Send + Sync>),
}

/// The body of a response.
#[derive(Debug, Clone)]
pub enum ResponseBody {
    /// A successful SOAP response.
    Soap(String),
    /// An error SOAP response.
    SoapFault { code: u16, description: String },
    /// A raw HTTP response body.
    Raw { content_type: String, body: String },
}

/// Data for successful responses.
#[derive(Debug, Clone, Default)]
pub(crate) struct SuccessResponse {
    // GetExternalIPAddress
    pub(crate) external_ip: Option<IpAddr>,

    // GetGenericPortMappingEntry / GetSpecificPortMappingEntry
    pub(crate) remote_host: Option<String>,
    pub(crate) external_port: Option<u16>,
    pub(crate) protocol: Option<String>,
    pub(crate) internal_port: Option<u16>,
    pub(crate) internal_client: Option<String>,
    pub(crate) enabled: Option<bool>,
    pub(crate) description: Option<String>,
    pub(crate) lease_duration: Option<u32>,

    // GetCommonLinkProperties
    pub(crate) wan_access_type: Option<String>,
    pub(crate) layer1_upstream_max_bit_rate: Option<u32>,
    pub(crate) layer1_downstream_max_bit_rate: Option<u32>,
    pub(crate) physical_link_status: Option<String>,

    // GetTotalBytesReceived / GetTotalBytesSent
    pub(crate) total_bytes: Option<u64>,
}

impl Responder {
    /// Create a successful response.
    pub fn success() -> SuccessResponseBuilder {
        SuccessResponseBuilder::default()
    }

    /// Create an error response with UPnP error code.
    pub fn error(code: u16, description: impl Into<String>) -> Self {
        Responder {
            inner: Arc::new(ResponderInner::Error {
                code,
                description: description.into(),
            }),
        }
    }

    /// Create a custom responder with a closure.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(&SoapRequest) -> ResponseBody + Send + Sync + 'static,
    {
        Responder {
            inner: Arc::new(ResponderInner::Custom(Arc::new(f))),
        }
    }

    /// Generate a response for the given request.
    pub fn respond(&self, request: &SoapRequest) -> ResponseBody {
        match self.inner.as_ref() {
            ResponderInner::Success(data) => {
                let xml = generate_success_response(&request.action_name, data);
                ResponseBody::Soap(xml)
            }
            ResponderInner::Error { code, description } => ResponseBody::SoapFault {
                code: *code,
                description: description.clone(),
            },
            ResponderInner::Custom(f) => f(request),
        }
    }
}

impl std::fmt::Debug for Responder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_ref() {
            ResponderInner::Success(data) => f.debug_tuple("Responder::Success").field(data).finish(),
            ResponderInner::Error { code, description } => f
                .debug_struct("Responder::Error")
                .field("code", code)
                .field("description", description)
                .finish(),
            ResponderInner::Custom(_) => f.debug_tuple("Responder::Custom").finish(),
        }
    }
}
