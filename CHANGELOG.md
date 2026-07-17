# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-07-17

### Added

- IGD v2 (`InternetGatewayDevice:2`) support via
  `MockIgdServer::builder().igd_version(IgdVersion::V2)`:
  - SSDP responses advertise `InternetGatewayDevice:2` and `UPnP/1.1`
  - Device description advertises `InternetGatewayDevice:2`, `WANDevice:2`,
    `WANConnectionDevice:2` and `WANIPConnection:2`
  - WANIPConnection SCPD lists the v2-only actions and state variables
- New WANIPConnection:2 actions with matchers and responders:
  - `AddAnyPortMapping` (`Action::add_any_port_mapping()`,
    `Responder::success().with_reserved_port(..)`)
  - `DeletePortMappingRange` (`Action::delete_port_mapping_range()`)
  - `GetListOfPortMappings` (`Action::get_list_of_port_mappings()`,
    `Responder::success().with_port_listing(..)` or a listing generated
    from the port mapping fields)
- `MockIgdServer::igd_version()` getter and `IgdVersion` re-export
- IGD v2 servers are backward compatible with v1 clients, like real
  dual-version routers: SSDP M-SEARCH responses echo the searched ST
  (a search for `InternetGatewayDevice:1` / `WANIPConnection:1` is
  answered with the v1 ST), and SOAP requests with the
  `WANIPConnection:1` service type are accepted. A v1 server no longer
  answers SSDP searches for version 2 targets it does not support

### Changed

- SOAP success responses now echo the service type from the request's
  SOAPACTION header in the response namespace (previously hardcoded to
  `WANIPConnection:1` / `WANCommonInterfaceConfig:1`), so v2 clients
  receive a matching namespace

### Fixed

- `GetSpecificPortMappingEntry` success responses now use the
  `GetSpecificPortMappingEntryResponse` element (previously
  `GetGenericPortMappingEntryResponse`)

## [0.2.0] - 2026-06-13

### Fixed

- `MockIgdServer::ssdp_addr()` now returns a loopback address (`127.0.0.1`)
  instead of the unspecified address (`0.0.0.0`) when the SSDP socket is bound
  to all interfaces, so the returned address can be used directly as a discovery
  destination by clients. The actual (possibly ephemeral) port is preserved.

### Changed

- Bumped minimum supported Rust version (MSRV) to 1.88.0
- Migrated to Rust edition 2024

## [0.1.0] - 2025-01-25

### Added

- Initial release of mock-igd
- **HTTP Server**
  - Device description endpoint (`/rootDesc.xml`)
  - SCPD endpoints (`/WANIPCn.xml`, `/WANCommonIFC1.xml`)
  - SOAP control endpoints (`/ctl/IPConn`, `/ctl/WANCommonIFC1`)
- **SSDP Server**
  - M-SEARCH request handling
  - Configurable port
- **WANIPConnection Actions**
  - `GetExternalIPAddress`
  - `GetStatusInfo`
  - `AddPortMapping`
  - `DeletePortMapping`
  - `GetGenericPortMappingEntry`
  - `GetSpecificPortMappingEntry`
- **WANCommonInterfaceConfig Actions**
  - `GetCommonLinkProperties`
  - `GetTotalBytesReceived`
  - `GetTotalBytesSent`
- **Mock Configuration**
  - Flexible Matcher + Responder pattern
  - Priority-based mock matching
  - Limited-use mocks (times)
  - Success and error responses
  - Custom raw responses
- **Request Recording**
  - Record and verify SOAP requests
  - Record and verify SSDP M-SEARCH requests
  - Clear recorded requests

[0.3.0]: https://github.com/ackintosh/mock-igd/releases/tag/v0.3.0
[0.2.0]: https://github.com/ackintosh/mock-igd/releases/tag/v0.2.0
[0.1.0]: https://github.com/ackintosh/mock-igd/releases/tag/v0.1.0
