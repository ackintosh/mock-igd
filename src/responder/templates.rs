//! XML/SOAP response templates.

use super::SuccessResponse;

/// SOAP envelope template.
const SOAP_ENVELOPE_START: &str = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>"#;

const SOAP_ENVELOPE_END: &str = r#"</s:Body>
</s:Envelope>"#;

/// Generate a SOAP fault response.
pub(crate) fn generate_soap_fault(code: u16, description: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<s:Fault>
<faultcode>s:Client</faultcode>
<faultstring>UPnPError</faultstring>
<detail>
<UPnPError xmlns="urn:schemas-upnp-org:control-1-0">
<errorCode>{code}</errorCode>
<errorDescription>{description}</errorDescription>
</UPnPError>
</detail>
</s:Fault>
</s:Body>
</s:Envelope>"#
    )
}

/// Default service type namespace for an action, used when the request did
/// not carry a SOAPACTION service type.
fn default_service_type(action_name: &str) -> &'static str {
    match action_name {
        "GetCommonLinkProperties" | "GetTotalBytesReceived" | "GetTotalBytesSent" => {
            "urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1"
        }
        "GetLinkLayerMaxBitRates" => "urn:schemas-upnp-org:service:WANPPPConnection:1",
        _ => "urn:schemas-upnp-org:service:WANIPConnection:1",
    }
}

/// Generate a successful SOAP response for the given action.
///
/// The response namespace echoes the service type from the request's
/// SOAPACTION header, so both WANIPConnection:1 and WANIPConnection:2
/// clients receive a matching namespace.
pub(crate) fn generate_success_response(
    action_name: &str,
    service_type: &str,
    data: &SuccessResponse,
) -> String {
    let ns = if service_type.is_empty() {
        default_service_type(action_name)
    } else {
        service_type
    };
    let body = match action_name {
        "GetExternalIPAddress" => generate_get_external_ip_response(ns, data),
        "GetStatusInfo" => generate_get_status_info_response(ns, data),
        "AddPortMapping" => generate_empty_response(action_name, ns),
        "DeletePortMapping" => generate_empty_response(action_name, ns),
        "GetGenericPortMappingEntry" | "GetSpecificPortMappingEntry" => {
            generate_get_port_mapping_entry_response(action_name, ns, data)
        }
        "AddAnyPortMapping" => generate_add_any_port_mapping_response(ns, data),
        "DeletePortMappingRange" => generate_empty_response(action_name, ns),
        "GetListOfPortMappings" => generate_get_list_of_port_mappings_response(ns, data),
        "GetCommonLinkProperties" => generate_get_common_link_properties_response(ns, data),
        "GetTotalBytesReceived" => generate_get_total_bytes_received_response(ns, data),
        "GetTotalBytesSent" => generate_get_total_bytes_sent_response(ns, data),
        "GetLinkLayerMaxBitRates" => generate_get_link_layer_max_bit_rates_response(ns, data),
        _ => generate_empty_response(action_name, ns),
    };

    format!("{SOAP_ENVELOPE_START}\n{body}\n{SOAP_ENVELOPE_END}")
}

/// Generate a response with no output arguments.
fn generate_empty_response(action_name: &str, ns: &str) -> String {
    format!("<u:{action_name}Response xmlns:u=\"{ns}\">\n</u:{action_name}Response>")
}

fn generate_get_external_ip_response(ns: &str, data: &SuccessResponse) -> String {
    let ip = data
        .external_ip
        .map(|ip| ip.to_string())
        .unwrap_or_default();
    format!(
        r#"<u:GetExternalIPAddressResponse xmlns:u="{ns}">
<NewExternalIPAddress>{ip}</NewExternalIPAddress>
</u:GetExternalIPAddressResponse>"#
    )
}

fn generate_get_status_info_response(ns: &str, data: &SuccessResponse) -> String {
    let connection_status = data.connection_status.as_deref().unwrap_or("Connected");
    let last_connection_error = data
        .last_connection_error
        .as_deref()
        .unwrap_or("ERROR_NONE");
    let uptime = data.uptime.unwrap_or(0);
    format!(
        r#"<u:GetStatusInfoResponse xmlns:u="{ns}">
<NewConnectionStatus>{connection_status}</NewConnectionStatus>
<NewLastConnectionError>{last_connection_error}</NewLastConnectionError>
<NewUptime>{uptime}</NewUptime>
</u:GetStatusInfoResponse>"#
    )
}

fn generate_get_port_mapping_entry_response(
    action_name: &str,
    ns: &str,
    data: &SuccessResponse,
) -> String {
    let remote_host = data.remote_host.as_deref().unwrap_or("");
    let external_port = data.external_port.unwrap_or(0);
    let protocol = data.protocol.as_deref().unwrap_or("TCP");
    let internal_port = data.internal_port.unwrap_or(0);
    let internal_client = data.internal_client.as_deref().unwrap_or("");
    let enabled = if data.enabled.unwrap_or(true) {
        "1"
    } else {
        "0"
    };
    let description = data.description.as_deref().unwrap_or("");
    let lease_duration = data.lease_duration.unwrap_or(0);

    format!(
        r#"<u:{action_name}Response xmlns:u="{ns}">
<NewRemoteHost>{remote_host}</NewRemoteHost>
<NewExternalPort>{external_port}</NewExternalPort>
<NewProtocol>{protocol}</NewProtocol>
<NewInternalPort>{internal_port}</NewInternalPort>
<NewInternalClient>{internal_client}</NewInternalClient>
<NewEnabled>{enabled}</NewEnabled>
<NewPortMappingDescription>{description}</NewPortMappingDescription>
<NewLeaseDuration>{lease_duration}</NewLeaseDuration>
</u:{action_name}Response>"#
    )
}

