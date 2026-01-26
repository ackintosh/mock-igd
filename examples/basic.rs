//! Basic example demonstrating mock-igd usage.
//!
//! Run with: cargo run --example basic

use mock_igd::{Action, MockIgdServer, Protocol, Responder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the mock IGD server
    let server = MockIgdServer::start().await?;

    println!("Mock IGD server started!");
    println!("  Root URL: {}", server.url());
    println!("  Control URL: {}", server.control_url());
    println!();

    // Register mock behaviors
    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("203.0.113.42".parse().unwrap()),
        )
        .await;

    server
        .mock(
            Action::add_port_mapping()
                .with_external_port(8080)
                .with_protocol(Protocol::TCP),
            Responder::success(),
        )
        .await;

    server
        .mock(
            Action::add_port_mapping().with_external_port(80),
            Responder::error(718, "ConflictInMappingEntry"),
        )
        .await;

    println!("Registered mocks:");
    println!("  - GetExternalIPAddress -> 203.0.113.42");
    println!("  - AddPortMapping(8080/TCP) -> Success");
    println!("  - AddPortMapping(80) -> Error 718");
    println!();

    // Test: GetExternalIPAddress
    println!("=== Test: GetExternalIPAddress ===");
    let response = send_soap_request(
        &server.control_url(),
        "GetExternalIPAddress",
        r#"<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
        </u:GetExternalIPAddress>"#,
    )
    .await?;
    println!("Response:\n{}\n", response);

    // Test: AddPortMapping (success case)
    println!("=== Test: AddPortMapping 8080/TCP (success) ===");
    let response = send_soap_request(
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
    .await?;
    println!("Response:\n{}\n", response);

    // Test: AddPortMapping (error case)
    println!("=== Test: AddPortMapping 80/TCP (error) ===");
    let response = send_soap_request(
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
    .await?;
    println!("Response:\n{}\n", response);

    Ok(())
}

async fn send_soap_request(
    url: &str,
    action: &str,
    body: &str,
) -> Result<String, Box<dyn std::error::Error>> {
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
            format!("\"urn:schemas-upnp-org:service:WANIPConnection:1#{}\"", action),
        )
        .body(soap_body)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    Ok(format!("[Status: {}]\n{}", status, text))
}
