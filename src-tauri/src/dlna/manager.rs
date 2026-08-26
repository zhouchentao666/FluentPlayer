use crate::dlna::types::{
    is_local_unicast_ipv4, search_responses, CachedDevice, ControlService, DlnaDevice,
    PositionInfo, SsdpResponse,
};
use crate::dlna::xml::{
    build_didl_metadata, build_soap_envelope, format_duration, parse_device_description,
    parse_position_info, parse_soap_fault,
};
use crate::network::{HttpPolicy, RestrictedHttpClient};
use futures_util::stream::{self, StreamExt};
use reqwest::{Method, Url};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tokio::sync::{watch, Mutex as AsyncMutex, RwLock};
use tokio::time::{timeout_at, Instant};

const MAX_DESCRIPTION_BYTES: usize = 256 * 1024;
const MAX_SOAP_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_DESCRIPTION_FETCHES: usize = 6;
const DESCRIPTION_WINDOW: Duration = Duration::from_secs(3);
const SEARCH_COOLDOWN: Duration = Duration::from_secs(3);

pub struct DlnaManager {
    cache: Arc<RwLock<HashMap<String, CachedDevice>>>,
    active_location: Arc<RwLock<Option<String>>>,
    active_generation: Arc<AtomicU64>,
    handoff_generation_gate: Arc<RwLock<()>>,
    play_handoff: Arc<AsyncMutex<()>>,
    search_generation: Arc<AtomicU64>,
    search_gate: AsyncMutex<SearchGate>,
    search_cancel: watch::Sender<u64>,
    multicast_lock: Arc<MulticastLockState>,
    http: Arc<RestrictedHttpClient>,
}

#[derive(Default)]
struct SearchGate {
    last_finished: Option<Instant>,
    last_result: Option<Result<Vec<DlnaDevice>, String>>,
}

struct MulticastLockState {
    held_generation: AtomicU64,
    operation: Mutex<()>,
}

struct MulticastLockLease {
    state: Arc<MulticastLockState>,
    generation: u64,
}

