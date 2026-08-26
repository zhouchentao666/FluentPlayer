use encoding_rs::{Encoding, UTF_8};
use once_cell::sync::Lazy;
use reqwest::{header, Method, StatusCode, Url};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_REDIRECTS: u8 = 10;
const MAX_CONCURRENT_BUFFERED_RESPONSES: usize = 4;
const MAX_IN_FLIGHT_BUFFERED_BYTES: u32 = 160 * 1024 * 1024;

static RESOURCE_GOVERNOR: Lazy<ResourceGovernor> = Lazy::new(|| {
    ResourceGovernor::new(
        MAX_CONCURRENT_BUFFERED_RESPONSES,
        MAX_IN_FLIGHT_BUFFERED_BYTES,
    )
});

#[derive(Clone, Copy)]
pub struct HttpPolicy {
    pub allow_private_networks: bool,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub max_redirects: u8,
}

impl HttpPolicy {
    pub const fn public_proxy() -> Self {
        Self::new(false)
    }

    /// Allow local/private sources only for explicit plugin package downloads.
    pub const fn plugin_download() -> Self {
        Self::new(true)
    }

    pub const fn dlna_local() -> Self {
        Self::new(true)
    }

    const fn new(allow_private_networks: bool) -> Self {
        Self {
            allow_private_networks,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_redirects: DEFAULT_MAX_REDIRECTS,
        }
    }
}

pub struct RestrictedHttpClient {
    policy: HttpPolicy,
    resource_governor: ResourceGovernor,
}

pub struct RestrictedHttpResponse {
    pub status: StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub bytes: Vec<u8>,
    _reservation: Option<ResponseReservation>,
}

pub struct ValidatedHttpTarget {
    pub url: Url,
    host: String,
    addresses: Vec<SocketAddr>,
}

pub struct BodyLimiter {
    max_bytes: usize,
    bytes: Vec<u8>,
}

#[derive(Clone)]
struct ResourceGovernor {
    response_slots: Arc<Semaphore>,
    byte_budget: Arc<Semaphore>,
    max_reserved_bytes: u32,
}

struct ResponseReservation {
    _response_slot: OwnedSemaphorePermit,
    _byte_budget: OwnedSemaphorePermit,
}

impl ResourceGovernor {
    fn new(max_buffered_responses: usize, max_reserved_bytes: u32) -> Self {
        Self {
            response_slots: Arc::new(Semaphore::new(max_buffered_responses)),
            byte_budget: Arc::new(Semaphore::new(max_reserved_bytes as usize)),
            max_reserved_bytes,
        }
    }

    async fn reserve(&self, max_bytes: usize) -> Result<ResponseReservation, String> {
        let max_bytes =
            u32::try_from(max_bytes).map_err(|_| "响应体限制超过全局资源预算".to_string())?;
        if max_bytes > self.max_reserved_bytes {
            return Err(format!(
                "响应体限制超过全局 {} 字节资源预算",
                self.max_reserved_bytes
            ));
        }
        let byte_budget = self
            .byte_budget
            .clone()
            .acquire_many_owned(max_bytes)
            .await
            .map_err(|_| "HTTP 字节资源治理器不可用".to_string())?;
        let response_slot = self
            .response_slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "HTTP 响应资源治理器不可用".to_string())?;
        Ok(ResponseReservation {
            _response_slot: response_slot,
            _byte_budget: byte_budget,
        })
    }

    #[cfg(test)]
    fn available_response_slots(&self) -> usize {
        self.response_slots.available_permits()
    }

    #[cfg(test)]
    fn available_bytes(&self) -> usize {
        self.byte_budget.available_permits()
    }
}

impl BodyLimiter {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            bytes: Vec::new(),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), String> {
        if chunk.len() > self.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(format!("响应体超过 {} 字节限制", self.max_bytes));
        }
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum RequestMode {
    StrictDownload,
    BrowserProxy,
}

impl RestrictedHttpClient {
    pub fn new(policy: HttpPolicy) -> Self {
        Self {
            policy,
            resource_governor: RESOURCE_GOVERNOR.clone(),
        }
    }

