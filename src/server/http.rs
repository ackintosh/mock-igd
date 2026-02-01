//! HTTP/SOAP server implementation.

use crate::matcher::{
    AddPortMappingRequest, DeletePortMappingRequest, GetGenericPortMappingEntryRequest,
    GetSpecificPortMappingEntryRequest, SoapRequest, SoapRequestBody,
};
use crate::mock::MockRegistry;
use crate::responder::{generate_soap_fault, ResponseBody};
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Shared state for the HTTP server.
struct AppState {
    registry: Arc<MockRegistry>,
}

/// Run the HTTP server.
pub async fn run_http_server(
    listener: TcpListener,
    registry: Arc<MockRegistry>,
    shutdown_rx: oneshot::Receiver<()>,
) {
    let state = Arc::new(AppState { registry });

    let app = Router::new()
        .route("/rootDesc.xml", get(handle_root_desc))
        .route("/WANIPCn.xml", get(handle_wan_ip_connection_scpd))
        .route("/WANCommonIFC1.xml", get(handle_wan_common_ifc_scpd))
        .route("/ctl/IPConn", post(handle_soap_action))
        .route("/ctl/WANCommonIFC1", post(handle_soap_action))
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await
        .ok();
}

/// Handle device description request.
async fn handle_root_desc() -> impl IntoResponse {
    let xml = generate_device_description();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")
        .body(Body::from(xml))
        .unwrap()
}

/// Handle WANIPConnection SCPD request.
async fn handle_wan_ip_connection_scpd() -> impl IntoResponse {
    let xml = generate_wan_ip_connection_scpd();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")
        .body(Body::from(xml))
        .unwrap()
}

/// Handle WANCommonInterfaceConfig SCPD request.
async fn handle_wan_common_ifc_scpd() -> impl IntoResponse {
    let xml = generate_wan_common_ifc_scpd();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")
        .body(Body::from(xml))
        .unwrap()
}

/// Handle SOAP action requests.
async fn handle_soap_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    // Parse SOAP action from header
    let soap_action = headers
        .get("SOAPACTION")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Parse the request
    let request = match parse_soap_request(soap_action, &body) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!("Failed to parse SOAP request: {}", e);
            return soap_error_response(401, "Invalid Action");
        }
    };

    // Find a matching mock
    match state.registry.find_response(&request).await {
        Some(response) => match response {
            ResponseBody::Soap(xml) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")
                .body(Body::from(xml))
                .unwrap(),
            ResponseBody::SoapFault { code, description } => {
                soap_error_response(code, &description)
            }
            ResponseBody::Raw { content_type, body } => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .unwrap(),
        },
        None => {
            tracing::debug!("No mock found for action: {}", request.action_name);
            soap_error_response(401, "Invalid Action")
        }
    }
}

/// Generate a SOAP error response.
fn soap_error_response(code: u16, description: &str) -> Response<Body> {
    let xml = generate_soap_fault(code, description);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")
        .body(Body::from(xml))
        .unwrap()
}

/// Parse a SOAP request from the action header and body.
fn parse_soap_request(soap_action: &str, body: &str) -> Result<SoapRequest, String> {
    // Extract action name from SOAPACTION header
    // Format: "urn:schemas-upnp-org:service:WANIPConnection:1#GetExternalIPAddress"
    let action_name = soap_action
        .trim_matches('"')
        .rsplit('#')
        .next()
        .unwrap_or("")
        .to_string();

    let service_type = soap_action
        .trim_matches('"')
        .split('#')
        .next()
        .unwrap_or("")
        .to_string();

    // Parse body based on action
    let request_body = parse_soap_body(&action_name, body)?;

    Ok(SoapRequest {
        action_name,
        service_type,
        body: request_body,
    })
}