impl DlnaManager {
    pub fn new() -> Self {
        let mut policy = HttpPolicy::dlna_local();
        policy.max_redirects = 0;
        let (search_cancel, _) = watch::channel(0);
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            active_location: Arc::new(RwLock::new(None)),
            active_generation: Arc::new(AtomicU64::new(0)),
            handoff_generation_gate: Arc::new(RwLock::new(())),
            play_handoff: Arc::new(AsyncMutex::new(())),
            search_generation: Arc::new(AtomicU64::new(0)),
            search_gate: AsyncMutex::new(SearchGate::default()),
            search_cancel,
            multicast_lock: Arc::new(MulticastLockState::new()),
            http: Arc::new(RestrictedHttpClient::new(policy)),
        }
    }

    pub async fn start_search(&self) -> Result<Vec<DlnaDevice>, String> {
        let mut gate = self.search_gate.lock().await;
        if let Some(result) = gate.recent_result(Instant::now()) {
            return result;
        }
        let generation = self.search_generation.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.search_cancel.send(generation);
        let result = match self.acquire_multicast_lock(generation) {
            Ok(_lock) => self.search_with_lock(generation).await,
            Err(error) => Err(error),
        };
        gate.complete(result.clone(), Instant::now());
        result
    }

    pub fn stop_search(&self) {
        let generation = self.search_generation.fetch_add(1, Ordering::AcqRel) + 1;
        let _ = self.search_cancel.send(generation);
        self.multicast_lock.release_before(generation);
    }

    pub async fn devices(&self) -> Vec<DlnaDevice> {
        let mut devices = self
            .cache
            .read()
            .await
            .values()
            .map(|device| device.public.clone())
            .collect::<Vec<_>>();
        devices.sort_by(|left, right| left.name.cmp(&right.name));
        devices
    }

    pub async fn play(&self, location: &str, media_url: &str, title: &str) -> Result<(), String> {
        validate_media_url(media_url)?;
        let _handoff = self.play_handoff.lock().await;
        let generation = self.begin_play_handoff().await;
        self.ensure_current_handoff(generation)?;
        let device = self.device_for_location(location).await?;
        let metadata = build_didl_metadata(title, media_url);
        let result = async {
            self.av_transport_if_current(
                generation,
                &device,
                "SetAVTransportURI",
                vec![
                    ("InstanceID", "0".to_string()),
                    ("CurrentURI", media_url.to_string()),
                    ("CurrentURIMetaData", metadata),
                ],
            )
            .await?;
            self.av_transport_if_current(
                generation,
                &device,
                "Play",
                vec![("InstanceID", "0".to_string()), ("Speed", "1".to_string())],
            )
            .await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                self.set_active_location_if_current(generation, Some(location))
                    .await;
                Ok(())
            }
            Err(error) => {
                self.set_active_location_if_current(generation, None).await;
                Err(error)
            }
        }
    }

    pub async fn pause(&self) -> Result<(), String> {
        let device = self.active_device().await?;
        self.av_transport(&device, "Pause", vec![("InstanceID", "0".to_string())])
            .await
            .map(|_| ())
    }

    pub async fn resume(&self) -> Result<(), String> {
        let device = self.active_device().await?;
        self.av_transport(
            &device,
            "Play",
            vec![("InstanceID", "0".to_string()), ("Speed", "1".to_string())],
        )
        .await
        .map(|_| ())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let _handoff = self.play_handoff.lock().await;
        let _generation = self.handoff_generation_gate.write().await;
        self.active_generation.fetch_add(1, Ordering::AcqRel);
        drop(_generation);
        let device = self.active_device().await;
        *self.active_location.write().await = None;
        let device = device?;
        let result = self
            .av_transport(&device, "Stop", vec![("InstanceID", "0".to_string())])
            .await
            .map(|_| ());
        result
    }

    pub async fn seek(&self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("播放位置必须是非负有限数字".to_string());
        }
        let device = self.active_device().await?;
        self.av_transport(
            &device,
            "Seek",
            vec![
                ("InstanceID", "0".to_string()),
                ("Unit", "REL_TIME".to_string()),
                ("Target", format_duration(seconds)),
            ],
        )
        .await
        .map(|_| ())
    }

    pub async fn set_volume(&self, volume: f64) -> Result<(), String> {
        let device = self.active_device().await?;
        self.rendering_control(
            &device,
            "SetVolume",
            vec![
                ("InstanceID", "0".to_string()),
                ("Channel", "Master".to_string()),
                ("DesiredVolume", normalize_volume(volume)?.to_string()),
            ],
        )
        .await
        .map(|_| ())
    }

    pub async fn position(&self) -> Result<PositionInfo, String> {
        let device = self.active_device().await?;
        let response = self
            .av_transport(
                &device,
                "GetPositionInfo",
                vec![("InstanceID", "0".to_string())],
            )
            .await?;
        parse_position_info(&response, &device.av_transport.service_type)
    }

    async fn describe_all(
        &self,
        generation: u64,
        cancellation: &mut watch::Receiver<u64>,
        responses: HashMap<String, SsdpResponse>,
    ) -> Result<Vec<CachedDevice>, String> {
        let descriptions = stream::iter(
            unique_responses_by_location(responses)
                .into_iter()
                .map(|response| self.describe(response)),
        )
        .buffer_unordered(MAX_DESCRIPTION_FETCHES);
        tokio::pin!(descriptions);
        let deadline = Instant::now() + DESCRIPTION_WINDOW;
        let mut devices = Vec::new();
        while self.is_current_search(generation) {
            let result = tokio::select! {
                _ = cancellation.changed() => return Err("DLNA 搜索已取消".to_string()),
                result = next_before_deadline(descriptions.as_mut(), deadline) => result,
            };
            let Some(result) = result else {
                break;
            };
            if let Ok(device) = result {
                devices.push(device);
            }
        }
        if self.is_current_search(generation) {
            Ok(devices)
        } else {
            Err("DLNA 搜索已取消".to_string())
        }
    }

    async fn describe(&self, response: SsdpResponse) -> Result<CachedDevice, String> {
        let device_ip = ipv4_sender(response.sender)?;
        validate_trusted_endpoint(&response.location, device_ip)?;
        let document = self
            .http
            .request_bytes(
                Method::GET,
                &response.location,
                HashMap::new(),
                None,
                MAX_DESCRIPTION_BYTES,
            )
            .await?;
        let document = String::from_utf8(document.bytes)
            .map_err(|_| "DLNA 设备描述不是 UTF-8 文本".to_string())?;
        let parsed = parse_device_description(&document, &response.location)?;
        validate_trusted_service(&parsed.av_transport, device_ip)?;
        validate_trusted_service(&parsed.rendering_control, device_ip)?;
        Ok(CachedDevice {
            public: DlnaDevice {
                usn: response.usn,
                name: parsed.name,
                location: response.location,
                address: device_ip.to_string(),
            },
            av_transport: parsed.av_transport,
            rendering_control: parsed.rendering_control,
            device_ip,
        })
    }

    async fn replace_cache_if_current(&self, generation: u64, devices: Vec<CachedDevice>) -> bool {
        let _handoff = self.play_handoff.lock().await;
        let mut replacement = devices
            .into_iter()
            .map(|device| (device.public.location.clone(), device))
            .collect::<HashMap<_, _>>();
        let active_location = self.active_location.read().await.clone();
        let mut cache = self.cache.write().await;
        if self.is_current_search(generation) {
            preserve_active_device(&mut replacement, &cache, active_location.as_deref());
            *cache = replacement;
            true
        } else {
            false
        }
    }

    async fn active_device(&self) -> Result<CachedDevice, String> {
        let location = self
            .active_location
            .read()
            .await
            .clone()
            .ok_or_else(|| "尚未开始 DLNA 投放".to_string())?;
        self.device_for_location(&location).await
    }

    async fn device_for_location(&self, location: &str) -> Result<CachedDevice, String> {
        self.cache
            .read()
            .await
            .get(location)
            .cloned()
            .ok_or_else(|| "DLNA 设备未在当前发现缓存中".to_string())
    }

    async fn begin_play_handoff(&self) -> u64 {
        let _generation = self.handoff_generation_gate.write().await;
        self.active_generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    async fn set_active_location_if_current(
        &self,
        generation: u64,
        location: Option<&str>,
    ) -> bool {
        if self.active_generation.load(Ordering::Acquire) != generation {
            return false;
        }
        let mut active_location = self.active_location.write().await;
        if self.active_generation.load(Ordering::Acquire) != generation {
            return false;
        }
        *active_location = location.map(ToString::to_string);
        true
    }

    fn ensure_current_handoff(&self, generation: u64) -> Result<(), String> {
        if self.active_generation.load(Ordering::Acquire) == generation {
            Ok(())
        } else {
            Err("DLNA 投放已被更新的播放请求取代".to_string())
        }
    }

    async fn av_transport_if_current(
        &self,
        generation: u64,
        device: &CachedDevice,
        action: &str,
        values: Vec<(&str, String)>,
    ) -> Result<String, String> {
        let _generation = self.handoff_generation_gate.read().await;
        self.ensure_current_handoff(generation)?;
        self.av_transport(device, action, values).await
    }

    async fn av_transport(
        &self,
        device: &CachedDevice,
        action: &str,
        values: Vec<(&str, String)>,
    ) -> Result<String, String> {
        self.soap(device, &device.av_transport, action, values)
            .await
    }

    async fn rendering_control(
        &self,
        device: &CachedDevice,
        action: &str,
        values: Vec<(&str, String)>,
    ) -> Result<String, String> {
        self.soap(device, &device.rendering_control, action, values)
            .await
    }

    async fn soap(
        &self,
        device: &CachedDevice,
        service: &ControlService,
        action: &str,
        values: Vec<(&str, String)>,
    ) -> Result<String, String> {
        validate_trusted_service(service, device.device_ip)?;
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "text/xml; charset=\"utf-8\"".to_string(),
        );
        headers.insert(
            "SOAPACTION".to_string(),
            soap_action(&service.service_type, action),
        );
        let response = self
            .http
            .request_bytes(
                Method::POST,
                service.url.as_str(),
                headers,
                Some(build_soap_envelope(&service.service_type, action, &values).into_bytes()),
                MAX_SOAP_RESPONSE_BYTES,
            )
            .await?;
        decode_soap_document(response.bytes)
    }

    fn is_current_search(&self, generation: u64) -> bool {
        self.search_generation.load(Ordering::Acquire) == generation
    }

    fn acquire_multicast_lock(&self, generation: u64) -> Result<MulticastLockLease, String> {
        self.multicast_lock
            .acquire(generation, &self.search_generation)
    }

    async fn search_with_lock(&self, generation: u64) -> Result<Vec<DlnaDevice>, String> {
        let mut cancellation = self.search_cancel.subscribe();
        let responses =
            search_responses(generation, &self.search_generation, &mut cancellation).await?;
        let devices = self
            .describe_all(generation, &mut cancellation, responses)
            .await?;
        let _ = self.replace_cache_if_current(generation, devices).await;
        if !self.is_current_search(generation) {
            return Err("DLNA 搜索已取消".to_string());
        }
        Ok(self.devices().await)
    }
}

