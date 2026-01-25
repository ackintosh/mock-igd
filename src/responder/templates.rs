//! XML/SOAP response templates.

use super::SuccessResponse;

/// SOAP envelope template.
const SOAP_ENVELOPE_START: &str = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>"#;

const SOAP_ENVELOPE_END: &str = r#"</s:Body>
</s:Envelope>"#;

/// Generate a SOAP fault response.
pub fn generate_soap_fault(code: u16, description: &str) -> String {
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

/// Generate a successful SOAP response for the given action.
pub fn generate_success_response(action_name: &str, data: &SuccessResponse) -> String {
    let body = match action_name {
        "GetExternalIPAddress" => generate_get_external_ip_response(data),
        "AddPortMapping" => generate_add_port_mapping_response(),
        "DeletePortMapping" => generate_delete_port_mapping_response(),
        "GetGenericPortMappingEntry" => generate_get_port_mapping_entry_response(data),
        "GetSpecificPortMappingEntry" => generate_get_port_mapping_entry_response(data),
        "GetCommonLinkProperties" => generate_get_common_link_properties_response(data),
        "GetTotalBytesReceived" => generate_get_total_bytes_received_response(data),
        "GetTotalBytesSent" => generate_get_total_bytes_sent_response(data),
        _ => format!(
            "<u:{action_name}Response xmlns:u=\"urn:schemas-upnp-org:service:WANIPConnection:1\"></u:{action_name}Response>"
        ),
    };

    format!("{SOAP_ENVELOPE_START}\n{body}\n{SOAP_ENVELOPE_END}")
}

fn generate_get_external_ip_response(data: &SuccessResponse) -> String {
    let ip = data
        .external_ip
        .map(|ip| ip.to_string())
        .unwrap_or_default();
    format!(
        r#"<u:GetExternalIPAddressResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
<NewExternalIPAddress>{ip}</NewExternalIPAddress>
</u:GetExternalIPAddressResponse>"#
    )
}

fn generate_add_port_mapping_response() -> String {
    r#"<u:AddPortMappingResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
</u:AddPortMappingResponse>"#
        .to_string()
}

fn generate_delete_port_mapping_response() -> String {
    r#"<u:DeletePortMappingResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
</u:DeletePortMappingResponse>"#
        .to_string()
}

fn generate_get_port_mapping_entry_response(data: &SuccessResponse) -> String {
    let remote_host = data.remote_host.as_deref().unwrap_or("");
    let external_port = data.external_port.unwrap_or(0);
    let protocol = data.protocol.as_deref().unwrap_or("TCP");
    let internal_port = data.internal_port.unwrap_or(0);
    let internal_client = data.internal_client.as_deref().unwrap_or("");
    let enabled = if data.enabled.unwrap_or(true) { "1" } else { "0" };
    let description = data.description.as_deref().unwrap_or("");
    let lease_duration = data.lease_duration.unwrap_or(0);

    format!(
        r#"<u:GetGenericPortMappingEntryResponse xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
<NewRemoteHost>{remote_host}</NewRemoteHost>
<NewExternalPort>{external_port}</NewExternalPort>
<NewProtocol>{protocol}</NewProtocol>
<NewInternalPort>{internal_port}</NewInternalPort>
<NewInternalClient>{internal_client}</NewInternalClient>
<NewEnabled>{enabled}</NewEnabled>
<NewPortMappingDescription>{description}</NewPortMappingDescription>
<NewLeaseDuration>{lease_duration}</NewLeaseDuration>
</u:GetGenericPortMappingEntryResponse>"#
    )
}

fn generate_get_common_link_properties_response(data: &SuccessResponse) -> String {
    let wan_access_type = data.wan_access_type.as_deref().unwrap_or("Cable");
    let upstream = data.layer1_upstream_max_bit_rate.unwrap_or(10000000);
    let downstream = data.layer1_downstream_max_bit_rate.unwrap_or(100000000);
    let status = data.physical_link_status.as_deref().unwrap_or("Up");

    format!(
        r#"<u:GetCommonLinkPropertiesResponse xmlns:u="urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1">
<NewWANAccessType>{wan_access_type}</NewWANAccessType>
<NewLayer1UpstreamMaxBitRate>{upstream}</NewLayer1UpstreamMaxBitRate>
<NewLayer1DownstreamMaxBitRate>{downstream}</NewLayer1DownstreamMaxBitRate>
<NewPhysicalLinkStatus>{status}</NewPhysicalLinkStatus>
</u:GetCommonLinkPropertiesResponse>"#
    )
}

fn generate_get_total_bytes_received_response(data: &SuccessResponse) -> String {
    let bytes = data.total_bytes.unwrap_or(0);
    format!(
        r#"<u:GetTotalBytesReceivedResponse xmlns:u="urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1">
<NewTotalBytesReceived>{bytes}</NewTotalBytesReceived>
</u:GetTotalBytesReceivedResponse>"#
    )
}

fn generate_get_total_bytes_sent_response(data: &SuccessResponse) -> String {
    let bytes = data.total_bytes.unwrap_or(0);
    format!(
        r#"<u:GetTotalBytesSentResponse xmlns:u="urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1">
<NewTotalBytesSent>{bytes}</NewTotalBytesSent>
</u:GetTotalBytesSentResponse>"#
    )
}