fn generate_add_any_port_mapping_response(ns: &str, data: &SuccessResponse) -> String {
    let reserved_port = data.reserved_port.or(data.external_port).unwrap_or(0);
    format!(
        r#"<u:AddAnyPortMappingResponse xmlns:u="{ns}">
<NewReservedPort>{reserved_port}</NewReservedPort>
</u:AddAnyPortMappingResponse>"#
    )
}

fn generate_get_list_of_port_mappings_response(ns: &str, data: &SuccessResponse) -> String {
    let listing = data
        .port_listing
        .clone()
        .unwrap_or_else(|| generate_port_mapping_list(data));
    let listing = escape_xml(&listing);
    format!(
        r#"<u:GetListOfPortMappingsResponse xmlns:u="{ns}">
<NewPortListing>{listing}</NewPortListing>
</u:GetListOfPortMappingsResponse>"#
    )
}

/// Build a WANIPConnection:2 PortMappingList document from the port mapping
/// fields, containing a single entry when an external port is set.
fn generate_port_mapping_list(data: &SuccessResponse) -> String {
    let entry = match data.external_port {
        Some(external_port) => {
            let remote_host = data.remote_host.as_deref().unwrap_or("");
            let protocol = data.protocol.as_deref().unwrap_or("TCP");
            let internal_port = data.internal_port.unwrap_or(0);
            let internal_client = data.internal_client.as_deref().unwrap_or("");
            let enabled = if data.enabled.unwrap_or(true) {
                "1"
            } else {
                "0"
            };
            let description = data.description.as_deref().unwrap_or("");
            let lease_duration = data.lease_duration.unwrap_or(0);
            format!(
                "<p:PortMappingEntry>\
<p:NewRemoteHost>{remote_host}</p:NewRemoteHost>\
<p:NewExternalPort>{external_port}</p:NewExternalPort>\
<p:NewProtocol>{protocol}</p:NewProtocol>\
<p:NewInternalPort>{internal_port}</p:NewInternalPort>\
<p:NewInternalClient>{internal_client}</p:NewInternalClient>\
<p:NewEnabled>{enabled}</p:NewEnabled>\
<p:NewDescription>{description}</p:NewDescription>\
<p:NewLeaseTime>{lease_duration}</p:NewLeaseTime>\
</p:PortMappingEntry>"
            )
        }
        None => String::new(),
    };
    format!(
        "<p:PortMappingList xmlns:p=\"urn:schemas-upnp-org:gw:WANIPConnection\" \
xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
xsi:schemaLocation=\"urn:schemas-upnp-org:gw:WANIPConnection \
http://www.upnp.org/schemas/gw/WANIPConnection-v2.xsd\">{entry}</p:PortMappingList>"
    )
}

/// Escape a string for embedding as XML text content.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn generate_get_common_link_properties_response(ns: &str, data: &SuccessResponse) -> String {
    let wan_access_type = data.wan_access_type.as_deref().unwrap_or("Cable");
    let upstream = data.layer1_upstream_max_bit_rate.unwrap_or(10000000);
    let downstream = data.layer1_downstream_max_bit_rate.unwrap_or(100000000);
    let status = data.physical_link_status.as_deref().unwrap_or("Up");

    format!(
        r#"<u:GetCommonLinkPropertiesResponse xmlns:u="{ns}">
<NewWANAccessType>{wan_access_type}</NewWANAccessType>
<NewLayer1UpstreamMaxBitRate>{upstream}</NewLayer1UpstreamMaxBitRate>
<NewLayer1DownstreamMaxBitRate>{downstream}</NewLayer1DownstreamMaxBitRate>
<NewPhysicalLinkStatus>{status}</NewPhysicalLinkStatus>
</u:GetCommonLinkPropertiesResponse>"#
    )
}

fn generate_get_link_layer_max_bit_rates_response(ns: &str, data: &SuccessResponse) -> String {
    let upstream = data.upstream_max_bit_rate.unwrap_or(10000000);
    let downstream = data.downstream_max_bit_rate.unwrap_or(100000000);
    format!(
        r#"<u:GetLinkLayerMaxBitRatesResponse xmlns:u="{ns}">
<NewUpstreamMaxBitRate>{upstream}</NewUpstreamMaxBitRate>
<NewDownstreamMaxBitRate>{downstream}</NewDownstreamMaxBitRate>
</u:GetLinkLayerMaxBitRatesResponse>"#
    )
}

fn generate_get_total_bytes_received_response(ns: &str, data: &SuccessResponse) -> String {
    let bytes = data.total_bytes.unwrap_or(0);
    format!(
        r#"<u:GetTotalBytesReceivedResponse xmlns:u="{ns}">
<NewTotalBytesReceived>{bytes}</NewTotalBytesReceived>
</u:GetTotalBytesReceivedResponse>"#
    )
}

fn generate_get_total_bytes_sent_response(ns: &str, data: &SuccessResponse) -> String {
    let bytes = data.total_bytes.unwrap_or(0);
    format!(
        r#"<u:GetTotalBytesSentResponse xmlns:u="{ns}">
<NewTotalBytesSent>{bytes}</NewTotalBytesSent>
</u:GetTotalBytesSentResponse>"#
    )
}
