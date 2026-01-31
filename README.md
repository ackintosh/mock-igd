# mock-igd

A mock UPnP Internet Gateway Device (IGD) server for testing client implementations.

## Features

- SSDP discovery response (M-SEARCH)
- SOAP action handling (GetExternalIPAddress, AddPortMapping, etc.)
- Flexible behavior definition with Matcher + Responder pattern
- Request recording for test verification
- Async/await support with Tokio

## Usage

```rust
use mock_igd::{MockIgdServer, Action, Protocol, Responder};

#[tokio::test]
async fn test_port_mapping() {
    // Start mock server
    let server = MockIgdServer::start().await.unwrap();

    // Define behavior for GetExternalIPAddress
    server.mock(
        Action::GetExternalIPAddress,
        Responder::success()
            .with_external_ip("203.0.113.1".parse().unwrap())
    ).await;

    // Define behavior for AddPortMapping with specific parameters
    server.mock(
        Action::add_port_mapping()
            .with_external_port(8080)
            .with_protocol(Protocol::TCP),
        Responder::success()
    ).await;

    // Error response for port 80
    server.mock(
        Action::add_port_mapping().with_external_port(80),
        Responder::error(718, "ConflictInMappingEntry")
    ).await;

    // Use server.url() to connect your IGD client
    let gateway_url = server.url();
    // ...
}
```

## Verifying Requests

You can verify that your client sent the expected requests:

```rust
use mock_igd::{MockIgdServer, Action, Responder};

#[tokio::test]
async fn test_verify_requests() {
    let server = MockIgdServer::start().await.unwrap();

    server.mock(Action::any(), Responder::success()).await;

    // ... run your client code ...

    // Verify received requests
    let requests = server.received_requests().await;
    assert_eq!(requests[0].action_name, "GetExternalIPAddress");
}
```

## License

MIT OR Apache-2.0