    pub async fn fetch_bytes(
        &self,
        url: &str,
        max_bytes: usize,
        accepted_mime: &[&str],
    ) -> Result<RestrictedHttpResponse, String> {
        let response = self
            .request_bytes(Method::GET, url, HashMap::new(), None, max_bytes)
            .await?;
        validate_content_type(&response.headers, accepted_mime)?;
        Ok(response)
    }

    pub async fn request_bytes(
        &self,
        method: Method,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        max_bytes: usize,
    ) -> Result<RestrictedHttpResponse, String> {
        self.request_with_deadline(
            method,
            url,
            headers,
            body,
            max_bytes,
            self.policy.request_timeout,
            RequestMode::StrictDownload,
        )
        .await
    }

    pub async fn proxy_request_bytes(
        &self,
        method: Method,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        max_bytes: usize,
        timeout: Duration,
    ) -> Result<RestrictedHttpResponse, String> {
        self.request_with_deadline(
            method,
            url,
            headers,
            body,
            max_bytes,
            timeout,
            RequestMode::BrowserProxy,
        )
        .await
    }

    async fn request_with_deadline(
        &self,
        method: Method,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        max_bytes: usize,
        timeout: Duration,
        mode: RequestMode,
    ) -> Result<RestrictedHttpResponse, String> {
        let request = async {
            let target = self.validate_url(url).await?;
            let reservation = self.resource_governor.reserve(max_bytes).await?;
            let mut response = self
                .request_inner(method, target, headers, body, max_bytes, mode)
                .await?;
            response._reservation = Some(reservation);
            Ok(response)
        };
        tokio::time::timeout(timeout, request)
            .await
            .map_err(|_| format!("请求超过 {} 毫秒总时限", timeout.as_millis()))?
    }

    async fn request_inner(
        &self,
        mut method: Method,
        mut target: ValidatedHttpTarget,
        mut headers: HashMap<String, String>,
        mut body: Option<Vec<u8>>,
        max_bytes: usize,
        mode: RequestMode,
    ) -> Result<RestrictedHttpResponse, String> {
        if headers.keys().any(|name| name.eq_ignore_ascii_case("host")) {
            return Err("不允许覆盖 Host 请求头".to_string());
        }
        let mut redirects = 0u8;
        loop {
            let client = self.client_for(&target)?;
            let response = self
                .send_with_retries(&client, &target, &method, &headers, &body, mode)
                .await?;
            if response.status().is_redirection() {
                let next_url =
                    redirect_url(&target.url, &response, redirects, self.policy.max_redirects)?;
                if target.url.origin() != next_url.origin() {
                    strip_cross_origin_credentials(&mut headers);
                }
                if response.status() == StatusCode::SEE_OTHER
                    || (matches!(
                        response.status(),
                        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND
                    ) && method == Method::POST)
                {
                    method = Method::GET;
                    body = None;
                }
                target = self.validate_url(next_url.as_str()).await?;
                redirects += 1;
                continue;
            }
            return read_bounded_response(response, max_bytes, mode).await;
        }
    }

    async fn send_with_retries(
        &self,
        client: &reqwest::Client,
        target: &ValidatedHttpTarget,
        method: &Method,
        headers: &HashMap<String, String>,
        body: &Option<Vec<u8>>,
        mode: RequestMode,
    ) -> Result<reqwest::Response, String> {
        let attempts = browser_proxy_retry_attempts(mode, method);
        for attempt in 0..attempts {
            let request_headers = merge_request_headers(target, headers, mode)?;
            let mut request = client
                .request(method.clone(), target.url.clone())
                .headers(request_headers);
            if let Some(request_body) = body {
                request = request.body(request_body.clone());
            }
            let response = request
                .send()
                .await
                .map_err(|error| format!("请求失败: {error}"))?;
            if !response.status().is_server_error() || attempt + 1 == attempts {
                return Ok(response);
            }
            tokio::time::sleep(Duration::from_millis(500 * (attempt as u64 + 1))).await;
        }
        Err("请求失败：重试次数已用尽".to_string())
    }

    async fn validate_url(&self, raw_url: &str) -> Result<ValidatedHttpTarget, String> {
        validate_http_url(raw_url, self.policy.allow_private_networks).await
    }