/// Parse the SOAP body into a structured request.
fn parse_soap_body(action_name: &str, body: &str) -> Result<SoapRequestBody, String> {
    match action_name {
        "GetExternalIPAddress" => Ok(SoapRequestBody::GetExternalIPAddress),
        "GetStatusInfo" => Ok(SoapRequestBody::GetStatusInfo),
        "AddPortMapping" => parse_add_port_mapping(body),
        "DeletePortMapping" => parse_delete_port_mapping(body),
        "GetGenericPortMappingEntry" => parse_get_generic_port_mapping_entry(body),
        "GetSpecificPortMappingEntry" => parse_get_specific_port_mapping_entry(body),
        "GetCommonLinkProperties" => Ok(SoapRequestBody::GetCommonLinkProperties),
        "GetTotalBytesReceived" => Ok(SoapRequestBody::GetTotalBytesReceived),
        "GetTotalBytesSent" => Ok(SoapRequestBody::GetTotalBytesSent),
        _ => Ok(SoapRequestBody::Unknown(action_name.to_string())),
    }
}

/// Extract a value from XML by tag name (simple implementation).
fn extract_xml_value(body: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);

    let start = body.find(&start_tag)?;
    let after_start = &body[start..];
    let tag_end = after_start.find('>')?;
    let content_start = start + tag_end + 1;

    let end = body[content_start..].find(&end_tag)?;
    Some(body[content_start..content_start + end].to_string())
}

fn parse_add_port_mapping(body: &str) -> Result<SoapRequestBody, String> {
    Ok(SoapRequestBody::AddPortMapping(AddPortMappingRequest {
        remote_host: extract_xml_value(body, "NewRemoteHost").unwrap_or_default(),
        external_port: extract_xml_value(body, "NewExternalPort")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        protocol: extract_xml_value(body, "NewProtocol").unwrap_or_else(|| "TCP".to_string()),
        internal_port: extract_xml_value(body, "NewInternalPort")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        internal_client: extract_xml_value(body, "NewInternalClient").unwrap_or_default(),
        enabled: extract_xml_value(body, "NewEnabled")
            .map(|s| s == "1" || s.to_lowercase() == "true")
            .unwrap_or(true),
        description: extract_xml_value(body, "NewPortMappingDescription").unwrap_or_default(),
        lease_duration: extract_xml_value(body, "NewLeaseDuration")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    }))
}

fn parse_delete_port_mapping(body: &str) -> Result<SoapRequestBody, String> {
    Ok(SoapRequestBody::DeletePortMapping(DeletePortMappingRequest {
        remote_host: extract_xml_value(body, "NewRemoteHost").unwrap_or_default(),
        external_port: extract_xml_value(body, "NewExternalPort")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        protocol: extract_xml_value(body, "NewProtocol").unwrap_or_else(|| "TCP".to_string()),
    }))
}

fn parse_get_generic_port_mapping_entry(body: &str) -> Result<SoapRequestBody, String> {
    Ok(SoapRequestBody::GetGenericPortMappingEntry(
        GetGenericPortMappingEntryRequest {
            index: extract_xml_value(body, "NewPortMappingIndex")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        },
    ))
}

fn parse_get_specific_port_mapping_entry(body: &str) -> Result<SoapRequestBody, String> {
    Ok(SoapRequestBody::GetSpecificPortMappingEntry(
        GetSpecificPortMappingEntryRequest {
            remote_host: extract_xml_value(body, "NewRemoteHost").unwrap_or_default(),
            external_port: extract_xml_value(body, "NewExternalPort")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            protocol: extract_xml_value(body, "NewProtocol").unwrap_or_else(|| "TCP".to_string()),
        },
    ))
}

