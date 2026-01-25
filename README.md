# mock-igd

A mock UPnP Internet Gateway Device (IGD) server for testing client implementations.

## Features

- SSDP discovery response (M-SEARCH)
- SOAP action handling (GetExternalIPAddress, AddPortMapping, etc.)
- Flexible behavior definition with Matcher + Responder pattern
- Async/await support with Tokio

## Usage

```rust
use mock_igd::{MockIgdServer, Action, Responder};

#[tokio::test]
async fn test_port_mapping() {
    // Start mock server
    let server = MockIgdServer::start().await;

    // Define behaviors
    server.mock(
        Action::GetExternalIPAddress,
        Responder::success()
            .with_external_ip("203.0.113.1")
    ).await;

    server.mock(
        Action::AddPortMapping
            .with_external_port(8080)
            .with_protocol("TCP"),
        Responder::success()
    ).await;

    // Error response
    server.mock(
        Action::AddPortMapping.with_external_port(80),
        Responder::error(718, "ConflictInMappingEntry")
    ).await;

    // Use server.url() to connect your IGD client
    let gateway_url = server.url();
    // ...
}
```

## Documentation

- [Design Document](docs/design.md) - Architecture and API design

## License

MIT OR Apache-2.0
