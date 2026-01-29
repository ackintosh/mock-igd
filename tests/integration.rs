//! Integration tests for mock-igd server.

use mock_igd::{Action, MockIgdServer, Protocol, Responder};
use std::net::UdpSocket;

/// Helper to send a SOAP request and return the response body.
async fn soap_request(url: &str, action: &str, body: &str) -> (u16, String) {
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
                "\"urn:schemas-upnp-org:service:WANIPConnection:1#{}\"",
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
// GetExternalIPAddress tests
// =============================================================================

#[tokio::test]
async fn test_get_external_ip_address() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("192.0.2.1".parse().unwrap()),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("<NewExternalIPAddress>192.0.2.1</NewExternalIPAddress>"));
}

// =============================================================================
// GetStatusInfo tests
// =============================================================================

#[tokio::test]
async fn test_get_status_info() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::GetStatusInfo,
            Responder::success()
                .with_connection_status("Connected")
                .with_last_connection_error("ERROR_NONE")
                .with_uptime(86400),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "GetStatusInfo",
        r#"<u:GetStatusInfo xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetStatusInfo>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("<NewConnectionStatus>Connected</NewConnectionStatus>"));
    assert!(body.contains("<NewLastConnectionError>ERROR_NONE</NewLastConnectionError>"));
    assert!(body.contains("<NewUptime>86400</NewUptime>"));
}

#[tokio::test]
async fn test_get_external_ip_address_error() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::error(501, "ActionFailed"),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>501</errorCode>"));
    assert!(body.contains("<errorDescription>ActionFailed</errorDescription>"));
}

// =============================================================================
// AddPortMapping tests
// =============================================================================

#[tokio::test]
async fn test_add_port_mapping_success() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::add_port_mapping()
                .with_external_port(8080)
                .with_protocol(Protocol::TCP),
            Responder::success(),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
            <NewInternalPort>8080</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Test</NewPortMappingDescription>
            <NewLeaseDuration>0</NewLeaseDuration>
        </u:AddPortMapping>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("AddPortMappingResponse"));
}

#[tokio::test]
async fn test_add_port_mapping_conflict_error() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::add_port_mapping().with_external_port(80),
            Responder::error(718, "ConflictInMappingEntry"),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>80</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
            <NewInternalPort>80</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Web</NewPortMappingDescription>
            <NewLeaseDuration>0</NewLeaseDuration>
        </u:AddPortMapping>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>718</errorCode>"));
    assert!(body.contains("ConflictInMappingEntry"));
}

#[tokio::test]
async fn test_add_port_mapping_match_by_protocol() {
    let server = MockIgdServer::start().await.unwrap();

    // TCP should succeed
    server
        .mock(
            Action::add_port_mapping()
                .with_external_port(5000)
                .with_protocol(Protocol::TCP),
            Responder::success(),
        )
        .await;

    // UDP should fail
    server
        .mock(
            Action::add_port_mapping()
                .with_external_port(5000)
                .with_protocol(Protocol::UDP),
            Responder::error(718, "ConflictInMappingEntry"),
        )
        .await;

    // Test TCP - should succeed
    let (status, _) = soap_request(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>5000</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
            <NewInternalPort>5000</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Test</NewPortMappingDescription>
            <NewLeaseDuration>0</NewLeaseDuration>
        </u:AddPortMapping>"#,
    )
    .await;
    assert_eq!(status, 200);

    // Test UDP - should fail
    let (status, body) = soap_request(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>5000</NewExternalPort>
            <NewProtocol>UDP</NewProtocol>
            <NewInternalPort>5000</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Test</NewPortMappingDescription>
            <NewLeaseDuration>0</NewLeaseDuration>
        </u:AddPortMapping>"#,
    )
    .await;
    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>718</errorCode>"));
}

// =============================================================================
// DeletePortMapping tests
// =============================================================================

#[tokio::test]
async fn test_delete_port_mapping_success() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::delete_port_mapping()
                .with_external_port(8080)
                .with_protocol(Protocol::TCP),
            Responder::success(),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "DeletePortMapping",
        r#"<u:DeletePortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
        </u:DeletePortMapping>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("DeletePortMappingResponse"));
}

#[tokio::test]
async fn test_delete_port_mapping_not_found() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::delete_port_mapping().with_external_port(9999),
            Responder::error(714, "NoSuchEntryInArray"),
        )
        .await;

    let (status, body) = soap_request(
        &server.control_url(),
        "DeletePortMapping",
        r#"<u:DeletePortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>9999</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
        </u:DeletePortMapping>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>714</errorCode>"));
    assert!(body.contains("NoSuchEntryInArray"));
}

// =============================================================================
// Mock priority and times tests
// =============================================================================

#[tokio::test]
async fn test_mock_priority() {
    let server = MockIgdServer::start().await.unwrap();

    // Lower priority - catch-all error
    server
        .mock_with_priority(
            Action::GetExternalIPAddress,
            Responder::error(501, "DefaultError"),
            0,
        )
        .await;

    // Higher priority - specific success
    server
        .mock_with_priority(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("10.0.0.1".parse().unwrap()),
            10,
        )
        .await;

    // Should match the higher priority mock
    let (status, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.contains("<NewExternalIPAddress>10.0.0.1</NewExternalIPAddress>"));
}

