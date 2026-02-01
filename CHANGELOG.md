# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/ackintosh/mock-igd/releases/tag/v0.1.0
