//! Request matching logic.

use crate::action::{
    Action, AddPortMappingParams, DeletePortMappingParams, GetGenericPortMappingEntryParams,
    GetSpecificPortMappingEntryParams,
};

/// A parsed SOAP request that can be matched against.
#[derive(Debug, Clone)]
pub struct SoapRequest {
    pub action_name: String,
    pub service_type: String,
    pub body: SoapRequestBody,
}

/// The body of a SOAP request, parsed into a known action type.
#[derive(Debug, Clone)]
pub enum SoapRequestBody {
    GetExternalIPAddress,
    AddPortMapping(AddPortMappingRequest),
    DeletePortMapping(DeletePortMappingRequest),
    GetGenericPortMappingEntry(GetGenericPortMappingEntryRequest),
    GetSpecificPortMappingEntry(GetSpecificPortMappingEntryRequest),
    GetCommonLinkProperties,
    GetTotalBytesReceived,
    GetTotalBytesSent,
    Unknown(String),
}

/// Parsed AddPortMapping request.
#[derive(Debug, Clone)]
pub struct AddPortMappingRequest {
    pub remote_host: String,
    pub external_port: u16,
    pub protocol: String,
    pub internal_port: u16,
    pub internal_client: String,
    pub enabled: bool,
    pub description: String,
    pub lease_duration: u32,
}

/// Parsed DeletePortMapping request.
#[derive(Debug, Clone)]
pub struct DeletePortMappingRequest {
    pub remote_host: String,
    pub external_port: u16,
    pub protocol: String,
}

/// Parsed GetGenericPortMappingEntry request.
#[derive(Debug, Clone)]
pub struct GetGenericPortMappingEntryRequest {
    pub index: u32,
}

/// Parsed GetSpecificPortMappingEntry request.
#[derive(Debug, Clone)]
pub struct GetSpecificPortMappingEntryRequest {
    pub remote_host: String,
    pub external_port: u16,
    pub protocol: String,
}

/// Trait for matching requests.
pub trait Matcher: Send + Sync {
    /// Check if this matcher matches the given request.
    fn matches(&self, request: &SoapRequest) -> bool;
}

impl Matcher for Action {
    fn matches(&self, request: &SoapRequest) -> bool {
        match self {
            Action::Any => true,

            Action::GetExternalIPAddress => {
                matches!(request.body, SoapRequestBody::GetExternalIPAddress)
            }

            Action::AddPortMapping(params) => match &request.body {
                SoapRequestBody::AddPortMapping(req) => matches_add_port_mapping(params, req),
                _ => false,
            },

            Action::DeletePortMapping(params) => match &request.body {
                SoapRequestBody::DeletePortMapping(req) => matches_delete_port_mapping(params, req),
                _ => false,
            },

            Action::GetGenericPortMappingEntry(params) => match &request.body {
                SoapRequestBody::GetGenericPortMappingEntry(req) => {
                    matches_get_generic_port_mapping_entry(params, req)
                }
                _ => false,
            },

            Action::GetSpecificPortMappingEntry(params) => match &request.body {
                SoapRequestBody::GetSpecificPortMappingEntry(req) => {
                    matches_get_specific_port_mapping_entry(params, req)
                }
                _ => false,
            },

            Action::GetCommonLinkProperties => {
                matches!(request.body, SoapRequestBody::GetCommonLinkProperties)
            }

            Action::GetTotalBytesReceived => {
                matches!(request.body, SoapRequestBody::GetTotalBytesReceived)
            }

            Action::GetTotalBytesSent => {
                matches!(request.body, SoapRequestBody::GetTotalBytesSent)
            }
        }
    }
}

fn matches_add_port_mapping(params: &AddPortMappingParams, req: &AddPortMappingRequest) -> bool {
    if let Some(port) = params.external_port {
        if req.external_port != port {
            return false;
        }
    }
    if let Some(protocol) = &params.protocol {
        if req.protocol.to_uppercase() != protocol.as_str() {
            return false;
        }
    }
    if let Some(port) = params.internal_port {
        if req.internal_port != port {
            return false;
        }
    }
    if let Some(client) = &params.internal_client {
        if req.internal_client != client.to_string() {
            return false;
        }
    }
    if let Some(desc) = &params.description {
        if !req.description.contains(desc.as_str()) {
            return false;
        }
    }
    true
}

fn matches_delete_port_mapping(
    params: &DeletePortMappingParams,
    req: &DeletePortMappingRequest,
) -> bool {
    if let Some(port) = params.external_port {
        if req.external_port != port {
            return false;
        }
    }
    if let Some(protocol) = &params.protocol {
        if req.protocol.to_uppercase() != protocol.as_str() {
            return false;
        }
    }
    true
}

fn matches_get_generic_port_mapping_entry(
    params: &GetGenericPortMappingEntryParams,
    req: &GetGenericPortMappingEntryRequest,
) -> bool {
    if let Some(index) = params.index {
        if req.index != index {
            return false;
        }
    }
    true
}

fn matches_get_specific_port_mapping_entry(
    params: &GetSpecificPortMappingEntryParams,
    req: &GetSpecificPortMappingEntryRequest,
) -> bool {
    if let Some(port) = params.external_port {
        if req.external_port != port {
            return false;
        }
    }
    if let Some(protocol) = &params.protocol {
        if req.protocol.to_uppercase() != protocol.as_str() {
            return false;
        }
    }
    true
}
