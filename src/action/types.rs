//! UPnP IGD action type definitions.

use std::net::IpAddr;

/// Protocol type for port mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::TCP => "TCP",
            Protocol::UDP => "UDP",
        }
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// UPnP IGD actions that can be matched against.
#[derive(Debug, Clone)]
pub enum Action {
    // WANIPConnection actions
    /// Get the external IP address of the gateway.
    GetExternalIPAddress,

    /// Get the connection status information.
    GetStatusInfo,

    /// Add a port mapping.
    AddPortMapping(AddPortMappingParams),

    /// Delete a port mapping.
    DeletePortMapping(DeletePortMappingParams),

    /// Get a port mapping entry by index.
    GetGenericPortMappingEntry(GetGenericPortMappingEntryParams),

    /// Get a specific port mapping entry.
    GetSpecificPortMappingEntry(GetSpecificPortMappingEntryParams),

    // WANIPConnection:2 actions (IGD v2)
    /// Add a port mapping, letting the gateway pick an alternative external
    /// port on conflict (IGD v2 only).
    AddAnyPortMapping(AddPortMappingParams),

    /// Delete a range of port mappings (IGD v2 only).
    DeletePortMappingRange(DeletePortMappingRangeParams),

    /// Get a list of port mappings in a port range (IGD v2 only).
    GetListOfPortMappings(GetListOfPortMappingsParams),

    // WANCommonInterfaceConfig actions
    /// Get common link properties.
    GetCommonLinkProperties,

    /// Get total bytes received.
    GetTotalBytesReceived,

    /// Get total bytes sent.
    GetTotalBytesSent,

    // WANPPPConnection actions
    /// Get the upstream/downstream maximum bit rates of the PPP link
    /// (WANPPPConnection only).
    GetLinkLayerMaxBitRates,

    /// Match any action (wildcard).
    Any,
}

impl Action {
    /// Create an AddPortMapping action with matching parameters.
    pub fn add_port_mapping() -> AddPortMappingBuilder {
        AddPortMappingBuilder::default()
    }

    /// Create a DeletePortMapping action with matching parameters.
    pub fn delete_port_mapping() -> DeletePortMappingBuilder {
        DeletePortMappingBuilder::default()
    }

    /// Create a GetGenericPortMappingEntry action with matching parameters.
    pub fn get_generic_port_mapping_entry() -> GetGenericPortMappingEntryBuilder {
        GetGenericPortMappingEntryBuilder::default()
    }

    /// Create a GetSpecificPortMappingEntry action with matching parameters.
    pub fn get_specific_port_mapping_entry() -> GetSpecificPortMappingEntryBuilder {
        GetSpecificPortMappingEntryBuilder::default()
    }

    /// Create an AddAnyPortMapping action with matching parameters (IGD v2).
    pub fn add_any_port_mapping() -> AddAnyPortMappingBuilder {
        AddAnyPortMappingBuilder::default()
    }

    /// Create a DeletePortMappingRange action with matching parameters (IGD v2).
    pub fn delete_port_mapping_range() -> DeletePortMappingRangeBuilder {
        DeletePortMappingRangeBuilder::default()
    }

    /// Create a GetListOfPortMappings action with matching parameters (IGD v2).
    pub fn get_list_of_port_mappings() -> GetListOfPortMappingsBuilder {
        GetListOfPortMappingsBuilder::default()
    }

    /// Match any action.
    pub fn any() -> Self {
        Action::Any
    }
}

// =============================================================================
// AddPortMapping
// =============================================================================

/// Parameters for matching AddPortMapping requests.
#[derive(Debug, Clone, Default)]
pub struct AddPortMappingParams {
    pub external_port: Option<u16>,
    pub protocol: Option<Protocol>,
    pub internal_port: Option<u16>,
    pub internal_client: Option<IpAddr>,
    pub description: Option<String>,
}

/// Builder for AddPortMapping matching parameters.
#[derive(Debug, Clone, Default)]
pub struct AddPortMappingBuilder {
    params: AddPortMappingParams,
}

