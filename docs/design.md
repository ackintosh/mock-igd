# mock-igd Design Document

## Overview

A mock server for UPnP IGD (Internet Gateway Device). Used for testing client implementations.

## Architecture

### UPnP IGD Components

1. **SSDP** (UDP 1900) - Device discovery protocol
2. **Device Description** (HTTP GET) - XML device information
3. **SOAP Actions** (HTTP POST) - Port mapping and other operations

### Module Structure

```
src/
├── lib.rs              # Public API
├── server/
│   ├── mod.rs
│   ├── ssdp.rs         # SSDP response (M-SEARCH)
│   └── http.rs         # HTTP/SOAP server
├── action/
│   ├── mod.rs
│   ├── types.rs        # UPnP IGD action definitions
│   └── parser.rs       # SOAP request parser
├── matcher/
│   ├── mod.rs
│   └── builder.rs      # Request matching conditions
├── responder/
│   ├── mod.rs
│   ├── builder.rs      # Response generation
│   └── templates.rs    # XML/SOAP templates
└── mock.rs             # Mock registration and management
```

## Behavior Definition (Matcher + Responder Pattern)

Design inspired by wiremock-rs. Flexibility is achieved by separating request matching conditions from responses.

### Basic Usage

```rust
use mock_igd::{MockIgdServer, Action, Responder};

#[tokio::test]
async fn test_get_external_ip() {
    let server = MockIgdServer::start().await;

    // Define behavior
    server.mock(
        Action::GetExternalIPAddress,
        Responder::success()
            .with_external_ip("203.0.113.1")
    ).await;

    // Run the client under test
    let client = IgdClient::new(server.url());
    let ip = client.get_external_ip().await.unwrap();
    assert_eq!(ip, "203.0.113.1".parse().unwrap());
}
```

### Conditional Matching

```rust
// Match specific parameters
server.mock(
    Action::AddPortMapping
        .with_external_port(8080)
        .with_protocol("TCP"),
    Responder::success()
).await;

// Match any request
server.mock(
    Action::any(),
    Responder::error(501, "ActionNotImplemented")
).await;
```

### Error Responses

```rust
// UPnP standard error codes
server.mock(
    Action::AddPortMapping.with_external_port(80),
    Responder::error(718, "ConflictInMappingEntry")
).await;
```

### Custom Responses

```rust
// Return a fully custom response
server.mock(
    Action::GetExternalIPAddress,
    Responder::custom(|_request| {
        // Arbitrary logic
        HttpResponse::Ok()
            .content_type("text/xml")
            .body(custom_soap_xml)
    })
).await;
```

## Core Type Definitions

### Action (UPnP IGD Actions)

```rust
pub enum Action {
    // WANIPConnection
    GetExternalIPAddress,
    AddPortMapping(AddPortMappingMatcher),
    DeletePortMapping(DeletePortMappingMatcher),
    GetGenericPortMappingEntry(GetGenericPortMappingEntryMatcher),
    GetSpecificPortMappingEntry(GetSpecificPortMappingEntryMatcher),

    // WANCommonInterfaceConfig
    GetCommonLinkProperties,
    GetTotalBytesReceived,
    GetTotalBytesSent,

    // Match any action
    Any,
}
```

### Responder

```rust
pub struct Responder {
    kind: ResponderKind,
}

enum ResponderKind {
    Success(SuccessResponse),
    Error { code: u16, description: String },
    Custom(Box<dyn Fn(&SoapRequest) -> HttpResponse + Send + Sync>),
}
```

### Mock

```rust
pub struct Mock {
    action: Action,
    responder: Responder,
    priority: u32,        // Higher = checked first
    times: Option<u32>,   // Max match count (None = unlimited)
}
```

## Matching Priority

1. Only Mocks with remaining `times` are considered
2. Ordered by `priority` (highest first)
3. For equal priority, registration order (last wins)
4. First matching Mock's Responder is used
5. If no match, return 404 or 501 error

## Future Extensions (Phase 2+)

### Stateful Mode

Maintains an internal port mapping table and behaves like a real IGD.

```rust
let server = MockIgdServer::start()
    .with_stateful_behavior()
    .with_external_ip("203.0.113.1")
    .await;

// Entries added via AddPortMapping can be retrieved
// via GetGenericPortMappingEntry
```

### Record & Replay Mode

```rust
// Record
let server = MockIgdServer::start()
    .record_to("fixtures/session.json")
    .await;

// Replay
let server = MockIgdServer::from_recording("fixtures/session.json").await;
```

### Configuration File Loading

```rust
let server = MockIgdServer::from_config("mock-config.yaml").await;
```

## Dependencies

- `tokio` - Async runtime
- `axum` - HTTP server
- `quick-xml` - XML parsing and generation
- `socket2` - UDP socket (for SSDP)
