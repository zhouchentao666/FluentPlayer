use futures_util::{future::join_all, stream, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::{watch, Mutex};
use tokio::time::{timeout_at, Instant};

pub(crate) const MEDIA_RENDERER_TARGET: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";

#[derive(Debug, Clone, Serialize)]
pub struct DlnaDevice {
    pub usn: String,
    pub name: String,
    pub location: String,
    pub address: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ControlService {
    pub url: reqwest::Url,
    pub service_type: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedDeviceDescription {
    pub name: String,
    pub av_transport: ControlService,
    pub rendering_control: ControlService,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct PositionInfo {
    pub position: f64,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedDevice {
    pub public: DlnaDevice,
    pub av_transport: ControlService,
    pub rendering_control: ControlService,
    pub device_ip: Ipv4Addr,
}

#[derive(Debug, Clone)]
pub(crate) struct SsdpResponse {
    pub usn: String,
    pub location: String,
    pub sender: SocketAddr,
}

const SSDP_TARGET: &str = "239.255.255.250:1900";
const SEARCH_ATTEMPTS: usize = 3;
const SEARCH_WINDOW: Duration = Duration::from_secs(3);
const MAX_SSDP_PACKET_BYTES: usize = 8 * 1024;
const MAX_SSDP_RESPONSES: usize = 64;

pub(crate) async fn search_responses(
    generation: u64,
    current: &AtomicU64,
    cancellation: &mut watch::Receiver<u64>,
) -> Result<HashMap<String, SsdpResponse>, String> {
    let sockets = bind_discovery_sockets().await?;
    let request = build_search_request();
    let deadline = Instant::now() + SEARCH_WINDOW;
    let mut sent = false;
    let mut send_errors = Vec::new();

    for _ in 0..SEARCH_ATTEMPTS {
        if current.load(Ordering::Acquire) != generation {
            return Err("DLNA 搜索已取消".to_string());
        }
        let sends =
            join_all(sockets.iter().map(|socket| {
                timeout_at(deadline, socket.send_to(request.as_bytes(), SSDP_TARGET))
            }));
        let results = tokio::select! {
            _ = cancellation.changed() => return Err("DLNA 搜索已取消".to_string()),
            results = sends => results,
        };
        for result in results {
            match result {
                Ok(Ok(_)) => sent = true,
                Ok(Err(error)) => send_errors.push(format!("发送 DLNA M-SEARCH 失败: {error}")),
                Err(_) => send_errors.push("发送 DLNA M-SEARCH 超时".to_string()),
            }
        }
        if Instant::now() < deadline {
            tokio::select! {
                _ = cancellation.changed() => return Err("DLNA 搜索已取消".to_string()),
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
        }
    }
    if !sent {
        return Err(format!(
            "DLNA M-SEARCH 未成功发送: {}",
            send_errors.join("；")
        ));
    }
    let socket_count = sockets.len();
    let responses = Arc::new(Mutex::new(HashMap::new()));
    let collector_cancellations = (0..socket_count)
        .map(|_| cancellation.clone())
        .collect::<Vec<_>>();
    let mut collectors = stream::iter(sockets.into_iter().zip(collector_cancellations).map(
        |(socket, mut socket_cancellation)| {
            let responses = responses.clone();
            async move {
                collect_ssdp_responses(
                    socket,
                    generation,
                    current,
                    &mut socket_cancellation,
                    deadline,
                    responses,
                )
                .await
            }
        },
    ))
    .buffer_unordered(socket_count);
    while let Some(result) = tokio::select! {
        _ = cancellation.changed() => return Err("DLNA 搜索已取消".to_string()),
        result = timeout_at(deadline, collectors.next()) => result.ok().flatten(),
    } {
        result?;
    }
    if current.load(Ordering::Acquire) != generation {
        return Err("DLNA 搜索已取消".to_string());
    }
    let discovered = responses.lock().await.clone();
    Ok(discovered)
}

async fn bind_discovery_sockets() -> Result<Vec<UdpSocket>, String> {
    let mut sockets = Vec::new();
    for address in local_ipv4_interfaces()? {
        sockets.push(bind_discovery_socket(address).await?);
    }
    Ok(sockets)
}

#[cfg(all(unix, not(target_os = "android")))]
async fn bind_discovery_socket(address: Ipv4Addr) -> Result<UdpSocket, String> {
    let socket = std::net::UdpSocket::bind((address, 0))
        .map_err(|error| format!("绑定 {address} 失败: {error}"))?;
    configure_multicast_interface(&socket, address)?;
    socket
        .set_nonblocking(true)
        .map_err(|error| format!("设置 {address} 发现套接字为非阻塞失败: {error}"))?;
    UdpSocket::from_std(socket).map_err(|error| format!("接管 {address} 发现套接字失败: {error}"))
}

#[cfg(all(unix, not(target_os = "android")))]
fn configure_multicast_interface(
    socket: &std::net::UdpSocket,
    interface: Ipv4Addr,
) -> Result<(), String> {
    use std::os::fd::AsRawFd;

    let address = libc::in_addr {
        s_addr: u32::from_ne_bytes(interface.octets()),
    };
    let result = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::IPPROTO_IP,
            libc::IP_MULTICAST_IF,
            &address as *const libc::in_addr as *const libc::c_void,
            std::mem::size_of::<libc::in_addr>() as libc::socklen_t,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "设置 DLNA 多播接口 {interface} 失败: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(all(test, unix, not(target_os = "android")))]
fn multicast_interface(socket: &std::net::UdpSocket) -> Result<Ipv4Addr, String> {
    use std::os::fd::AsRawFd;

    let mut address = libc::in_addr { s_addr: 0 };
    let mut length = std::mem::size_of::<libc::in_addr>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::IPPROTO_IP,
            libc::IP_MULTICAST_IF,
            &mut address as *mut libc::in_addr as *mut libc::c_void,
            &mut length,
        )
    };
    if result == 0 && length == std::mem::size_of::<libc::in_addr>() as libc::socklen_t {
        Ok(Ipv4Addr::from(address.s_addr.to_ne_bytes()))
    } else if result == 0 {
        Err("读取 DLNA 多播接口返回了意外长度".to_string())
    } else {
        Err(format!(
            "读取 DLNA 多播接口失败: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(any(not(unix), target_os = "android"))]
async fn bind_discovery_socket(address: Ipv4Addr) -> Result<UdpSocket, String> {
    UdpSocket::bind((address, 0))
        .await
        .map_err(|error| format!("绑定 {address} 失败: {error}"))
}

#[cfg(all(unix, not(target_os = "android")))]
fn local_ipv4_interfaces() -> Result<Vec<Ipv4Addr>, String> {
    use std::collections::HashSet;
    use std::ptr;

    let mut addresses = ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut addresses) } != 0 {
        return Err(format!(
            "枚举本地 IPv4 接口失败: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut interfaces = HashSet::new();
    let mut current = addresses;
    while !current.is_null() {
        let interface = unsafe { &*current };
        if !interface.ifa_addr.is_null()
            && unsafe { (*interface.ifa_addr).sa_family as i32 } == libc::AF_INET
            && interface.ifa_flags & libc::IFF_UP as u32 != 0
            && interface.ifa_flags & libc::IFF_LOOPBACK as u32 == 0
        {
            let address = unsafe { &*(interface.ifa_addr as *const libc::sockaddr_in) };
            let address = Ipv4Addr::from(u32::from_be(address.sin_addr.s_addr));
            if is_local_unicast_ipv4(address) {
                interfaces.insert(address);
            }
        }
        current = interface.ifa_next;
    }
    unsafe { libc::freeifaddrs(addresses) };
    if interfaces.is_empty() {
        return Err("未找到可用的私有 IPv4 网络接口".to_string());
    }
    Ok(interfaces.into_iter().collect())
}

#[cfg(any(not(unix), target_os = "android"))]
fn local_ipv4_interfaces() -> Result<Vec<Ipv4Addr>, String> {
    Ok(vec![Ipv4Addr::UNSPECIFIED])
}

async fn collect_ssdp_responses(
    socket: UdpSocket,
    generation: u64,
    current: &AtomicU64,
    cancellation: &mut watch::Receiver<u64>,
    deadline: Instant,
    responses: Arc<Mutex<HashMap<String, SsdpResponse>>>,
) -> Result<(), String> {
    let mut buffer = [0; MAX_SSDP_PACKET_BYTES];
    while current.load(Ordering::Acquire) == generation
        && responses.lock().await.len() < MAX_SSDP_RESPONSES
    {
        let received = tokio::select! {
            _ = cancellation.changed() => return Err("DLNA 搜索已取消".to_string()),
            result = timeout_at(deadline, socket.recv_from(&mut buffer)) => result,
        };
        let Ok(Ok((size, sender))) = received else {
            break;
        };
        if let Some(response) = parse_ssdp_response(&buffer[..size], sender) {
            let mut responses = responses.lock().await;
            if responses.len() < MAX_SSDP_RESPONSES {
                responses.entry(response.usn.clone()).or_insert(response);
            }
        }
    }
    if current.load(Ordering::Acquire) != generation {
        return Err("DLNA 搜索已取消".to_string());
    }
    Ok(())
}

fn build_search_request() -> &'static str {
    "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n"
}

fn parse_ssdp_response(packet: &[u8], sender: SocketAddr) -> Option<SsdpResponse> {
    let packet = std::str::from_utf8(packet).ok()?;
    let mut status = packet.lines().next()?.split_whitespace();
    if !matches!(status.next()?, "HTTP/1.0" | "HTTP/1.1") || status.next()? != "200" {
        return None;
    }
    let headers = packet
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect::<HashMap<_, _>>();
    let location = headers.get("location")?;
    let usn = headers.get("usn")?.to_string();
    let target_suffix = format!("::{}", MEDIA_RENDERER_TARGET.to_ascii_lowercase());
    if !headers
        .get("st")?
        .eq_ignore_ascii_case(MEDIA_RENDERER_TARGET)
        || !usn.starts_with("uuid:")
        || !usn.to_ascii_lowercase().ends_with(&target_suffix)
        || !location_matches_sender(location, sender.ip())
    {
        return None;
    }
    Some(SsdpResponse {
        usn,
        location: location.to_string(),
        sender,
    })
}

pub(crate) fn is_local_unicast_ipv4(address: Ipv4Addr) -> bool {
    address.is_private()
        && !address.is_loopback()
        && !address.is_link_local()
        && !address.is_multicast()
        && !address.is_broadcast()
        && !address.is_unspecified()
}

fn location_matches_sender(location: &str, sender: IpAddr) -> bool {
    let Ok(url) = reqwest::Url::parse(location) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    let (IpAddr::V4(sender), Some(host)) = (sender, url.host_str()) else {
        return false;
    };
    host.parse::<Ipv4Addr>().ok() == Some(sender) && is_local_unicast_ipv4(sender)
}

#[cfg(test)]
mod tests {
    use super::parse_ssdp_response;
    use std::net::SocketAddr;

    #[cfg(all(unix, not(target_os = "android")))]
    use super::{configure_multicast_interface, multicast_interface};

    #[cfg(all(unix, not(target_os = "android")))]
    #[test]
    fn pins_multicast_socket_to_requested_interface() {
        let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        configure_multicast_interface(&socket, std::net::Ipv4Addr::LOCALHOST).unwrap();
        assert_eq!(
            multicast_interface(&socket).unwrap(),
            std::net::Ipv4Addr::LOCALHOST
        );
    }

    #[test]
    fn parses_case_insensitive_ssdp_headers() {
        let response = parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nLoCaTiOn: http://192.168.1.20/device.xml\r\nSt: urn:schemas-upnp-org:device:MediaRenderer:1\r\nUsN: uuid:renderer::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n",
            "192.168.1.20:1900".parse::<SocketAddr>().unwrap(),
        )
        .unwrap();
        assert_eq!(
            response.usn,
            "uuid:renderer::urn:schemas-upnp-org:device:MediaRenderer:1"
        );
    }

    #[test]
    fn accepts_http_1_0_ssdp_success_responses() {
        let response = parse_ssdp_response(
            b"HTTP/1.0 200 OK\r\nLOCATION: http://192.168.1.20/device.xml\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\nUSN: uuid:renderer::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n",
            "192.168.1.20:1900".parse::<SocketAddr>().unwrap(),
        );

        assert!(response.is_some());
    }

    #[test]
    fn rejects_spoofed_ssdp_responses() {
        let sender = "192.168.1.20:1900".parse::<SocketAddr>().unwrap();
        assert!(parse_ssdp_response(
            b"HTTP/1.1 404 Not Found\r\nLOCATION: http://192.168.1.20/device.xml\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\nUSN: uuid:r::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n",
            sender,
        ).is_none());
        assert!(parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nLOCATION: http://8.8.8.8/device.xml\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\nUSN: uuid:r::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n",
            sender,
        ).is_none());
        assert!(parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nLOCATION: http://192.168.1.20/device.xml\r\nST: upnp:rootdevice\r\nUSN: uuid:r::upnp:rootdevice\r\n\r\n",
            sender,
        ).is_none());
    }
}
