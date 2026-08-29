//! Integration tests for WANPPPConnection support.

use mock_igd::{Action, ConnectionService, IgdVersion, MockIgdServer, Responder};
use std::time::Duration;
use tokio::net::UdpSocket;

const PPP_SERVICE_TYPE: &str = "urn:schemas-upnp-org:service:WANPPPConnection:1";

/// Helper to start a mock server exposing the given connection service.
async fn start_server(service: ConnectionService) -> MockIgdServer {
    MockIgdServer::builder()
        .connection_service(service)
        .start()
        .await
        .unwrap()
}

/// Helper to send a SOAP request with a WANPPPConnection:1 SOAPAction header.
async fn soap_request_ppp(url: &str, action: &str, body: &str) -> (u16, String) {
    let soap_body = format!(
        r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
{}
</s:Body>
</s:Envelope>"#,
        body
    );

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPAction", format!("\"{}#{}\"", PPP_SERVICE_TYPE, action))
        .body(soap_body)
        .send()
        .await
        .unwrap();

    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    (status, text)
}

// =============================================================================
// Device description tests
// =============================================================================

#[tokio::test]
async fn test_ppp_device_description() {
    let server = start_server(ConnectionService::Ppp).await;
    assert_eq!(server.connection_service(), ConnectionService::Ppp);

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    assert!(body.contains(PPP_SERVICE_TYPE));
    assert!(body.contains("<serviceId>urn:upnp-org:serviceId:WANPPPConn1</serviceId>"));
    assert!(body.contains("<SCPDURL>/WANPPPCn.xml</SCPDURL>"));
    assert!(body.contains("<controlURL>/ctl/PPPConn</controlURL>"));
    // A PPP-only device does not advertise WANIPConnection.
    assert!(!body.contains("urn:schemas-upnp-org:service:WANIPConnection"));
}

#[tokio::test]
async fn test_ip_connection_is_default() {
    let server = MockIgdServer::start().await.unwrap();
    assert_eq!(server.connection_service(), ConnectionService::Ip);

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    let body = response.text().await.unwrap();
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:1"));
    assert!(!body.contains("WANPPPConnection"));
}

#[tokio::test]
async fn test_both_device_description_advertises_both_services() {
    let server = start_server(ConnectionService::Both).await;

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    let body = response.text().await.unwrap();
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:1"));
    assert!(body.contains(PPP_SERVICE_TYPE));
    assert!(body.contains("<controlURL>/ctl/IPConn</controlURL>"));
    assert!(body.contains("<controlURL>/ctl/PPPConn</controlURL>"));
}

#[tokio::test]
async fn test_v2_device_with_ppp_service() {
    // WANPPPConnection is only defined in version 1, also on IGD v2 devices.
    let server = MockIgdServer::builder()
        .igd_version(IgdVersion::V2)
        .connection_service(ConnectionService::Ppp)
        .start()
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    let body = response.text().await.unwrap();
    assert!(body.contains("urn:schemas-upnp-org:device:InternetGatewayDevice:2"));
    assert!(body.contains("urn:schemas-upnp-org:device:WANConnectionDevice:2"));
    assert!(body.contains(PPP_SERVICE_TYPE));
    assert!(!body.contains("urn:schemas-upnp-org:service:WANPPPConnection:2"));
}

// =============================================================================
// SCPD tests
// =============================================================================

#[tokio::test]
async fn test_ppp_scpd() {
    let server = start_server(ConnectionService::Ppp).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANPPPCn.xml", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    // Actions shared with WANIPConnection
    assert!(body.contains("<name>GetExternalIPAddress</name>"));
    assert!(body.contains("<name>AddPortMapping</name>"));
    assert!(body.contains("<name>DeletePortMapping</name>"));
    assert!(body.contains("<name>GetSpecificPortMappingEntry</name>"));
    // PPP-only action and state variables
    assert!(body.contains("<name>GetLinkLayerMaxBitRates</name>"));
    assert!(body.contains("<name>UpstreamMaxBitRate</name>"));
    assert!(body.contains("<name>DownstreamMaxBitRate</name>"));
}

