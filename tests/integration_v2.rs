//! Integration tests for IGD v2 (InternetGatewayDevice:2) support.

use mock_igd::{Action, IgdVersion, MockIgdServer, Protocol, Responder};
use std::time::Duration;
use tokio::net::UdpSocket;

/// Helper to start a mock server emulating IGD v2.
async fn start_v2_server() -> MockIgdServer {
    MockIgdServer::builder()
        .igd_version(IgdVersion::V2)
        .start()
        .await
        .unwrap()
}

/// Helper to send a SOAP request with a WANIPConnection:2 SOAPAction header.
async fn soap_request_v2(url: &str, action: &str, body: &str) -> (u16, String) {
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
        .header(
            "SOAPAction",
            format!(
                "\"urn:schemas-upnp-org:service:WANIPConnection:2#{}\"",
                action
            ),
        )
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
async fn test_v2_device_description() {
    let server = start_v2_server().await;

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    assert!(body.contains("urn:schemas-upnp-org:device:InternetGatewayDevice:2"));
    assert!(body.contains("urn:schemas-upnp-org:device:WANDevice:2"));
    assert!(body.contains("urn:schemas-upnp-org:device:WANConnectionDevice:2"));
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:2"));
    // IGD v2 is based on UPnP Device Architecture 1.1
    assert!(body.contains("<minor>1</minor>"));
}

#[tokio::test]
async fn test_v1_device_description_is_default() {
    let server = MockIgdServer::start().await.unwrap();
    assert_eq!(server.igd_version(), IgdVersion::V1);

    let client = reqwest::Client::new();
    let response = client.get(server.description_url()).send().await.unwrap();

    let body = response.text().await.unwrap();
    assert!(body.contains("urn:schemas-upnp-org:device:InternetGatewayDevice:1"));
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:1"));
}

#[tokio::test]
async fn test_v2_scpd_contains_v2_actions() {
    let server = start_v2_server().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANIPCn.xml", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    // v1 actions are still present
    assert!(body.contains("<name>GetExternalIPAddress</name>"));
    assert!(body.contains("<name>AddPortMapping</name>"));
    // v2-only actions
    assert!(body.contains("<name>AddAnyPortMapping</name>"));
    assert!(body.contains("<name>DeletePortMappingRange</name>"));
    assert!(body.contains("<name>GetListOfPortMappings</name>"));
    // v2-only state variables
    assert!(body.contains("<name>A_ARG_TYPE_Manage</name>"));
    assert!(body.contains("<name>A_ARG_TYPE_PortListing</name>"));
    assert!(body.contains("<name>SystemUpdateID</name>"));
}

#[tokio::test]
async fn test_v1_scpd_has_no_v2_actions() {
    let server = MockIgdServer::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANIPCn.xml", server.url()))
        .send()
        .await
        .unwrap();

    let body = response.text().await.unwrap();
    assert!(!body.contains("AddAnyPortMapping"));
    assert!(!body.contains("DeletePortMappingRange"));
    assert!(!body.contains("GetListOfPortMappings"));
}

// =============================================================================
// AddAnyPortMapping tests
// =============================================================================

#[tokio::test]
async fn test_add_any_port_mapping() {
    let server = start_v2_server().await;

    server
        .mock(
            Action::add_any_port_mapping()
                .with_external_port(8080)
                .with_protocol(Protocol::TCP),
            Responder::success().with_reserved_port(8081),
        )
        .await;

    let (status, body) = soap_request_v2(
        &server.control_url(),
        "AddAnyPortMapping",
        r#"<u:AddAnyPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
            <NewInternalPort>8080</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Test</NewPortMappingDescription>
            <NewLeaseDuration>3600</NewLeaseDuration>
        </u:AddAnyPortMapping>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("AddAnyPortMappingResponse"));
    assert!(body.contains("<NewReservedPort>8081</NewReservedPort>"));
    // Response namespace echoes the v2 service type
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:2"));
}

#[tokio::test]
async fn test_add_any_port_mapping_does_not_match_add_port_mapping_mock() {
    let server = start_v2_server().await;

    // Only AddPortMapping is mocked
    server
        .mock(Action::add_port_mapping(), Responder::success())
        .await;

    let (status, _) = soap_request_v2(
        &server.control_url(),
        "AddAnyPortMapping",
        r#"<u:AddAnyPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
        </u:AddAnyPortMapping>"#,
    )
    .await;

    // No mock matches -> error response
    assert_eq!(status, 500);
}

// =============================================================================
// DeletePortMappingRange tests
// =============================================================================

#[tokio::test]
async fn test_delete_port_mapping_range() {
    let server = start_v2_server().await;

    server
        .mock(
            Action::delete_port_mapping_range()
                .with_start_port(8000)
                .with_end_port(8100)
                .with_protocol(Protocol::UDP),
            Responder::success(),
        )
        .await;

    let (status, body) = soap_request_v2(
        &server.control_url(),
        "DeletePortMappingRange",
        r#"<u:DeletePortMappingRange xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewStartPort>8000</NewStartPort>
            <NewEndPort>8100</NewEndPort>
            <NewProtocol>UDP</NewProtocol>
            <NewManage>1</NewManage>
        </u:DeletePortMappingRange>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("DeletePortMappingRangeResponse"));
}

#[tokio::test]
async fn test_delete_port_mapping_range_error() {
    let server = start_v2_server().await;

    server
        .mock(
            Action::delete_port_mapping_range().with_start_port(9000),
            Responder::error(730, "PortMappingNotFound"),
        )
        .await;

    let (status, body) = soap_request_v2(
        &server.control_url(),
        "DeletePortMappingRange",
        r#"<u:DeletePortMappingRange xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewStartPort>9000</NewStartPort>
            <NewEndPort>9100</NewEndPort>
            <NewProtocol>TCP</NewProtocol>
            <NewManage>0</NewManage>
        </u:DeletePortMappingRange>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>730</errorCode>"));
    assert!(body.contains("PortMappingNotFound"));
}

// =============================================================================
// GetListOfPortMappings tests
// =============================================================================

#[tokio::test]
async fn test_get_list_of_port_mappings_generated_listing() {
    let server = start_v2_server().await;

    server
        .mock(
            Action::get_list_of_port_mappings()
                .with_start_port(1)
                .with_end_port(65535),
            Responder::success()
                .with_external_port(8080)
                .with_protocol("TCP")
                .with_internal_port(8080)
                .with_internal_client("192.168.1.100")
                .with_description("Test")
                .with_lease_duration(3600),
        )
        .await;

    let (status, body) = soap_request_v2(
        &server.control_url(),
        "GetListOfPortMappings",
        r#"<u:GetListOfPortMappings xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewStartPort>1</NewStartPort>
            <NewEndPort>65535</NewEndPort>
            <NewProtocol>TCP</NewProtocol>
            <NewManage>1</NewManage>
            <NewNumberOfPorts>0</NewNumberOfPorts>
        </u:GetListOfPortMappings>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("GetListOfPortMappingsResponse"));
    assert!(body.contains("<NewPortListing>"));
    // The listing is XML-escaped inside NewPortListing
    assert!(body.contains("&lt;p:PortMappingList"));
    assert!(body.contains("&lt;p:NewExternalPort&gt;8080&lt;/p:NewExternalPort&gt;"));
    assert!(body.contains("&lt;p:NewInternalClient&gt;192.168.1.100&lt;/p:NewInternalClient&gt;"));
}

#[tokio::test]
async fn test_get_list_of_port_mappings_custom_listing() {
    let server = start_v2_server().await;

    server
        .mock(
            Action::get_list_of_port_mappings(),
            Responder::success().with_port_listing(
                r#"<p:PortMappingList xmlns:p="urn:schemas-upnp-org:gw:WANIPConnection"></p:PortMappingList>"#,
            ),
        )
        .await;

    let (status, body) = soap_request_v2(
        &server.control_url(),
        "GetListOfPortMappings",
        r#"<u:GetListOfPortMappings xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewStartPort>1</NewStartPort>
            <NewEndPort>100</NewEndPort>
            <NewProtocol>UDP</NewProtocol>
            <NewManage>0</NewManage>
            <NewNumberOfPorts>10</NewNumberOfPorts>
        </u:GetListOfPortMappings>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("&lt;p:PortMappingList"));
}

// =============================================================================
// Request recording tests
// =============================================================================

#[tokio::test]
async fn test_v2_requests_are_recorded() {
    let server = start_v2_server().await;

    server.mock(Action::any(), Responder::success()).await;

    let _ = soap_request_v2(
        &server.control_url(),
        "GetListOfPortMappings",
        r#"<u:GetListOfPortMappings xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:2">
            <NewStartPort>100</NewStartPort>
            <NewEndPort>200</NewEndPort>
            <NewProtocol>UDP</NewProtocol>
            <NewManage>1</NewManage>
            <NewNumberOfPorts>5</NewNumberOfPorts>
        </u:GetListOfPortMappings>"#,
    )
    .await;

    let requests = server.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action_name, "GetListOfPortMappings");
    assert_eq!(
        requests[0].service_type,
        "urn:schemas-upnp-org:service:WANIPConnection:2"
    );

    if let mock_igd::matcher::SoapRequestBody::GetListOfPortMappings(ref req) = requests[0].body {
        assert_eq!(req.start_port, 100);
        assert_eq!(req.end_port, 200);
        assert_eq!(req.protocol, "UDP");
        assert!(req.manage);
        assert_eq!(req.number_of_ports, 5);
    } else {
        panic!("Expected GetListOfPortMappings request body");
    }
}

// =============================================================================
// SSDP tests
// =============================================================================

/// Helper to start a server with SSDP enabled on a random port.
/// Returns None if the SSDP server could not start (e.g. permission issues).
async fn start_ssdp_server(igd_version: IgdVersion) -> Option<MockIgdServer> {
    let server = MockIgdServer::builder()
        .igd_version(igd_version)
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
async fn test_v2_ssdp_response_advertises_igd_v2() {
    let Some(server) = start_ssdp_server(IgdVersion::V2).await else {
        return;
    };

    let response = ssdp_search(
        &server,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:2",
    )
    .await
    .expect("timed out waiting for M-SEARCH response");

    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("ST: urn:schemas-upnp-org:device:InternetGatewayDevice:2"));
    assert!(
        response.contains(
            "USN: uuid:mock-igd-001::urn:schemas-upnp-org:device:InternetGatewayDevice:2"
        )
    );
    assert!(response.contains("UPnP/1.1"));
}

#[tokio::test]
async fn test_v2_ssdp_echoes_v1_device_search() {
    // An IGD v2 device is backward compatible: a search for
    // InternetGatewayDevice:1 is answered with a version 1 ST.
    let Some(server) = start_ssdp_server(IgdVersion::V2).await else {
        return;
    };

    let response = ssdp_search(
        &server,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:1",
    )
    .await
    .expect("timed out waiting for M-SEARCH response");

    assert!(response.contains("ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1"));
    assert!(
        response.contains(
            "USN: uuid:mock-igd-001::urn:schemas-upnp-org:device:InternetGatewayDevice:1"
        )
    );
}

#[tokio::test]
async fn test_v2_ssdp_echoes_v1_service_search() {
    let Some(server) = start_ssdp_server(IgdVersion::V2).await else {
        return;
    };

    let response = ssdp_search(&server, "urn:schemas-upnp-org:service:WANIPConnection:1")
        .await
        .expect("timed out waiting for M-SEARCH response");

    assert!(response.contains("ST: urn:schemas-upnp-org:service:WANIPConnection:1"));
}

#[tokio::test]
async fn test_v1_ssdp_ignores_v2_search() {
    // A v1 device does not support version 2, so a search for
    // InternetGatewayDevice:2 must not be answered.
    let Some(server) = start_ssdp_server(IgdVersion::V1).await else {
        return;
    };

    let response = ssdp_search(
        &server,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:2",
    )
    .await;
    assert!(response.is_none());

    // The same server still answers a version 1 search.
    let response = ssdp_search(
        &server,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:1",
    )
    .await
    .expect("timed out waiting for M-SEARCH response");
    assert!(response.contains("ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1"));
}

// =============================================================================
// v1 client against a v2 server (backward compatibility)
// =============================================================================

#[tokio::test]
async fn test_v2_server_accepts_v1_soap_request() {
    // A v1-only client sends WANIPConnection:1 SOAP requests to the same
    // control URL; the response namespace echoes the v1 service type.
    let server = start_v2_server().await;

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("192.0.2.1".parse().unwrap()),
        )
        .await;

    let client = reqwest::Client::new();
    let response = client
        .post(server.control_url())
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
    assert!(body.contains("<NewExternalIPAddress>192.0.2.1</NewExternalIPAddress>"));
    assert!(body.contains("urn:schemas-upnp-org:service:WANIPConnection:1"));
}