impl SearchGate {
    fn recent_result(&self, now: Instant) -> Option<Result<Vec<DlnaDevice>, String>> {
        self.last_finished
            .filter(|finished| now.saturating_duration_since(*finished) < SEARCH_COOLDOWN)
            .and_then(|_| self.last_result.clone())
    }

    fn complete(&mut self, result: Result<Vec<DlnaDevice>, String>, now: Instant) {
        if matches!(&result, Err(error) if error == "DLNA 搜索已取消") {
            self.last_finished = None;
            self.last_result = None;
        } else {
            self.last_finished = Some(now);
            self.last_result = Some(result);
        }
    }
}

impl MulticastLockState {
    fn new() -> Self {
        Self {
            held_generation: AtomicU64::new(0),
            operation: Mutex::new(()),
        }
    }

    fn acquire(
        self: &Arc<Self>,
        generation: u64,
        current_generation: &AtomicU64,
    ) -> Result<MulticastLockLease, String> {
        let _operation = self.lock_operation();
        if generation != current_generation.load(Ordering::Acquire) {
            return Err("DLNA 搜索已取消".to_string());
        }
        if self.held_generation.load(Ordering::Acquire) == 0 {
            crate::dlna::android::acquire_multicast_lock()?;
        }
        self.held_generation.store(generation, Ordering::Release);
        Ok(MulticastLockLease {
            state: self.clone(),
            generation,
        })
    }