impl AddPortMappingBuilder {
    pub fn with_external_port(mut self, port: u16) -> Self {
        self.params.external_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn with_internal_port(mut self, port: u16) -> Self {
        self.params.internal_port = Some(port);
        self
    }

    pub fn with_internal_client(mut self, client: IpAddr) -> Self {
        self.params.internal_client = Some(client);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.params.description = Some(desc.into());
        self
    }

    pub fn build(self) -> Action {
        Action::AddPortMapping(self.params)
    }
}

impl From<AddPortMappingBuilder> for Action {
    fn from(builder: AddPortMappingBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// DeletePortMapping
// =============================================================================

/// Parameters for matching DeletePortMapping requests.
#[derive(Debug, Clone, Default)]
pub struct DeletePortMappingParams {
    pub external_port: Option<u16>,
    pub protocol: Option<Protocol>,
}

/// Builder for DeletePortMapping matching parameters.
#[derive(Debug, Clone, Default)]
pub struct DeletePortMappingBuilder {
    params: DeletePortMappingParams,
}

impl DeletePortMappingBuilder {
    pub fn with_external_port(mut self, port: u16) -> Self {
        self.params.external_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn build(self) -> Action {
        Action::DeletePortMapping(self.params)
    }
}

impl From<DeletePortMappingBuilder> for Action {
    fn from(builder: DeletePortMappingBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// GetGenericPortMappingEntry
// =============================================================================

/// Parameters for matching GetGenericPortMappingEntry requests.
#[derive(Debug, Clone, Default)]
pub struct GetGenericPortMappingEntryParams {
    pub index: Option<u32>,
}

/// Builder for GetGenericPortMappingEntry matching parameters.
#[derive(Debug, Clone, Default)]
pub struct GetGenericPortMappingEntryBuilder {
    params: GetGenericPortMappingEntryParams,
}

impl GetGenericPortMappingEntryBuilder {
    pub fn with_index(mut self, index: u32) -> Self {
        self.params.index = Some(index);
        self
    }

    pub fn build(self) -> Action {
        Action::GetGenericPortMappingEntry(self.params)
    }
}

impl From<GetGenericPortMappingEntryBuilder> for Action {
    fn from(builder: GetGenericPortMappingEntryBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// GetSpecificPortMappingEntry
// =============================================================================

/// Parameters for matching GetSpecificPortMappingEntry requests.
#[derive(Debug, Clone, Default)]
pub struct GetSpecificPortMappingEntryParams {
    pub external_port: Option<u16>,
    pub protocol: Option<Protocol>,
}

/// Builder for GetSpecificPortMappingEntry matching parameters.
#[derive(Debug, Clone, Default)]
pub struct GetSpecificPortMappingEntryBuilder {
    params: GetSpecificPortMappingEntryParams,
}

impl GetSpecificPortMappingEntryBuilder {
    pub fn with_external_port(mut self, port: u16) -> Self {
        self.params.external_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn build(self) -> Action {
        Action::GetSpecificPortMappingEntry(self.params)
    }
}

impl From<GetSpecificPortMappingEntryBuilder> for Action {
    fn from(builder: GetSpecificPortMappingEntryBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// AddAnyPortMapping (IGD v2)
// =============================================================================

/// Builder for AddAnyPortMapping matching parameters (IGD v2).
///
/// AddAnyPortMapping takes the same input arguments as AddPortMapping, so it
/// reuses [`AddPortMappingParams`].
#[derive(Debug, Clone, Default)]
pub struct AddAnyPortMappingBuilder {
    params: AddPortMappingParams,
}

impl AddAnyPortMappingBuilder {
    pub fn with_external_port(mut self, port: u16) -> Self {
        self.params.external_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn with_internal_port(mut self, port: u16) -> Self {
        self.params.internal_port = Some(port);
        self
    }

    pub fn with_internal_client(mut self, client: IpAddr) -> Self {
        self.params.internal_client = Some(client);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.params.description = Some(desc.into());
        self
    }

    pub fn build(self) -> Action {
        Action::AddAnyPortMapping(self.params)
    }
}

impl From<AddAnyPortMappingBuilder> for Action {
    fn from(builder: AddAnyPortMappingBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// DeletePortMappingRange (IGD v2)
// =============================================================================

/// Parameters for matching DeletePortMappingRange requests (IGD v2).
#[derive(Debug, Clone, Default)]
pub struct DeletePortMappingRangeParams {
    pub start_port: Option<u16>,
    pub end_port: Option<u16>,
    pub protocol: Option<Protocol>,
}

/// Builder for DeletePortMappingRange matching parameters (IGD v2).
#[derive(Debug, Clone, Default)]
pub struct DeletePortMappingRangeBuilder {
    params: DeletePortMappingRangeParams,
}

impl DeletePortMappingRangeBuilder {
    pub fn with_start_port(mut self, port: u16) -> Self {
        self.params.start_port = Some(port);
        self
    }

    pub fn with_end_port(mut self, port: u16) -> Self {
        self.params.end_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn build(self) -> Action {
        Action::DeletePortMappingRange(self.params)
    }
}

impl From<DeletePortMappingRangeBuilder> for Action {
    fn from(builder: DeletePortMappingRangeBuilder) -> Self {
        builder.build()
    }
}

// =============================================================================
// GetListOfPortMappings (IGD v2)
// =============================================================================

/// Parameters for matching GetListOfPortMappings requests (IGD v2).
#[derive(Debug, Clone, Default)]
pub struct GetListOfPortMappingsParams {
    pub start_port: Option<u16>,
    pub end_port: Option<u16>,
    pub protocol: Option<Protocol>,
}

/// Builder for GetListOfPortMappings matching parameters (IGD v2).
#[derive(Debug, Clone, Default)]
pub struct GetListOfPortMappingsBuilder {
    params: GetListOfPortMappingsParams,
}

impl GetListOfPortMappingsBuilder {
    pub fn with_start_port(mut self, port: u16) -> Self {
        self.params.start_port = Some(port);
        self
    }

    pub fn with_end_port(mut self, port: u16) -> Self {
        self.params.end_port = Some(port);
        self
    }

    pub fn with_protocol(mut self, protocol: Protocol) -> Self {
        self.params.protocol = Some(protocol);
        self
    }

    pub fn build(self) -> Action {
        Action::GetListOfPortMappings(self.params)
    }
}

impl From<GetListOfPortMappingsBuilder> for Action {
    fn from(builder: GetListOfPortMappingsBuilder) -> Self {
        builder.build()
    }
}
