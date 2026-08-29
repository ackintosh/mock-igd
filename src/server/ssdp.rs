//! SSDP (Simple Service Discovery Protocol) server implementation.

use super::{DeviceConfig, IgdVersion};
use crate::Result;
use crate::mock::{MockRegistry, ReceivedSsdpRequest};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;

/// SSDP multicast address.
const SSDP_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// Start the SSDP server for device discovery.
pub async fn start_ssdp_server(
    http_addr: SocketAddr,
    port: u16,
    config: DeviceConfig,
    registry: Arc<MockRegistry>,
) -> Result<SocketAddr> {
    let socket = create_multicast_socket(port)?;
    let socket = UdpSocket::from_std(socket.into())?;
    let local_addr = socket.local_addr()?;

    // The socket is bound to 0.0.0.0 (UNSPECIFIED), so `local_addr` returns an
    // unspecified IP that can't be used as a destination by clients. Replace the
    // IP with loopback so callers can send discovery requests directly, while
    // keeping the actual (possibly ephemeral) port.
    let advertised_addr = match local_addr {
        SocketAddr::V4(addr) if addr.ip().is_unspecified() => {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, addr.port()))
        }
        other => other,
    };

    tokio::spawn(async move {
        run_ssdp_server(socket, http_addr, config, registry).await;
    });

    Ok(advertised_addr)
}

/// Create a UDP socket for SSDP multicast.
fn create_multicast_socket(port: u16) -> Result<Socket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;

    #[cfg(unix)]
    socket.set_reuse_port(true)?;

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    socket.bind(&addr.into())?;

    socket.join_multicast_v4(&SSDP_MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(true)?;

    Ok(socket)
}

/// Run the SSDP server loop.
async fn run_ssdp_server(
    socket: UdpSocket,
    http_addr: SocketAddr,
    config: DeviceConfig,
    registry: Arc<MockRegistry>,
) {
    let mut buf = [0u8; 2048];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let request = String::from_utf8_lossy(&buf[..len]).to_string();
                if is_msearch_request(&request) {
                    // Record the request
                    let received = parse_ssdp_request(&request, src, registry.start_time());
                    let st = response_search_target(&received.search_target, config);
                    registry.record_ssdp_request(received).await;

                    // Only respond when the searched version is one we emulate.
                    if let Some(st) = st {
                        if let Err(e) =
                            send_msearch_response(&socket, src, http_addr, config.igd_version, &st)
                                .await
                        {
                            tracing::warn!("Failed to send M-SEARCH response: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("SSDP receive error: {}", e);
            }
        }
    }
}

/// Parse an SSDP M-SEARCH request into a structured format.
fn parse_ssdp_request(
    request: &str,
    source: SocketAddr,
    start_time: std::time::Instant,
) -> ReceivedSsdpRequest {
    let search_target = extract_header(request, "ST").unwrap_or_default();
    let man = extract_header(request, "MAN").unwrap_or_default();
    let mx = extract_header(request, "MX").and_then(|s| s.parse().ok());

    ReceivedSsdpRequest {
        source,
        search_target,
        man,
        mx,
        raw: request.to_string(),
        timestamp: start_time.elapsed(),
    }
}

/// Extract a header value from an SSDP request.
fn extract_header(request: &str, header: &str) -> Option<String> {
    for line in request.lines() {
        let line = line.trim();
        if line
            .to_uppercase()
            .starts_with(&format!("{}:", header.to_uppercase()))
        {
            let value = line[header.len() + 1..].trim();
            // Remove surrounding quotes if present
            let value = value.trim_matches('"');
            return Some(value.to_string());
        }
    }
    None
}

/// Search target prefix of the WANIPConnection service.
const WAN_IP_CONNECTION_PREFIX: &str = "urn:schemas-upnp-org:service:WANIPConnection";

/// Search target prefix of the WANPPPConnection service.
const WAN_PPP_CONNECTION_PREFIX: &str = "urn:schemas-upnp-org:service:WANPPPConnection";

/// Search target prefix of the InternetGatewayDevice device type.
const IGD_DEVICE_PREFIX: &str = "urn:schemas-upnp-org:device:InternetGatewayDevice";

/// Check if the request is an M-SEARCH request for IGD.
fn is_msearch_request(request: &str) -> bool {
    request.starts_with("M-SEARCH")
        && (request.contains("ssdp:all")
            || request.contains("upnp:rootdevice")
            || request.contains(IGD_DEVICE_PREFIX)
            || request.contains(WAN_IP_CONNECTION_PREFIX)
            || request.contains(WAN_PPP_CONNECTION_PREFIX))
}

/// Determine the ST value for an M-SEARCH response.
///
/// Like real IGD devices, the response echoes the search target when it
/// names a device/service version this server supports: an IGD v2 device
/// is backward compatible and answers searches for version 1 with a
/// version 1 ST. Returns `None` when the searched version is higher than
/// the emulated one, or when the searched connection service is not
/// exposed by this server, in which case no response must be sent.
fn response_search_target(search_target: &str, config: DeviceConfig) -> Option<String> {
    if let Some(rest) = search_target.strip_prefix(WAN_PPP_CONNECTION_PREFIX) {
        // WANPPPConnection only exists in version 1.
        let requested: u8 = rest.strip_prefix(':').and_then(|v| v.parse().ok())?;
        if config.connection_service.has_ppp() && requested == 1 {
            return Some(search_target.to_string());
        }
        return None;
    }

    if let Some(rest) = search_target.strip_prefix(WAN_IP_CONNECTION_PREFIX) {
        let requested: u8 = rest.strip_prefix(':').and_then(|v| v.parse().ok())?;
        if config.connection_service.has_ip() && requested <= config.igd_version.number() {
            return Some(search_target.to_string());
        }
        return None;
    }

    if let Some(rest) = search_target.strip_prefix(IGD_DEVICE_PREFIX) {
        let requested: u8 = rest.strip_prefix(':').and_then(|v| v.parse().ok())?;
        if requested <= config.igd_version.number() {
            return Some(search_target.to_string());
        }
        return None;
    }

    // ssdp:all, upnp:rootdevice, etc.: advertise the device's own version.
    Some(format!(
        "{IGD_DEVICE_PREFIX}:{}",
        config.igd_version.number()
    ))
}

/// Send M-SEARCH response.
async fn send_msearch_response(
    socket: &UdpSocket,
    dest: SocketAddr,
    http_addr: SocketAddr,
    igd_version: IgdVersion,
    st: &str,
) -> Result<()> {
    let upnp_version = match igd_version {
        IgdVersion::V1 => "UPnP/1.0",
        IgdVersion::V2 => "UPnP/1.1",
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         ST: {st}\r\n\
         USN: uuid:mock-igd-001::{st}\r\n\
         EXT:\r\n\
         SERVER: mock-igd/0.1 {upnp_version}\r\n\
         LOCATION: http://{http_addr}/rootDesc.xml\r\n\
         \r\n"
    );

    socket.send_to(response.as_bytes(), dest).await?;
    Ok(())
}