#[tokio::test]
async fn test_mock_times_limit() {
    let server = MockIgdServer::start().await.unwrap();

    // First response - only once
    server
        .mock_with_times(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("1.1.1.1".parse().unwrap()),
            1,
        )
        .await;

    // Fallback response
    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("2.2.2.2".parse().unwrap()),
        )
        .await;

    // First request should return 1.1.1.1
    let (_, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;
    assert!(body.contains("1.1.1.1"));

    // Second request should return 2.2.2.2 (first mock exhausted)
    let (_, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;
    assert!(body.contains("2.2.2.2"));
}

// =============================================================================
// Any action matcher tests
// =============================================================================

#[tokio::test]
async fn test_any_action_matcher() {
    let server = MockIgdServer::start().await.unwrap();

    // Catch-all for any unhandled action
    server
        .mock(Action::any(), Responder::error(501, "ActionNotImplemented"))
        .await;

    // Any action should return the error
    let (status, body) = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(status, 500);
    assert!(body.contains("<errorCode>501</errorCode>"));
    assert!(body.contains("ActionNotImplemented"));
}

// =============================================================================
// Device description tests
// =============================================================================

#[tokio::test]
async fn test_device_description() {
    let server = MockIgdServer::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/rootDesc.xml", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    assert!(body.contains("InternetGatewayDevice"));
    assert!(body.contains("WANIPConnection"));
}

#[tokio::test]
async fn test_wan_ip_connection_scpd() {
    let server = MockIgdServer::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANIPCn.xml", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    // Verify it's a valid SCPD
    assert!(body.contains("<scpd xmlns=\"urn:schemas-upnp-org:service-1-0\">"));
    // Verify actions are defined
    assert!(body.contains("<name>GetExternalIPAddress</name>"));
    assert!(body.contains("<name>GetStatusInfo</name>"));
    assert!(body.contains("<name>AddPortMapping</name>"));
    assert!(body.contains("<name>DeletePortMapping</name>"));
    assert!(body.contains("<name>GetGenericPortMappingEntry</name>"));
    assert!(body.contains("<name>GetSpecificPortMappingEntry</name>"));
    // Verify state variables are defined
    assert!(body.contains("<name>ExternalIPAddress</name>"));
    assert!(body.contains("<name>PortMappingProtocol</name>"));
}

#[tokio::test]
async fn test_wan_common_ifc_scpd() {
    let server = MockIgdServer::start().await.unwrap();

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/WANCommonIFC1.xml", server.url()))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);

    let body = response.text().await.unwrap();
    // Verify it's a valid SCPD
    assert!(body.contains("<scpd xmlns=\"urn:schemas-upnp-org:service-1-0\">"));
    // Verify actions are defined
    assert!(body.contains("<name>GetCommonLinkProperties</name>"));
    assert!(body.contains("<name>GetTotalBytesReceived</name>"));
    assert!(body.contains("<name>GetTotalBytesSent</name>"));
    // Verify state variables are defined
    assert!(body.contains("<name>WANAccessType</name>"));
    assert!(body.contains("<name>PhysicalLinkStatus</name>"));
}

// =============================================================================
// Received requests tests
// =============================================================================

#[tokio::test]
async fn test_received_requests() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("192.0.2.1".parse().unwrap()),
        )
        .await;

    // Initially no requests
    let requests = server.received_requests().await;
    assert!(requests.is_empty());

    // Send a request
    let _ = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    // Should have one request
    let requests = server.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action_name, "GetExternalIPAddress");
}

#[tokio::test]
async fn test_received_requests_multiple() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(Action::any(), Responder::success())
        .await;

    // Send multiple requests
    let _ = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    let _ = soap_request(
        &server.control_url(),
        "AddPortMapping",
        r#"<u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
            <NewInternalPort>8080</NewInternalPort>
            <NewInternalClient>192.168.1.100</NewInternalClient>
            <NewEnabled>1</NewEnabled>
            <NewPortMappingDescription>Test</NewPortMappingDescription>
            <NewLeaseDuration>0</NewLeaseDuration>
        </u:AddPortMapping>"#,
    )
    .await;

    let requests = server.received_requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].action_name, "GetExternalIPAddress");
    assert_eq!(requests[1].action_name, "AddPortMapping");

    // Verify request body details
    if let mock_igd::matcher::SoapRequestBody::AddPortMapping(ref req) = requests[1].body {
        assert_eq!(req.external_port, 8080);
        assert_eq!(req.protocol, "TCP");
        assert_eq!(req.internal_client, "192.168.1.100");
    } else {
        panic!("Expected AddPortMapping request body");
    }
}