    fn client_for(&self, target: &ValidatedHttpTarget) -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .connect_timeout(self.policy.connect_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve_to_addrs(&target.host, &target.addresses)
            .build()
            .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
    }
}

fn browser_proxy_retry_attempts(mode: RequestMode, method: &Method) -> u8 {
    if mode == RequestMode::BrowserProxy && matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS") {
        3
    } else {
        1
    }
}

fn merge_request_headers(
    target: &ValidatedHttpTarget,
    headers: &HashMap<String, String>,
    mode: RequestMode,
) -> Result<reqwest::header::HeaderMap, String> {
    let mut merged = reqwest::header::HeaderMap::new();
    if mode == RequestMode::BrowserProxy {
        for (name, value) in browser_default_headers(&target.url) {
            merged.insert(
                reqwest::header::HeaderName::from_static(name),
                value
                    .parse()
                    .map_err(|error| format!("默认请求头无效: {error}"))?,
            );
        }
    }
    for (name, value) in headers {
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| format!("请求头名称无效: {error}"))?;
        let value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|error| format!("请求头值无效: {error}"))?;
        merged.insert(name, value);
    }
    Ok(merged)
}

async fn read_bounded_response(
    mut response: reqwest::Response,
    max_bytes: usize,
    mode: RequestMode,
) -> Result<RestrictedHttpResponse, String> {
    let status = response.status();
    if mode == RequestMode::StrictDownload && !status.is_success() {
        return Err(format!("远程服务器返回 HTTP {status}"));
    }
    let headers = response.headers().clone();
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(format!("响应体超过 {max_bytes} 字节限制"));
    }
    let mut limiter = BodyLimiter::new(max_bytes);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取响应失败: {error}"))?
    {
        limiter.push(&chunk)?;
    }
    Ok(RestrictedHttpResponse {
        status,
        headers,
        bytes: limiter.bytes,
        _reservation: None,
    })
}

fn redirect_url(
    current_url: &Url,
    response: &reqwest::Response,
    redirects: u8,
    max_redirects: u8,
) -> Result<Url, String> {
    if redirects >= max_redirects {
        return Err("重定向次数过多".to_string());
    }
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "重定向响应缺少 Location".to_string())?;
    current_url
        .join(location)
        .map_err(|error| format!("重定向地址无效: {error}"))
}

fn browser_default_headers(url: &Url) -> HashMap<&'static str, String> {
    let origin = url.origin().ascii_serialization();
    HashMap::from([
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string()),
        ("accept", "*/*".to_string()),
        ("accept-language", "zh-CN,zh;q=0.9,en;q=0.8".to_string()),
        ("referer", origin.clone()),
        ("origin", origin),
        ("sec-fetch-dest", "empty".to_string()),
        ("sec-fetch-mode", "cors".to_string()),
        ("sec-fetch-site", "cross-site".to_string()),
    ])
}

#[cfg(test)]
pub async fn validate_public_http_url(raw_url: &str) -> Result<ValidatedHttpTarget, String> {
    validate_http_url(raw_url, false).await
}

pub fn is_blocked_network_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, d] = ip.octets();
            // RFC 2544 benchmark addresses (198.18.0.0/15) are often used by local
            // proxy/VPN "fake-ip" DNS modes for public hostnames. They do not prove
            // that the caller requested a private address, so only the shared loopback
            // mapping remains blocked here.
            if a == 198 && b == 18 && c == 0 && d == 1 {
                return true;
            }
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || a == 0
                || (a == 100 && (64..=127).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 240
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            if let Some(ipv4) = ip.to_ipv4() {
                return is_blocked_network_ip(IpAddr::V4(ipv4));
            }
            ip.is_loopback()
                || ip.is_multicast()
                || ip.is_unspecified()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
                || (segments[0] & 0xffc0) == 0xfec0
                || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        }
    }
}

pub fn strip_cross_origin_credentials(headers: &mut HashMap<String, String>) {
    headers.retain(|name, _| {
        !name.eq_ignore_ascii_case("authorization")
            && !name.eq_ignore_ascii_case("cookie")
            && !name.eq_ignore_ascii_case("proxy-authorization")
    });
}

