# mock-igd

A mock UPnP Internet Gateway Device (IGD) server for testing client implementations.

## Features

- SSDP discovery response (M-SEARCH)
- SOAP action handling (GetExternalIPAddress, AddPortMapping, etc.)
- IGD v1 (InternetGatewayDevice:1) and IGD v2 (InternetGatewayDevice:2) emulation
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

## IGD v2

By default the server emulates IGD v1 (`InternetGatewayDevice:1` with
`WANIPConnection:1`). Use the builder to emulate IGD v2, which advertises
`InternetGatewayDevice:2` / `WANIPConnection:2` in SSDP responses and the
device description, and adds the v2-only actions `AddAnyPortMapping`,
`DeletePortMappingRange` and `GetListOfPortMappings`.

Like a real IGD v2 router, the v2 server stays backward compatible with
v1 clients: SSDP searches for `InternetGatewayDevice:1` /
`WANIPConnection:1` are answered with the searched (v1) ST, and SOAP
requests using the `WANIPConnection:1` service type are accepted, with
the response namespace echoing the request. Conversely, a v1 server does
not answer searches for version 2.

```rust
use mock_igd::{MockIgdServer, IgdVersion, Action, Protocol, Responder};

#[tokio::test]
async fn test_igd_v2() {
    let server = MockIgdServer::builder()
        .igd_version(IgdVersion::V2)
        .start()
        .await
        .unwrap();

    // AddAnyPortMapping returns the reserved external port
    server.mock(
        Action::add_any_port_mapping()
            .with_external_port(8080)
            .with_protocol(Protocol::TCP),
        Responder::success().with_reserved_port(8081)
    ).await;

    // DeletePortMappingRange
    server.mock(
        Action::delete_port_mapping_range()
            .with_start_port(8000)
            .with_end_port(8100),
        Responder::success()
    ).await;

    // GetListOfPortMappings returns a port listing generated from the
    // port mapping fields (or set a raw one with `with_port_listing`)
    server.mock(
        Action::get_list_of_port_mappings(),
        Responder::success()
            .with_external_port(8080)
            .with_protocol("TCP")
            .with_internal_client("192.168.1.100")
    ).await;
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