#[tokio::test]
async fn test_clear_received_requests() {
    let server = MockIgdServer::start().await.unwrap();

    server
        .mock(Action::any(), Responder::success())
        .await;

    // Send a request
    let _ = soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await;

    assert_eq!(server.received_requests().await.len(), 1);

    // Clear requests
    server.clear_received_requests().await;

    assert!(server.received_requests().await.is_empty());

    // New request should be recorded
    let _ = soap_request(
        &server.control_url(),
        "DeletePortMapping",
        r#"<u:DeletePortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
            <NewRemoteHost></NewRemoteHost>
            <NewExternalPort>8080</NewExternalPort>
            <NewProtocol>TCP</NewProtocol>
        </u:DeletePortMapping>"#,
    )
    .await;

    let requests = server.received_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action_name, "DeletePortMapping");
}

// =============================================================================
// SSDP request recording tests
// =============================================================================

/// Helper to send an SSDP M-SEARCH request.
fn send_msearch_request(target_addr: std::net::SocketAddr, search_target: &str) {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let request = format!(
        "M-SEARCH * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         MAN: \"ssdp:discover\"\r\n\
         MX: 3\r\n\
         ST: {}\r\n\
         \r\n",
        search_target
    );
    socket.send_to(request.as_bytes(), target_addr).unwrap();
}

#[tokio::test]
async fn test_received_ssdp_requests() {
    // Use a high ephemeral port to avoid conflicts with standard SSDP port
    let server = MockIgdServer::builder()
        .ssdp_port(0) // Use random available port
        .start()
        .await;

    // Skip test if SSDP server couldn't start (e.g., permission issues)
    let server = match server {
        Ok(s) if s.ssdp_addr().is_some() => s,
        _ => {
            eprintln!("Skipping SSDP test - could not start SSDP server");
            return;
        }
    };

    let ssdp_addr = server.ssdp_addr().unwrap();

    // Initially no SSDP requests
    let requests = server.received_ssdp_requests().await;
    assert!(requests.is_empty());

    // Send an M-SEARCH request
    send_msearch_request(ssdp_addr, "ssdp:all");

    // Give the server time to receive and process the request
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Should have one SSDP request
    let requests = server.received_ssdp_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].search_target, "ssdp:all");
    assert_eq!(requests[0].man, "ssdp:discover");
    assert_eq!(requests[0].mx, Some(3));
}

#[tokio::test]
async fn test_received_ssdp_requests_multiple() {
    let server = MockIgdServer::builder()
        .ssdp_port(0)
        .start()
        .await;

    let server = match server {
        Ok(s) if s.ssdp_addr().is_some() => s,
        _ => {
            eprintln!("Skipping SSDP test - could not start SSDP server");
            return;
        }
    };

    let ssdp_addr = server.ssdp_addr().unwrap();

    // Send multiple M-SEARCH requests with different ST values
    send_msearch_request(ssdp_addr, "ssdp:all");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    send_msearch_request(ssdp_addr, "urn:schemas-upnp-org:device:InternetGatewayDevice:1");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let requests = server.received_ssdp_requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].search_target, "ssdp:all");
    assert_eq!(
        requests[1].search_target,
        "urn:schemas-upnp-org:device:InternetGatewayDevice:1"
    );
}

#[tokio::test]
async fn test_clear_received_ssdp_requests() {
    let server = MockIgdServer::builder()
        .ssdp_port(0)
        .start()
        .await;

    let server = match server {
        Ok(s) if s.ssdp_addr().is_some() => s,
        _ => {
            eprintln!("Skipping SSDP test - could not start SSDP server");
            return;
        }
    };

    let ssdp_addr = server.ssdp_addr().unwrap();

    // Send a request
    send_msearch_request(ssdp_addr, "upnp:rootdevice");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert_eq!(server.received_ssdp_requests().await.len(), 1);

    // Clear requests
    server.clear_received_ssdp_requests().await;

    assert!(server.received_ssdp_requests().await.is_empty());

    // New request should be recorded
    send_msearch_request(ssdp_addr, "urn:schemas-upnp-org:service:WANIPConnection:1");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let requests = server.received_ssdp_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].search_target,
        "urn:schemas-upnp-org:service:WANIPConnection:1"
    );
}

#[tokio::test]
async fn test_ssdp_request_contains_raw_data() {
    let server = MockIgdServer::builder()
        .ssdp_port(0)
        .start()
        .await;

    let server = match server {
        Ok(s) if s.ssdp_addr().is_some() => s,
        _ => {
            eprintln!("Skipping SSDP test - could not start SSDP server");
            return;
        }
    };

    let ssdp_addr = server.ssdp_addr().unwrap();

    send_msearch_request(ssdp_addr, "ssdp:all");
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let requests = server.received_ssdp_requests().await;
    assert_eq!(requests.len(), 1);

    // Verify raw request contains expected content
    let raw = &requests[0].raw;
    assert!(raw.starts_with("M-SEARCH"));
    assert!(raw.contains("MAN: \"ssdp:discover\""));
    assert!(raw.contains("ST: ssdp:all"));
    assert!(raw.contains("MX: 3"));

    // Verify source address is set
    assert!(!requests[0].source.ip().is_unspecified());

    // Verify timestamp is reasonable
    assert!(requests[0].timestamp.as_secs() < 10);
}
