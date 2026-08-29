//! Long-running example: start a mock IGD server and wait for requests.
//!
//! Unlike `basic.rs`, which sends its own requests and exits, this example
//! keeps the server running so you can point a real IGD client (or `curl`)
//! at it and watch the requests arrive.
//!
//! Run with: cargo run --example serve
//!
//! Options:
//!   --http-port <PORT>   Bind the HTTP server to a fixed port (default: random)
//!   --ssdp-port <PORT>   Bind the SSDP server to a fixed port (default: 1900)
//!   --v2                 Emulate IGD v2 instead of v1

use mock_igd::{Action, IgdVersion, MockIgdServer, Protocol, Responder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mock_igd=info".into()),
        )
        .init();

    let options = Options::parse(std::env::args().skip(1))?;

    // Start the server and keep it alive until Ctrl+C. Dropping the returned
    // `MockIgdServer` shuts the server down, so it has to stay in scope.
    let mut builder = MockIgdServer::builder()
        .igd_version(options.igd_version)
        .ssdp_port(options.ssdp_port.unwrap_or(1900));
    if let Some(port) = options.http_port {
        builder = builder.http_port(port);
    }
    let server = builder.start().await?;

    register_mocks(&server).await;

    println!("Mock IGD server listening (IGD {:?})", server.igd_version());
    println!("  Root URL:        {}", server.url());
    println!("  Description URL: {}", server.description_url());
    println!("  Control URL:     {}", server.control_url());
    match server.ssdp_addr() {
        Some(addr) => println!("  SSDP:            {}", addr),
        None => println!("  SSDP:            unavailable (failed to bind)"),
    }
    println!();
    println!("Registered mocks:");
    println!("  - GetExternalIPAddress    -> 203.0.113.42");
    println!("  - AddPortMapping(80)      -> Error 718 ConflictInMappingEntry");
    println!("  - AddPortMapping(*)       -> Success");
    println!("  - <any other action>      -> Success");
    println!();
    println!("Try it with curl:");
    println!("  curl -s {} \\", server.control_url());
    println!("    -H 'Content-Type: text/xml; charset=\"utf-8\"' \\");
    println!(
        "    -H 'SOAPAction: \"urn:schemas-upnp-org:service:WANIPConnection:{}#GetExternalIPAddress\"' \\",
        server.igd_version().number()
    );
    println!("    --data-binary @- <<'XML'");
    println!("{}", get_external_ip_envelope(server.igd_version()));
    println!("XML");
    println!();
    println!("Waiting for requests... (Ctrl+C to stop)");

    // Print each request as it arrives, until Ctrl+C.
    let printer = async {
        let mut printed_soap = 0usize;
        let mut printed_ssdp = 0usize;
        loop {
            for request in server
                .received_requests()
                .await
                .into_iter()
                .skip(printed_soap)
            {
                printed_soap += 1;
                println!(
                    "[{:>8.3}s] SOAP  {} ({})\n           {:?}",
                    request.timestamp.as_secs_f64(),
                    request.action_name,
                    request.service_type,
                    request.body,
                );
            }
            for request in server
                .received_ssdp_requests()
                .await
                .into_iter()
                .skip(printed_ssdp)
            {
                printed_ssdp += 1;
                println!(
                    "[{:>8.3}s] SSDP  M-SEARCH from {} (ST: {}, MX: {:?})",
                    request.timestamp.as_secs_f64(),
                    request.source,
                    request.search_target,
                    request.mx,
                );
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };

    tokio::select! {
        _ = printer => {}
        result = tokio::signal::ctrl_c() => {
            result?;
            println!("\nShutting down.");
        }
    }

    server.shutdown();
    Ok(())
}

/// Register the behaviors the server responds with.
///
/// Mocks are checked in priority order (higher first), so the specific
/// AddPortMapping(80) error takes precedence over the catch-all below it.
async fn register_mocks(server: &MockIgdServer) {
    server
        .mock(
            Action::GetExternalIPAddress,
            Responder::success().with_external_ip("203.0.113.42".parse().unwrap()),
        )
        .await;

    server
        .mock_with_priority(
            Action::add_port_mapping().with_external_port(80),
            Responder::error(718, "ConflictInMappingEntry"),
            10,
        )
        .await;

    server
        .mock(
            Action::add_port_mapping().with_protocol(Protocol::TCP),
            Responder::success(),
        )
        .await;

    // Catch-all so any other action still gets a valid response.
    server.mock(Action::any(), Responder::success()).await;
}

fn get_external_ip_envelope(version: IgdVersion) -> String {
    format!(
        r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetExternalIPAddress xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:{}"/>
</s:Body>
</s:Envelope>"#,
        version.number()
    )
}

struct Options {
    http_port: Option<u16>,
    ssdp_port: Option<u16>,
    igd_version: IgdVersion,
}

impl Options {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut options = Options {
            http_port: None,
            ssdp_port: None,
            igd_version: IgdVersion::V1,
        };
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--http-port" => {
                    options.http_port = Some(parse_port(&arg, args.next())?);
                }
                "--ssdp-port" => {
                    options.ssdp_port = Some(parse_port(&arg, args.next())?);
                }
                "--v2" => options.igd_version = IgdVersion::V2,
                other => return Err(format!("unknown argument: {}", other).into()),
            }
        }

        Ok(options)
    }
}

fn parse_port(flag: &str, value: Option<String>) -> Result<u16, Box<dyn std::error::Error>> {
    let value = value.ok_or_else(|| format!("{} requires a port number", flag))?;
    value
        .parse()
        .map_err(|e| format!("invalid port for {}: {}", flag, e).into())
}