/// Generate the UPnP device description XML.
fn generate_device_description() -> String {
    r#"<?xml version="1.0"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:InternetGatewayDevice:1</deviceType>
    <friendlyName>Mock IGD</friendlyName>
    <manufacturer>mock-igd</manufacturer>
    <modelName>Mock Internet Gateway Device</modelName>
    <UDN>uuid:mock-igd-001</UDN>
    <deviceList>
      <device>
        <deviceType>urn:schemas-upnp-org:device:WANDevice:1</deviceType>
        <friendlyName>WANDevice</friendlyName>
        <UDN>uuid:mock-igd-wan-001</UDN>
        <deviceList>
          <device>
            <deviceType>urn:schemas-upnp-org:device:WANConnectionDevice:1</deviceType>
            <friendlyName>WANConnectionDevice</friendlyName>
            <UDN>uuid:mock-igd-wanconn-001</UDN>
            <serviceList>
              <service>
                <serviceType>urn:schemas-upnp-org:service:WANIPConnection:1</serviceType>
                <serviceId>urn:upnp-org:serviceId:WANIPConn1</serviceId>
                <SCPDURL>/WANIPCn.xml</SCPDURL>
                <controlURL>/ctl/IPConn</controlURL>
                <eventSubURL>/evt/IPConn</eventSubURL>
              </service>
            </serviceList>
          </device>
        </deviceList>
        <serviceList>
          <service>
            <serviceType>urn:schemas-upnp-org:service:WANCommonInterfaceConfig:1</serviceType>
            <serviceId>urn:upnp-org:serviceId:WANCommonIFC1</serviceId>
            <SCPDURL>/WANCommonIFC1.xml</SCPDURL>
            <controlURL>/ctl/WANCommonIFC1</controlURL>
            <eventSubURL>/evt/WANCommonIFC1</eventSubURL>
          </service>
        </serviceList>
      </device>
    </deviceList>
  </device>
</root>"#
        .to_string()
}

/// Generate the WANIPConnection SCPD XML.
fn generate_wan_ip_connection_scpd() -> String {
    r#"<?xml version="1.0"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <actionList>
    <action>
      <name>GetExternalIPAddress</name>
      <argumentList>
        <argument>
          <name>NewExternalIPAddress</name>
          <direction>out</direction>
          <relatedStateVariable>ExternalIPAddress</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetStatusInfo</name>
      <argumentList>
        <argument>
          <name>NewConnectionStatus</name>
          <direction>out</direction>
          <relatedStateVariable>ConnectionStatus</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLastConnectionError</name>
          <direction>out</direction>
          <relatedStateVariable>LastConnectionError</relatedStateVariable>
        </argument>
        <argument>
          <name>NewUptime</name>
          <direction>out</direction>
          <relatedStateVariable>Uptime</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>AddPortMapping</name>
      <argumentList>
        <argument>
          <name>NewRemoteHost</name>
          <direction>in</direction>
          <relatedStateVariable>RemoteHost</relatedStateVariable>
        </argument>
        <argument>
          <name>NewExternalPort</name>
          <direction>in</direction>
          <relatedStateVariable>ExternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewProtocol</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingProtocol</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalPort</name>
          <direction>in</direction>
          <relatedStateVariable>InternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalClient</name>
          <direction>in</direction>
          <relatedStateVariable>InternalClient</relatedStateVariable>
        </argument>
        <argument>
          <name>NewEnabled</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingEnabled</relatedStateVariable>
        </argument>
        <argument>
          <name>NewPortMappingDescription</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingDescription</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLeaseDuration</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingLeaseDuration</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>DeletePortMapping</name>
      <argumentList>
        <argument>
          <name>NewRemoteHost</name>
          <direction>in</direction>
          <relatedStateVariable>RemoteHost</relatedStateVariable>
        </argument>
        <argument>
          <name>NewExternalPort</name>
          <direction>in</direction>
          <relatedStateVariable>ExternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewProtocol</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingProtocol</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetGenericPortMappingEntry</name>
      <argumentList>
        <argument>
          <name>NewPortMappingIndex</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingNumberOfEntries</relatedStateVariable>
        </argument>
        <argument>
          <name>NewRemoteHost</name>
          <direction>out</direction>
          <relatedStateVariable>RemoteHost</relatedStateVariable>
        </argument>
        <argument>
          <name>NewExternalPort</name>
          <direction>out</direction>
          <relatedStateVariable>ExternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewProtocol</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingProtocol</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalPort</name>
          <direction>out</direction>
          <relatedStateVariable>InternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalClient</name>
          <direction>out</direction>
          <relatedStateVariable>InternalClient</relatedStateVariable>
        </argument>
        <argument>
          <name>NewEnabled</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingEnabled</relatedStateVariable>
        </argument>
        <argument>
          <name>NewPortMappingDescription</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingDescription</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLeaseDuration</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingLeaseDuration</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetSpecificPortMappingEntry</name>
      <argumentList>
        <argument>
          <name>NewRemoteHost</name>
          <direction>in</direction>
          <relatedStateVariable>RemoteHost</relatedStateVariable>
        </argument>
        <argument>
          <name>NewExternalPort</name>
          <direction>in</direction>
          <relatedStateVariable>ExternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewProtocol</name>
          <direction>in</direction>
          <relatedStateVariable>PortMappingProtocol</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalPort</name>
          <direction>out</direction>
          <relatedStateVariable>InternalPort</relatedStateVariable>
        </argument>
        <argument>
          <name>NewInternalClient</name>
          <direction>out</direction>
          <relatedStateVariable>InternalClient</relatedStateVariable>
        </argument>
        <argument>
          <name>NewEnabled</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingEnabled</relatedStateVariable>
        </argument>
        <argument>
          <name>NewPortMappingDescription</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingDescription</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLeaseDuration</name>
          <direction>out</direction>
          <relatedStateVariable>PortMappingLeaseDuration</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no">
      <name>ExternalIPAddress</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>ConnectionStatus</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>Unconfigured</allowedValue>
        <allowedValue>Connected</allowedValue>
        <allowedValue>Disconnected</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>LastConnectionError</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>Uptime</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>RemoteHost</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>ExternalPort</name>
      <dataType>ui2</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>PortMappingProtocol</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>TCP</allowedValue>
        <allowedValue>UDP</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>InternalPort</name>
      <dataType>ui2</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>InternalClient</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>PortMappingEnabled</name>
      <dataType>boolean</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>PortMappingDescription</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>PortMappingLeaseDuration</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="yes">
      <name>PortMappingNumberOfEntries</name>
      <dataType>ui2</dataType>
    </stateVariable>
  </serviceStateTable>
</scpd>"#
        .to_string()
}