#[tokio::test]
async fn test_ppp_only_server_does_not_serve_ip_endpoints() {
    let server = start_server(ConnectionService::Ppp).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANIPCn.xml", server.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);

    let response = client
        .post(server.ip_control_url())
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header(
            "SOAPAction",
            "\"urn:schemas-upnp-org:service:WANIPConnection:1#GetExternalIPAddress\"",
        )
        .body("<s:Envelope></s:Envelope>")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn test_ip_only_server_does_not_serve_ppp_endpoints() {
    let server = MockIgdServer::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANPPPCn.xml", server.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
}

// =============================================================================
// SOAP action tests
// =============================================================================

#[tokio::test]
async fn test_ppp_control_url() {
    let server = start_server(ConnectionService::Ppp).await;
    // The primary control URL of a PPP-only server is the PPP one.
    assert_eq!(server.control_url(), server.ppp_control_url());
    assert!(server.control_url().ends_with("/ctl/PPPConn"));
}

#[tokio::test]
async fn test_ppp_get_external_ip_address() {
    let server = start_server(ConnectionService::Ppp).await;

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("203.0.113.7".parse().unwrap()),
        )
        .await;

    let (status, body) = soap_request_ppp(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
</u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("<NewExternalIPAddress>203.0.113.7</NewExternalIPAddress>"));
    // The response namespace echoes the WANPPPConnection service type.
    assert!(body.contains(PPP_SERVICE_TYPE));
}

#[tokio::test]
async fn test_ppp_add_port_mapping() {
    let server = start_server(ConnectionService::Ppp).await;

    server
        .mock(
            Action::add_port_mapping().with_external_port(8080),
            Responder::success(),
        )
        .await;

    let (status, body) = soap_request_ppp(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
<NewRemoteHost></NewRemoteHost>
<NewExternalPort>8080</NewExternalPort>
<NewProtocol>TCP</NewProtocol>
<NewInternalPort>8080</NewInternalPort>
<NewInternalClient>192.168.1.100</NewInternalClient>
<NewEnabled>1</NewEnabled>
<NewPortMappingDescription>test</NewPortMappingDescription>
<NewLeaseDuration>0</NewLeaseDuration>
</u:AddPortMapping>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("AddPortMappingResponse"));
    assert!(body.contains(PPP_SERVICE_TYPE));
}

#[tokio::test]
async fn test_ppp_get_link_layer_max_bit_rates() {
    let server = start_server(ConnectionService::Ppp).await;

    server
        .mock(
            Action::GetLinkLayerMaxBitRates,
            Responder::success()
                .with_upstream_max_bit_rate(1_000_000)
                .with_downstream_max_bit_rate(8_000_000),
        )
        .await;

    let (status, body) = soap_request_ppp(
        &server.control_url(),
        "GetLinkLayerMaxBitRates",
        r#"<u:GetLinkLayerMaxBitRates xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
</u:GetLinkLayerMaxBitRates>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("<NewUpstreamMaxBitRate>1000000</NewUpstreamMaxBitRate>"));
    assert!(body.contains("<NewDownstreamMaxBitRate>8000000</NewDownstreamMaxBitRate>"));
    assert!(body.contains(PPP_SERVICE_TYPE));
}

#[tokio::test]
async fn test_ppp_error_response() {
    let server = start_server(ConnectionService::Ppp).await;

    server
        .mock(
            Action::add_port_mapping().with_external_port(80),
            Responder::error(718, "ConflictInMappingEntry"),
        )
        .await;

    let (status, body) = soap_request_ppp(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
<NewExternalPort>80</NewExternalPort>
<NewProtocol>TCP</NewProtocol>
</u:AddPortMapping>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>718</errorCode>"));
    assert!(body.contains("<errorDescription>ConflictInMappingEntry</errorDescription>"));
}

#[tokio::test]
async fn test_both_services_share_registered_mocks() {
    let server = start_server(ConnectionService::Both).await;

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("198.51.100.9".parse().unwrap()),
        )
        .await;

    // The same mock answers on both control URLs.
    let (status, body) = soap_request_ppp(
        &server.ppp_control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
</u:GetExternalIPAddress>"#,
    )
    .await;
    assert_eq!(status, 200);
    assert!(body.contains("<NewExternalIPAddress>198.51.100.9</NewExternalIPAddress>"));
    assert!(body.contains(PPP_SERVICE_TYPE));

    let client = reqwest::Client::new();
    let response = client
        .post(server.ip_control_url())
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header(
            "SOAPAction",
            "\"urn:schemas-upnp-org:service:WANIPConnection:1#GetExternalIPAddress\"",
        )
        .body(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
</u:GetExternalIPAddress>
</s:Body>
</s:Envelope>"#,
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("<NewExternalIPAddress>198.51.100.9</NewExternalIPAddress>"));
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:1"));
}

