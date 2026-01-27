//! Integration tests for mock-igd server.

use mock_igd::{Action, MockIgdServer, Protocol, Responder};

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