/// Generate the WANCommonInterfaceConfig SCPD XML.
fn generate_wan_common_ifc_scpd() -> String {
    r#"<?xml version="1.0"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <actionList>
    <action>
      <name>GetCommonLinkProperties</name>
      <argumentList>
        <argument>
          <name>NewWANAccessType</name>
          <direction>out</direction>
          <relatedStateVariable>WANAccessType</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLayer1UpstreamMaxBitRate</name>
          <direction>out</direction>
          <relatedStateVariable>Layer1UpstreamMaxBitRate</relatedStateVariable>
        </argument>
        <argument>
          <name>NewLayer1DownstreamMaxBitRate</name>
          <direction>out</direction>
          <relatedStateVariable>Layer1DownstreamMaxBitRate</relatedStateVariable>
        </argument>
        <argument>
          <name>NewPhysicalLinkStatus</name>
          <direction>out</direction>
          <relatedStateVariable>PhysicalLinkStatus</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetTotalBytesReceived</name>
      <argumentList>
        <argument>
          <name>NewTotalBytesReceived</name>
          <direction>out</direction>
          <relatedStateVariable>TotalBytesReceived</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetTotalBytesSent</name>
      <argumentList>
        <argument>
          <name>NewTotalBytesSent</name>
          <direction>out</direction>
          <relatedStateVariable>TotalBytesSent</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no">
      <name>WANAccessType</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>DSL</allowedValue>
        <allowedValue>POTS</allowedValue>
        <allowedValue>Cable</allowedValue>
        <allowedValue>Ethernet</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>Layer1UpstreamMaxBitRate</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>Layer1DownstreamMaxBitRate</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="yes">
      <name>PhysicalLinkStatus</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>Up</allowedValue>
        <allowedValue>Down</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>TotalBytesReceived</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>TotalBytesSent</name>
      <dataType>ui4</dataType>
    </stateVariable>
  </serviceStateTable>
</scpd>"#
        .to_string()
}
