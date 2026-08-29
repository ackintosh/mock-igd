# mock-igd

A mock UPnP Internet Gateway Device (IGD) server for testing client implementations.

## Features

- SSDP discovery response (M-SEARCH)
- SOAP action handling (GetExternalIPAddress, AddPortMapping, etc.)
- IGD v1 (InternetGatewayDevice:1) and IGD v2 (InternetGatewayDevice:2) emulation
- WANIPConnection and WANPPPConnection connection services
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

## WANPPPConnection

By default the server exposes the `WANIPConnection` service. Use
`connection_service` to emulate a PPP based gateway that advertises
`urn:schemas-upnp-org:service:WANPPPConnection:1` instead, or
`ConnectionService::Both` to advertise both services. `WANPPPConnection`
is only defined in version 1, so it is advertised as version 1 on IGD v1
and IGD v2 devices alike.

A PPP server answers SSDP searches for `WANPPPConnection:1`, advertises
the service in the device description (SCPD `/WANPPPCn.xml`, control URL
`/ctl/PPPConn`) and accepts SOAP requests with the `WANPPPConnection:1`
service type, echoing it in the response namespace. Only the endpoints of
the advertised services are served: a PPP-only server responds with 404
on the `WANIPConnection` endpoints and does not answer SSDP searches for
`WANIPConnection`, and vice versa.

Registered mocks are matched by action, so the same mocks apply to both
connection services.

```rust
use mock_igd::{MockIgdServer, ConnectionService, Action, Responder};

#[tokio::test]
async fn test_ppp_connection() {
    let server = MockIgdServer::builder()
        .connection_service(ConnectionService::Ppp)
        .start()
        .await
        .unwrap();

    // server.control_url() points at /ctl/PPPConn
    server.mock(
        Action::GetExternalIPAddress,
        Responder::success().with_external_ip("203.0.113.1".parse().unwrap())
    ).await;

    // GetLinkLayerMaxBitRates is a WANPPPConnection-only action
    server.mock(
        Action::GetLinkLayerMaxBitRates,
        Responder::success()
            .with_upstream_max_bit_rate(1_000_000)
            .with_downstream_max_bit_rate(8_000_000)
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

## Examples

- `examples/basic.rs` — registers mocks, sends SOAP requests itself and exits:
  `cargo run --example basic`
- `examples/serve.rs` — starts the server and waits for requests, printing each
  SOAP action and SSDP M-SEARCH as it arrives, until Ctrl+C. Useful for pointing
  a real IGD client (or `curl`) at the mock:

  ```console
  $ cargo run --example serve
  $ cargo run --example serve -- --http-port 45678 --ssdp-port 1900 --v2
  ```

  The example prints the root/description/control URLs, the SSDP address and a
  ready-to-paste `curl` command for `GetExternalIPAddress`.

## License

MIT OR Apache-2.0
