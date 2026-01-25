//! SSDP (Simple Service Discovery Protocol) server implementation.

use crate::Result;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

/// SSDP multicast address.
const SSDP_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// Start the SSDP server for device discovery.
pub async fn start_ssdp_server(http_addr: SocketAddr, port: u16) -> Result<SocketAddr> {
    let socket = create_multicast_socket(port)?;
    let socket = UdpSocket::from_std(socket.into())?;
    let local_addr = socket.local_addr()?;

    tokio::spawn(async move {
        run_ssdp_server(socket, http_addr).await;
    });

    Ok(local_addr)
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
async fn run_ssdp_server(socket: UdpSocket, http_addr: SocketAddr) {
    let mut buf = [0u8; 2048];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let request = String::from_utf8_lossy(&buf[..len]);
                if is_msearch_request(&request) {
                    if let Err(e) = send_msearch_response(&socket, src, http_addr).await {
                        tracing::warn!("Failed to send M-SEARCH response: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("SSDP receive error: {}", e);
            }
        }
    }
}

/// Check if the request is an M-SEARCH request for IGD.
fn is_msearch_request(request: &str) -> bool {
    request.starts_with("M-SEARCH")
        && (request.contains("ssdp:all")
            || request.contains("upnp:rootdevice")
            || request.contains("urn:schemas-upnp-org:device:InternetGatewayDevice")
            || request.contains("urn:schemas-upnp-org:service:WANIPConnection"))
}

/// Send M-SEARCH response.
async fn send_msearch_response(
    socket: &UdpSocket,
    dest: SocketAddr,
    http_addr: SocketAddr,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\
         USN: uuid:mock-igd-001::urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\
         EXT:\r\n\
         SERVER: mock-igd/0.1 UPnP/1.0\r\n\
         LOCATION: http://{}/rootDesc.xml\r\n\
         \r\n",
        http_addr
    );

    socket.send_to(response.as_bytes(), dest).await?;
    Ok(())
}