async fn validate_http_url(
    raw_url: &str,
    allow_private: bool,
) -> Result<ValidatedHttpTarget, String> {
    let url = Url::parse(raw_url).map_err(|error| format!("URL 格式错误: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("仅允许 HTTP 或 HTTPS 地址".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL 不允许包含用户凭据".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "URL 缺少主机名".to_string())?
        .to_string();
    if !allow_private && (host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost")) {
        return Err("禁止访问本机或内网地址".to_string());
    }
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "URL 缺少有效端口".to_string())?;
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|error| format!("解析主机失败: {error}"))?
        .collect();
    if addresses.is_empty()
        || (!allow_private
            && addresses
                .iter()
                .any(|address| is_blocked_network_ip(address.ip())))
    {
        return Err("禁止访问本机或内网地址".to_string());
    }
    Ok(ValidatedHttpTarget {
        url,
        host,
        addresses,
    })
}

fn validate_content_type(
    headers: &reqwest::header::HeaderMap,
    accepted_mime: &[&str],
) -> Result<(), String> {
    if accepted_mime.is_empty() {
        return Ok(());
    }
    let raw = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "响应缺少 Content-Type".to_string())?;
    let actual = parse_mime(raw, false).ok_or_else(|| "响应 Content-Type 无效".to_string())?;
    if accepted_mime
        .iter()
        .filter_map(|mime| parse_mime(mime, true))
        .any(|accepted| {
            (accepted.0 == "*" || accepted.0 == actual.0)
                && (accepted.1 == "*" || accepted.1 == actual.1)
        })
    {
        Ok(())
    } else {
        Err(format!("不接受的响应类型: {}/{}", actual.0, actual.1))
    }
}

fn parse_mime(raw: &str, allow_wildcard: bool) -> Option<(String, String)> {
    let essence = raw.split(';').next()?.trim().to_ascii_lowercase();
    let (mime_type, subtype) = essence.split_once('/')?;
    let valid = |token: &str| {
        !token.is_empty()
            && token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
    };
    let type_valid = valid(mime_type) || (allow_wildcard && mime_type == "*" && subtype == "*");
    let subtype_valid = valid(subtype) || (allow_wildcard && subtype == "*");
    (type_valid && subtype_valid).then(|| (mime_type.to_string(), subtype.to_string()))
}

pub fn decode_response_text(headers: &reqwest::header::HeaderMap, bytes: &[u8]) -> String {
    let charset = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(';').skip(1).find_map(|parameter| {
                let (name, value) = parameter.trim().split_once('=')?;
                name.trim()
                    .eq_ignore_ascii_case("charset")
                    .then(|| value.trim().trim_matches('"'))
            })
        });
    let encoding = charset
        .and_then(|label| Encoding::for_label(label.as_bytes()))
        .unwrap_or(UTF_8);
    encoding.decode(bytes).0.into_owned()
}

pub fn clamp_proxy_timeout(timeout_ms: u64) -> Duration {
    Duration::from_millis(timeout_ms.clamp(1, 120_000))
}

#[cfg(test)]
mod tests {
    use super::{
        browser_default_headers, browser_proxy_retry_attempts, clamp_proxy_timeout,
        decode_response_text, is_blocked_network_ip, validate_content_type,
        validate_public_http_url, BodyLimiter, HttpPolicy, RequestMode, ResourceGovernor,
        RestrictedHttpClient,
    };
    use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
    use reqwest::{Method, StatusCode};
    use std::net::IpAddr;
    use std::time::Duration;

    #[tokio::test]
    async fn rejects_private_and_credential_urls() {
        assert!(validate_public_http_url("http://127.0.0.1/x")
            .await
            .is_err());
        assert!(validate_public_http_url("http://user:pass@example.com/x")
            .await
            .is_err());
    }

    #[test]
    fn rejects_body_larger_than_limit() {
        let mut limiter = BodyLimiter::new(4);
        assert!(limiter.push(&[1, 2, 3]).is_ok());
        assert!(limiter.push(&[4, 5]).is_err());
    }