    fn release_if_owned(&self, generation: u64) {
        let _operation = self.lock_operation();
        if self.held_generation.load(Ordering::Acquire) == generation {
            crate::dlna::android::release_multicast_lock();
            self.held_generation.store(0, Ordering::Release);
        }
    }

    fn release_before(&self, generation: u64) {
        let _operation = self.lock_operation();
        let held = self.held_generation.load(Ordering::Acquire);
        if held != 0 && held < generation {
            crate::dlna::android::release_multicast_lock();
            self.held_generation.store(0, Ordering::Release);
        }
    }

    fn release_all(&self) {
        let _operation = self.lock_operation();
        if self.held_generation.swap(0, Ordering::AcqRel) != 0 {
            crate::dlna::android::release_multicast_lock();
        }
    }

    fn lock_operation(&self) -> std::sync::MutexGuard<'_, ()> {
        self.operation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

impl Drop for MulticastLockLease {
    fn drop(&mut self) {
        self.state.release_if_owned(self.generation);
    }
}

impl Drop for DlnaManager {
    fn drop(&mut self) {
        self.multicast_lock.release_all();
    }
}

fn unique_responses_by_location(responses: HashMap<String, SsdpResponse>) -> Vec<SsdpResponse> {
    let mut unique = HashMap::new();
    for response in responses.into_values() {
        unique.entry(response.location.clone()).or_insert(response);
    }
    unique.into_values().collect()
}

fn preserve_active_device(
    replacement: &mut HashMap<String, CachedDevice>,
    current: &HashMap<String, CachedDevice>,
    active_location: Option<&str>,
) {
    let Some(location) = active_location else {
        return;
    };
    if replacement.contains_key(location) {
        return;
    }
    if let Some(active) = current.get(location) {
        replacement.retain(|_, device| device.public.usn != active.public.usn);
        replacement.insert(location.to_string(), active.clone());
    }
}

fn decode_soap_document(bytes: Vec<u8>) -> Result<String, String> {
    let document =
        String::from_utf8(bytes).map_err(|_| "DLNA SOAP 响应不是 UTF-8 文本".to_string())?;
    if let Some(fault) = parse_soap_fault(&document) {
        Err(fault)
    } else {
        Ok(document)
    }
}

async fn next_before_deadline<T, S>(mut stream: Pin<&mut S>, deadline: Instant) -> Option<T>
where
    S: futures_util::Stream<Item = T> + ?Sized,
{
    let next = futures_util::future::poll_fn(|context| stream.as_mut().poll_next(context));
    timeout_at(deadline, next).await.ok().flatten()
}

fn normalize_volume(volume: f64) -> Result<u8, String> {
    if !volume.is_finite() {
        return Err("音量必须是有限数字".to_string());
    }
    Ok(volume.clamp(0.0, 100.0).round() as u8)
}

fn soap_action(service_type: &str, action: &str) -> String {
    format!("\"{service_type}#{action}\"")
}

fn ipv4_sender(sender: std::net::SocketAddr) -> Result<Ipv4Addr, String> {
    match sender.ip() {
        std::net::IpAddr::V4(address) if is_local_unicast_ipv4(address) => Ok(address),
        _ => Err("DLNA 响应未来自受信任的私有 IPv4 地址".to_string()),
    }
}

fn validate_trusted_service(service: &ControlService, device_ip: Ipv4Addr) -> Result<(), String> {
    validate_trusted_endpoint(service.url.as_str(), device_ip)
}

fn validate_trusted_endpoint(raw_url: &str, device_ip: Ipv4Addr) -> Result<(), String> {
    let url = Url::parse(raw_url).map_err(|error| format!("DLNA 端点无效: {error}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url
            .host_str()
            .and_then(|host| host.parse::<Ipv4Addr>().ok())
            != Some(device_ip)
    {
        return Err("DLNA 端点必须使用已发现设备的私有 IPv4 地址".to_string());
    }
    Ok(())
}

fn validate_media_url(raw_url: &str) -> Result<(), String> {
    let url = Url::parse(raw_url).map_err(|error| format!("媒体地址无效: {error}"))?;
    if matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
    {
        Ok(())
    } else {
        Err("媒体地址必须是无凭据的 HTTP(S) 地址".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_soap_document, next_before_deadline, normalize_volume, preserve_active_device,
        soap_action, unique_responses_by_location, validate_trusted_endpoint, DlnaManager,
        MulticastLockState, SearchGate, SEARCH_COOLDOWN,
    };
    use crate::dlna::types::{CachedDevice, ControlService, DlnaDevice, SsdpResponse};
    use futures_util::stream;
    use reqwest::Url;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::time::Instant;

    #[test]
    fn clamps_volume_as_a_0_to_100_value() {
        assert_eq!(normalize_volume(-1.0).unwrap(), 0);
        assert_eq!(normalize_volume(0.0).unwrap(), 0);
        assert_eq!(normalize_volume(50.4).unwrap(), 50);
        assert_eq!(normalize_volume(50.5).unwrap(), 51);
        assert_eq!(normalize_volume(100.0).unwrap(), 100);
        assert_eq!(normalize_volume(101.0).unwrap(), 100);
    }

    #[test]
    fn deduplicates_responses_by_location_before_description_fetches() {
        let sender = "192.168.1.20:1900".parse::<SocketAddr>().unwrap();
        let mut responses = HashMap::new();
        responses.insert(
            "uuid:one".to_string(),
            SsdpResponse {
                usn: "uuid:one".to_string(),
                location: "http://192.168.1.20/device.xml".to_string(),
                sender,
            },
        );
        responses.insert(
            "uuid:two".to_string(),
            SsdpResponse {
                usn: "uuid:two".to_string(),
                location: "http://192.168.1.20/device.xml".to_string(),
                sender,
            },
        );

        assert_eq!(unique_responses_by_location(responses).len(), 1);
    }

    #[tokio::test]
    async fn description_collection_stops_at_the_shared_deadline() {
        let delayed = stream::once(async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            1
        });
        tokio::pin!(delayed);
        let next =
            next_before_deadline(delayed.as_mut(), Instant::now() + Duration::from_millis(1)).await;

        assert_eq!(next, None);
    }

    #[test]
    fn rejects_an_untrusted_control_url() {
        assert!(validate_trusted_endpoint(
            "http://192.168.1.21/control",
            "192.168.1.20".parse().unwrap()
        )
        .is_err());
        assert!(validate_trusted_endpoint(
            "http://8.8.8.8/control",
            "192.168.1.20".parse().unwrap()
        )
        .is_err());
    }

    #[test]
    fn preserves_v2_service_type_in_soap_action() {
        assert_eq!(
            soap_action("urn:schemas-upnp-org:service:AVTransport:2", "Play"),
            "\"urn:schemas-upnp-org:service:AVTransport:2#Play\""
        );
    }

    #[tokio::test]
    async fn stale_search_cannot_replace_a_newer_cache() {
        let manager = DlnaManager::new();
        let generation = manager
            .search_generation
            .load(std::sync::atomic::Ordering::Acquire);
        manager.stop_search();
        assert!(
            !manager
                .replace_cache_if_current(generation, Vec::new())
                .await
        );
        assert_eq!(
            manager
                .search_generation
                .load(std::sync::atomic::Ordering::Acquire),
            generation + 1
        );
    }

    #[tokio::test]
    async fn stop_notifies_an_outstanding_search() {
        let manager = DlnaManager::new();
        let mut cancellation = manager.search_cancel.subscribe();
        manager.stop_search();
        tokio::time::timeout(std::time::Duration::from_millis(50), cancellation.changed())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn older_play_completion_cannot_replace_newer_active_location() {
        let manager = DlnaManager::new();
        let older = manager.begin_play_handoff().await;
        let newer = manager.begin_play_handoff().await;

        assert!(
            manager
                .set_active_location_if_current(newer, Some("http://192.168.1.21/device.xml"))
                .await
        );
        assert!(
            !manager
                .set_active_location_if_current(older, Some("http://192.168.1.20/device.xml"))
                .await
        );
        assert_eq!(
            manager.active_location.read().await.as_deref(),
            Some("http://192.168.1.21/device.xml")
        );
        assert!(!manager.set_active_location_if_current(older, None).await);
        assert_eq!(
            manager.active_location.read().await.as_deref(),
            Some("http://192.168.1.21/device.xml")
        );
    }

    #[tokio::test]
    async fn newer_handoff_invalidates_old_sequence_while_lock_is_held() {
        let manager = DlnaManager::new();
        let older = manager.begin_play_handoff().await;
        let handoff = manager.play_handoff.lock().await;
        let newer = manager.begin_play_handoff().await;

        assert!(manager.ensure_current_handoff(older).is_err());
        drop(handoff);
        let _new_handoff = manager.play_handoff.lock().await;
        assert!(manager.ensure_current_handoff(newer).is_ok());
    }

    #[tokio::test]
    async fn stop_waits_for_play_handoff_before_invalidating_it() {
        let manager = DlnaManager::new();
        let play_generation = manager.begin_play_handoff().await;
        let handoff = manager.play_handoff.lock().await;
        let stop = manager.stop();
        tokio::pin!(stop);

        assert!(matches!(
            futures_util::poll!(stop.as_mut()),
            std::task::Poll::Pending
        ));
        assert_eq!(
            manager
                .active_generation
                .load(std::sync::atomic::Ordering::Acquire),
            play_generation
        );
        assert!(
            manager
                .set_active_location_if_current(
                    play_generation,
                    Some("http://192.168.1.20/device.xml"),
                )
                .await
        );

        drop(handoff);
        assert_eq!(stop.await.unwrap_err(), "DLNA 设备未在当前发现缓存中");
        assert_eq!(
            manager
                .active_generation
                .load(std::sync::atomic::Ordering::Acquire),
            play_generation + 1
        );
        assert!(manager.active_location.read().await.is_none());
    }

    #[tokio::test]
    async fn newer_handoff_waits_for_an_in_flight_action_before_invalidating_it() {
        let manager = DlnaManager::new();
        let older = manager.begin_play_handoff().await;
        let action = manager.handoff_generation_gate.read().await;
        let newer = manager.begin_play_handoff();
        tokio::pin!(newer);

        assert!(
            tokio::time::timeout(Duration::from_millis(1), newer.as_mut())
                .await
                .is_err()
        );
        assert!(manager.ensure_current_handoff(older).is_ok());
        drop(action);
        let newer = newer.await;

        assert!(manager.ensure_current_handoff(older).is_err());
        assert!(manager.ensure_current_handoff(newer).is_ok());
    }

    #[test]
    fn cache_replacement_preserves_the_active_trusted_device() {
        let active = cached_device("uuid:active", "http://192.168.1.20/device.xml");
        let other = cached_device("uuid:other", "http://192.168.1.21/device.xml");
        let moved_active = cached_device("uuid:active", "http://192.168.1.22/device.xml");
        let current = HashMap::from([(active.public.location.clone(), active.clone())]);
        let mut replacement = HashMap::from([
            (other.public.location.clone(), other),
            (moved_active.public.location.clone(), moved_active.clone()),
        ]);

        preserve_active_device(
            &mut replacement,
            &current,
            Some(active.public.location.as_str()),
        );

        assert_eq!(replacement.len(), 2);
        assert!(!replacement.contains_key(moved_active.public.location.as_str()));
        assert_eq!(
            replacement
                .get(active.public.location.as_str())
                .map(|device| device.public.usn.as_str()),
            Some("uuid:active")
        );
    }

    #[test]
    fn search_gate_reuses_results_during_cooldown() {
        let now = Instant::now();
        let mut gate = SearchGate::default();
        gate.complete(Ok(Vec::new()), now);

        assert!(gate
            .recent_result(now + SEARCH_COOLDOWN - Duration::from_millis(1))
            .is_some());
        assert!(gate
            .recent_result(now + SEARCH_COOLDOWN + Duration::from_millis(1))
            .is_none());
    }

    #[test]
    fn soap_fault_is_rejected_even_with_http_success() {
        let fault = br#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring><detail><UPnPError><errorCode>701</errorCode><errorDescription>Transition not available</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>"#;

        assert_eq!(
            decode_soap_document(fault.to_vec()).unwrap_err(),
            "DLNA SOAP Fault 701: Transition not available"
        );
    }

    #[test]
    fn old_search_cleanup_cannot_release_a_newer_multicast_lock_lease() {
        let lock = std::sync::Arc::new(MulticastLockState::new());
        let generation = std::sync::atomic::AtomicU64::new(1);
        let old_lease = lock.acquire(1, &generation).unwrap();
        generation.store(2, std::sync::atomic::Ordering::Release);
        let new_lease = lock.acquire(2, &generation).unwrap();

        drop(old_lease);
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );

        drop(new_lease);
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
    }

    #[test]
    fn stopping_a_search_releases_only_its_multicast_lock_lease() {
        let lock = std::sync::Arc::new(MulticastLockState::new());
        let generation = std::sync::atomic::AtomicU64::new(4);
        let lease = lock.acquire(4, &generation).unwrap();

        lock.release_before(5);
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );

        drop(lease);
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
    }

    #[test]
    fn stale_search_cannot_acquire_a_multicast_lock() {
        let lock = std::sync::Arc::new(MulticastLockState::new());
        let generation = std::sync::atomic::AtomicU64::new(2);

        assert!(lock.acquire(1, &generation).is_err());
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
    }

    #[test]
    fn manager_drop_releases_the_multicast_lock() {
        let manager = DlnaManager::new();
        let lease = manager
            .multicast_lock
            .acquire(1, &std::sync::atomic::AtomicU64::new(1))
            .unwrap();
        let lock = manager.multicast_lock.clone();

        drop(manager);
        assert_eq!(
            lock.held_generation
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
        drop(lease);
    }

    fn cached_device(usn: &str, location: &str) -> CachedDevice {
        let device_ip: std::net::Ipv4Addr = Url::parse(location)
            .unwrap()
            .host_str()
            .unwrap()
            .parse()
            .unwrap();
        CachedDevice {
            public: DlnaDevice {
                usn: usn.to_string(),
                name: usn.to_string(),
                location: location.to_string(),
                address: device_ip.to_string(),
            },
            av_transport: ControlService {
                url: Url::parse(&format!("http://{device_ip}/avtransport")).unwrap(),
                service_type: "urn:schemas-upnp-org:service:AVTransport:1".to_string(),
            },
            rendering_control: ControlService {
                url: Url::parse(&format!("http://{device_ip}/rendering")).unwrap(),
                service_type: "urn:schemas-upnp-org:service:RenderingControl:1".to_string(),
            },
            device_ip,
        }
    }
}