#[tokio::test]
async fn test_ppp_requests_are_recorded() {
    let server = start_server(ConnectionService::Ppp).await;

    server.mock(Action::any(), Responder::success()).await;

    let _ = soap_request_ppp(
        &server.control_url(),
        "GetLinkLayerMaxBitRates",
        r#"<u:GetLinkLayerMaxBitRates xmlns:u="urn:schemas-upnp-org:service:WANPPPConnection:1">
</u:GetLinkLayerMaxBitRates>"#,
    )
    .await;

    let requests = server.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action_name, "GetLinkLayerMaxBitRates");
    assert_eq!(requests[0].service_type, PPP_SERVICE_TYPE);
}

// =============================================================================
// SSDP tests
// =============================================================================

/// Helper to start a server with SSDP enabled on a random port.
/// Returns None if the SSDP server could not start (e.g. permission issues).
async fn start_ssdp_server(service: ConnectionService) -> Option<MockIgdServer> {
    let server = MockIgdServer::builder()
        .connection_service(service)
        .ssdp_port(0)
        .start()
        .await;
    match server {
        Ok(s) if s.ssdp_addr().is_some() => Some(s),
        _ => {
            eprintln!("Skipping SSDP test - could not start SSDP server");
            None
        }
    }
}

/// Helper to send an M-SEARCH with the given ST and wait for a response.
/// Returns None if no response arrives within the timeout.
async fn ssdp_search(server: &MockIgdServer, st: &str) -> Option<String> {
    let ssdp_addr = server.ssdp_addr().unwrap();

    let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let request = format!(
        "M-SEARCH * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         MAN: \"ssdp:discover\"\r\n\
         MX: 3\r\n\
         ST: {st}\r\n\
         \r\n"
    );
    socket.send_to(request.as_bytes(), ssdp_addr).await.unwrap();

    let mut buf = [0u8; 2048];
    match tokio::time::timeout(Duration::from_millis(1500), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, _))) => Some(String::from_utf8_lossy(&buf[..len]).to_string()),
        _ => None,
    }
}

#[tokio::test]
async fn test_ppp_ssdp_answers_ppp_search() {
    let Some(server) = start_ssdp_server(ConnectionService::Ppp).await else {
        return;
    };

    let response = ssdp_search(&server, PPP_SERVICE_TYPE)
        .await
        .expect("timed out waiting for M-SEARCH response");

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains(&format!("ST: {PPP_SERVICE_TYPE}")));
    assert!(response.contains(&format!("USN: uuid:mock-igd-001::{PPP_SERVICE_TYPE}")));
}

#[tokio::test]
async fn test_ppp_ssdp_ignores_ip_connection_search() {
    let Some(server) = start_ssdp_server(ConnectionService::Ppp).await else {
        return;
    };

    let response = ssdp_search(&server, "urn:schemas-upnp-org:service:WANIPConnection:1").await;
    assert!(response.is_none());

    // The device itself is still discoverable.
    let response = ssdp_search(
        &server,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:1",
    )
    .await
    .expect("timed out waiting for M-SEARCH response");
    assert!(response.contains("ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1"));
}

#[tokio::test]
async fn test_ip_ssdp_ignores_ppp_connection_search() {
    let Some(server) = start_ssdp_server(ConnectionService::Ip).await else {
        return;
    };

    let response = ssdp_search(&server, PPP_SERVICE_TYPE).await;
    assert!(response.is_none());
}

#[tokio::test]
async fn test_ppp_ssdp_ignores_wan_ppp_connection_version_2() {
    // WANPPPConnection is only defined in version 1.
    let Some(server) = start_ssdp_server(ConnectionService::Ppp).await else {
        return;
    };

    let response = ssdp_search(&server, "urn:schemas-upnp-org:service:WANPPPConnection:2").await;
    assert!(response.is_none());
}

#[tokio::test]
async fn test_both_ssdp_answers_either_service_search() {
    let Some(server) = start_ssdp_server(ConnectionService::Both).await else {
        return;
    };

    let response = ssdp_search(&server, PPP_SERVICE_TYPE)
        .await
        .expect("timed out waiting for M-SEARCH response");
    assert!(response.contains(&format!("ST: {PPP_SERVICE_TYPE}")));

    let response = ssdp_search(&server, "urn:schemas-upnp-org:service:WANIPConnection:1")
        .await
        .expect("timed out waiting for M-SEARCH response");
    assert!(response.contains("ST: urn:schemas-upnp-org:service:WANIPConnection:1"));
}