    #[test]
    fn blocks_ipv6_site_local_range() {
        for raw_ip in ["fec0::1", "feff:ffff::1"] {
            assert!(is_blocked_network_ip(raw_ip.parse::<IpAddr>().unwrap()));
        }
    }

    #[test]
    fn allows_proxy_fake_ip_ranges_but_keeps_shared_loopback_blocked() {
        for raw_ip in ["198.18.0.2", "198.18.255.254", "198.19.0.1", "198.19.255.255"] {
            assert!(!is_blocked_network_ip(raw_ip.parse::<IpAddr>().unwrap()));
        }
        assert!(is_blocked_network_ip("198.18.0.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn mime_matching_requires_exact_or_wildcard_subtype() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("image/jpeg; charset=utf-8"),
        );
        assert!(validate_content_type(&headers, &["image/*"]).is_ok());
        assert!(validate_content_type(&headers, &["image/jpeg"]).is_ok());
        assert!(validate_content_type(&headers, &["*/*"]).is_ok());
        assert!(validate_content_type(&headers, &["image/j"]).is_err());
        assert!(validate_content_type(&headers, &["image/"]).is_err());
        assert!(validate_content_type(&headers, &["*/jpeg"]).is_err());
    }

    #[test]
    fn decodes_text_using_declared_charset_lossily() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=windows-1252"),
        );
        assert_eq!("café", decode_response_text(&headers, b"caf\xe9"));
        assert_eq!("�", decode_response_text(&HeaderMap::new(), &[0xff]));
    }

    #[test]
    fn proxy_timeout_is_clamped_to_safe_range() {
        assert_eq!(Duration::from_millis(1), clamp_proxy_timeout(0));
        assert_eq!(Duration::from_millis(15_000), clamp_proxy_timeout(15_000));
        assert_eq!(
            Duration::from_millis(120_000),
            clamp_proxy_timeout(u64::MAX)
        );
    }

    #[test]
    fn browser_headers_follow_current_origin() {
        let url = reqwest::Url::parse("https://example.com:8443/path").unwrap();
        let headers = browser_default_headers(&url);
        assert_eq!(
            Some(&"https://example.com:8443".to_string()),
            headers.get("origin")
        );
        assert_eq!(headers.get("origin"), headers.get("referer"));
        assert!(headers
            .get("user-agent")
            .is_some_and(|value| value.contains("Mozilla")));
    }

    #[test]
    fn browser_proxy_only_retries_safe_methods() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(
                3,
                browser_proxy_retry_attempts(RequestMode::BrowserProxy, &method)
            );
        }
        for method in [Method::POST, Method::PATCH, Method::PUT, Method::DELETE] {
            assert_eq!(
                1,
                browser_proxy_retry_attempts(RequestMode::BrowserProxy, &method)
            );
        }
        assert_eq!(
            1,
            browser_proxy_retry_attempts(RequestMode::StrictDownload, &Method::GET)
        );
    }

    #[tokio::test]
    async fn resource_governor_queues_and_releases_response_budget() {
        let governor = ResourceGovernor::new(1, 8);
        let reservation = governor.reserve(8).await.unwrap();
        assert_eq!(0, governor.available_response_slots());
        assert_eq!(0, governor.available_bytes());

        let waiting_governor = governor.clone();
        let waiter = tokio::spawn(async move { waiting_governor.reserve(4).await.is_ok() });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        drop(reservation);
        assert!(waiter.await.unwrap());
        assert_eq!(1, governor.available_response_slots());
        assert_eq!(8, governor.available_bytes());
    }

    #[tokio::test]
    async fn resource_governor_releases_bytes_when_a_reservation_is_dropped() {
        let governor = ResourceGovernor::new(2, 8);
        let reservation = governor.reserve(8).await.unwrap();
        assert_eq!(0, governor.available_bytes());

        drop(reservation);

        assert_eq!(2, governor.available_response_slots());
        assert_eq!(8, governor.available_bytes());
    }

    #[tokio::test]
    async fn resource_governor_rejects_reservations_larger_than_budget() {
        let governor = ResourceGovernor::new(1, 8);
        let error = governor.reserve(9).await.err().unwrap();
        assert!(error.contains("全局"));
        assert_eq!(1, governor.available_response_slots());
        assert_eq!(8, governor.available_bytes());
    }

    #[tokio::test]
    async fn plugin_download_accepts_local_http_source() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::io::AsyncReadExt::read(&mut socket, &mut [0; 1024])
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 200 OK\r\nContent-Type: text/javascript\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            )
            .await
            .unwrap();
        });

        let response = RestrictedHttpClient::new(HttpPolicy::plugin_download())
            .fetch_bytes(&format!("http://{address}/plugin.js"), 1024, &["text/*"])
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(b"ok", response.bytes.as_slice());
    }

    #[tokio::test]
    async fn strict_download_rejects_error_status_but_proxy_preserves_it() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::io::AsyncReadExt::read(&mut socket, &mut [0; 1024])
                    .await
                    .unwrap();
                tokio::io::AsyncWriteExt::write_all(
                    &mut socket,
                    b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain; charset=utf-8\r\nX-Test: kept\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing",
                )
                .await
                .unwrap();
            }
        });
        let url = format!("http://{address}/missing");
        let client = RestrictedHttpClient::new(HttpPolicy::dlna_local());

        assert!(client.fetch_bytes(&url, 1024, &["text/*"]).await.is_err());
        let response = client
            .proxy_request_bytes(
                Method::GET,
                &url,
                Default::default(),
                None,
                1024,
                Duration::from_secs(2),
            )
            .await
            .unwrap();

        assert_eq!(reqwest::StatusCode::NOT_FOUND, response.status);
        assert_eq!(
            Some("kept"),
            response.headers.get("x-test").unwrap().to_str().ok()
        );
        assert_eq!(b"missing", response.bytes.as_slice());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn browser_proxy_retries_get_server_errors_but_not_post() {
        let get_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let get_address = get_listener.local_addr().unwrap();
        let get_server = tokio::spawn(async move {
            for response in [
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice(),
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice(),
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".as_slice(),
            ] {
                let (mut socket, _) = get_listener.accept().await.unwrap();
                tokio::io::AsyncReadExt::read(&mut socket, &mut [0; 1024])
                    .await
                    .unwrap();
                tokio::io::AsyncWriteExt::write_all(&mut socket, response)
                    .await
                    .unwrap();
            }
        });
        let client = RestrictedHttpClient::new(HttpPolicy::dlna_local());
        let response = client
            .proxy_request_bytes(
                Method::GET,
                &format!("http://{get_address}/retry"),
                Default::default(),
                None,
                1024,
                Duration::from_secs(5),
            )
            .await
            .unwrap();
        assert_eq!(StatusCode::OK, response.status);
        get_server.await.unwrap();

        let post_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let post_address = post_listener.local_addr().unwrap();
        let post_server = tokio::spawn(async move {
            let (mut socket, _) = post_listener.accept().await.unwrap();
            tokio::io::AsyncReadExt::read(&mut socket, &mut [0; 1024])
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        });
        let response = client
            .proxy_request_bytes(
                Method::POST,
                &format!("http://{post_address}/no-retry"),
                Default::default(),
                Some(b"payload".to_vec()),
                1024,
                Duration::from_secs(2),
            )
            .await
            .unwrap();
        assert_eq!(StatusCode::SERVICE_UNAVAILABLE, response.status);
        post_server.await.unwrap();
    }

    #[tokio::test]
    async fn one_deadline_covers_the_entire_redirect_chain() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for response in [
                b"HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".as_slice(),
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".as_slice(),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 1024];
                tokio::io::AsyncReadExt::read(&mut socket, &mut buffer).await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
                tokio::io::AsyncWriteExt::write_all(&mut socket, response).await.unwrap();
            }
        });
        let client = RestrictedHttpClient::new(HttpPolicy::dlna_local());
        let result = client
            .proxy_request_bytes(
                Method::GET,
                &format!("http://{address}/start"),
                Default::default(),
                None,
                1024,
                Duration::from_millis(150),
            )
            .await;

        assert!(result.is_err());
        server.abort();
    }
}
