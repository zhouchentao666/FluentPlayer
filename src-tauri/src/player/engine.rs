use crate::player::effects::{
    AtomicF64, BalanceSource, EqSettings, EqSource, EqState, NUM_EQ_BANDS,
};
use crate::player::spectrum::{PositionSource, PositionState, SpectrumSource, SpectrumState};
use crate::player::{AudioSlot, PlaybackState, PlayerSnapshot, SharedPlayer};
use base64::Engine;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::VecDeque;
use std::io::{BufReader, Read as IoRead, Seek as IoSeek, Write as IoWrite};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::Emitter;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder as SymphoniaDecoderTrait, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub struct SlotPipeline {
    sink: Sink,
    url: String,
    eq_state: Arc<EqState>,
    spectrum_state: Arc<SpectrumState>,
    position_state: Arc<PositionState>,
    balance: Arc<AtomicF64>,
    #[allow(dead_code)]
    temp_file: Option<PathBuf>,
}

impl SlotPipeline {
    pub fn apply_eq_settings(&self, settings: &EqSettings) {
        self.eq_state.set_settings(settings);
    }

    /// 从已解析的文件路径创建管线
    pub fn from_file(
        stream_handle: &OutputStreamHandle,
        file_path: &std::path::Path,
        url: &str,
        volume: f64,
    ) -> Result<Self, String> {
        // AAC/M4A (ISO BMFF / ftyp) 直接走 symphonia，避免 rodio symphonia 解码器的 seek panic
        // 其他格式先尝试 rodio，失败再回退 symphonia
        let needs_symphonia_direct = {
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(ext.as_str(), "m4a" | "aac" | "mp4" | "m4b" | "m4p") {
                true
            } else {
                // 无扩展名时检测魔数：ISO BMFF 容器以 ftyp box 开头
                std::fs::read(&file_path)
                    .map(|buf| buf.len() >= 8 && &buf[4..8] == b"ftyp")
                    .unwrap_or(false)
            }
        };

        type StreamSource = Box<dyn Source<Item = f32> + Send>;

        let (channels, sample_rate, duration, source): (u16, u32, f64, StreamSource) =
            if needs_symphonia_direct {
                decode_streaming_symphonia(&file_path)?
            } else {
                match decode_streaming_rodio(&file_path) {
                    Ok(result) => result,
                    Err(_) => decode_streaming_symphonia(&file_path)?,
                }
            };

        let eq_state = Arc::new(EqState::new(channels, sample_rate));
        let spectrum_state = Arc::new(SpectrumState::new());
        let position_state = Arc::new(PositionState::new());
        let balance = Arc::new(AtomicF64::new(0.0));

        position_state.set_initial(sample_rate, channels, duration);

        // 管线: Decoder → EQ → Balance → Spectrum → Position
        let source = EqSource::new(source, eq_state.clone());
        let source = BalanceSource::new(source, balance.clone());
        let source = SpectrumSource::new(source, spectrum_state.clone());
        let source = PositionSource::new(source, position_state.clone());

        let sink = Sink::try_new(stream_handle).map_err(|e| format!("创建 Sink 失败: {e}"))?;
        sink.set_volume(volume as f32);
        sink.append(source);

        Ok(Self {
            sink,
            url: url.to_string(),
            eq_state,
            spectrum_state,
            position_state,
            balance,
            temp_file: None,
        })
    }

    pub fn pause(&self) {
        self.sink.pause()
    }
    pub fn resume(&self) {
        self.sink.play()
    }
    pub fn seek(&self, pos: Duration) -> bool {
        self.sink.try_seek(pos).is_ok()
    }
    pub fn set_volume(&self, vol: f64) {
        self.sink.set_volume(vol as f32)
    }
    pub fn position(&self) -> f64 {
        self.position_state.position_secs()
    }
    pub fn duration(&self) -> f64 {
        self.position_state.duration_secs()
    }
    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused() && !self.sink.empty()
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn take_spectrum(&self) -> Option<Vec<f64>> {
        self.spectrum_state.take_spectrum()
    }

    /// 创建管线后立即暂停（用于预加载）
    pub fn from_file_paused(
        stream_handle: &OutputStreamHandle,
        file_path: &std::path::Path,
        url: &str,
    ) -> Result<Self, String> {
        let pipeline = Self::from_file(stream_handle, file_path, url, 0.0)?;
        pipeline.sink.pause();
        Ok(pipeline)
    }

    /// Create pipeline from a spool file being downloaded in background.
    fn from_spool(
        stream_handle: &OutputStreamHandle,
        path: &std::path::Path,
        state: SharedDownloadState,
        url: &str,
        volume: f64,
    ) -> Result<Self, String> {
        let needs_symphonia = {
            let initial = std::fs::read(path)
                .map(|buf| buf.len() >= 8 && &buf[4..8] == b"ftyp")
                .unwrap_or(false);
            initial
        };

        type StreamSource = Box<dyn Source<Item = f32> + Send>;

        let (channels, sample_rate, duration, source): (u16, u32, f64, StreamSource) =
            if needs_symphonia {
                let file =
                    std::fs::File::open(path).map_err(|e| format!("打开音频文件失败: {e}"))?;
                let spool = SpoolReader::new(file, state.clone());
                decode_reader_symphonia(
                    Box::new(spool) as Box<dyn symphonia::core::io::MediaSource>,
                    None,
                )?
            } else {
                let file =
                    std::fs::File::open(path).map_err(|e| format!("打开音频文件失败: {e}"))?;
                let spool = SpoolReader::new(file, state.clone());
                match decode_reader_rodio(spool) {
                    Ok(result) => result,
                    Err(_) => {
                        let file = std::fs::File::open(path)
                            .map_err(|e| format!("打开音频文件失败: {e}"))?;
                        let spool = SpoolReader::new(file, state);
                        decode_reader_symphonia(
                            Box::new(spool) as Box<dyn symphonia::core::io::MediaSource>,
                            None,
                        )?
                    }
                }
            };

        let eq_state = Arc::new(EqState::new(channels, sample_rate));
        let spectrum_state = Arc::new(SpectrumState::new());
        let position_state = Arc::new(PositionState::new());
        let balance = Arc::new(AtomicF64::new(0.0));

        position_state.set_initial(sample_rate, channels, duration);

        let source = EqSource::new(source, eq_state.clone());
        let source = BalanceSource::new(source, balance.clone());
        let source = SpectrumSource::new(source, spectrum_state.clone());
        let source = PositionSource::new(source, position_state.clone());

        let sink = Sink::try_new(stream_handle).map_err(|e| format!("创建 Sink 失败: {e}"))?;
        sink.set_volume(volume as f32);
        sink.append(source);

        Ok(Self {
            sink,
            url: url.to_string(),
            eq_state,
            spectrum_state,
            position_state,
            balance,
            temp_file: Some(path.to_path_buf()),
        })
    }
}

// ==================== 流式下载：SpoolReader ====================

struct DownloadState {
    written: u64,
    total: Option<u64>,
    done: bool,
    error: Option<String>,
}

type SharedDownloadState = Arc<(Mutex<DownloadState>, Condvar)>;

struct SpoolReader {
    file: std::fs::File,
    state: SharedDownloadState,
}

impl SpoolReader {
    fn new(file: std::fs::File, state: SharedDownloadState) -> Self {
        Self { file, state }
    }
}

impl IoRead for SpoolReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let (lock, cvar) = &*self.state;
            let mut s = lock.lock().unwrap();
            let pos = self.file.stream_position()?;

            if pos < s.written {
                let available = (s.written - pos) as usize;
                let to_read = buf.len().min(available);
                drop(s);
                return self.file.read(&mut buf[..to_read]);
            }

            if s.done {
                if let Some(ref err) = s.error {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err.clone()));
                }
                return Ok(0);
            }

            s = cvar.wait(s).unwrap();
        }
    }
}

impl IoSeek for SpoolReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        loop {
            let new_pos = match pos {
                std::io::SeekFrom::Start(offset) => offset,
                std::io::SeekFrom::Current(offset) => {
                    let current = self.file.stream_position()?;
                    (current as i64 + offset).max(0) as u64
                }
                std::io::SeekFrom::End(offset) => {
                    let (lock, _) = &*self.state;
                    let s = lock.lock().unwrap();
                    let end = s.total.unwrap_or(s.written);
                    drop(s);
                    (end as i64 + offset).max(0) as u64
                }
            };

            let (lock, cvar) = &*self.state;
            let mut s = lock.lock().unwrap();

            if new_pos <= s.written || s.done {
                drop(s);
                return self.file.seek(std::io::SeekFrom::Start(new_pos));
            }

            s = cvar.wait(s).unwrap();
        }
    }
}

unsafe impl Send for SpoolReader {}
unsafe impl Sync for SpoolReader {}

impl symphonia::core::io::MediaSource for SpoolReader {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        let (lock, _) = &*self.state;
        let s = lock.lock().unwrap();
        s.total
    }
}

// ==================== URL 解析 ====================

fn resolve_audio_file(url: &str) -> Result<PathBuf, String> {
    if url.starts_with("file://") {
        return Ok(PathBuf::from(&url[7..]));
    }
    if url.starts_with('/') || url.starts_with('~') {
        return Ok(PathBuf::from(url));
    }
    if url.starts_with("data:") {
        return data_uri_to_temp(url);
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return download_to_temp(url);
    }
    Ok(PathBuf::from(url))
}

/// Resolve audio file with cache support.
/// For HTTP URLs with cache_key: check cache first, download and cache on miss.
fn resolve_audio_file_cached(
    url: &str,
    cache_dir: Option<&str>,
    cache_key: Option<&str>,
    max_size: u64,
) -> Result<PathBuf, String> {
    // Only cache HTTP(S) URLs when both cache_dir and cache_key are provided
    if url.starts_with("http://") || url.starts_with("https://") {
        if let (Some(dir), Some(key)) = (cache_dir, cache_key) {
            let dir_path = std::path::Path::new(dir);
            if let Some(cached) = crate::player::cache::lookup(dir_path, key) {
                return Ok(cached);
            }
            // Cache miss: download
            let temp = download_to_temp(url)?;
            if let Ok(cached) = crate::player::cache::insert(dir_path, key, &temp, max_size) {
                return Ok(cached);
            }
            return Ok(temp);
        }
    }
    resolve_audio_file(url)
}

fn download_to_temp(url: &str) -> Result<PathBuf, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // 从 URL 域名派生 Referer，酷我/QQ 等 CDN 需要此头才能正常返回音频
    let referer = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| {
            let host = u.host_str()?;
            let parts: Vec<&str> = host.split('.').collect();
            if parts.len() >= 2 {
                Some(format!(
                    "http://{}.{}",
                    parts[parts.len() - 2],
                    parts[parts.len() - 1]
                ))
            } else {
                Some(format!("http://{}/", host))
            }
        })
        .unwrap_or_else(|| "http://localhost/".to_string());

    let resp = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .header("Referer", &referer)
        .send()
        .map_err(|e| format!("下载音频失败: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("下载音频失败: HTTP {}", status));
    }

    let bytes = resp.bytes().map_err(|e| format!("读取音频数据失败: {e}"))?;

    let temp_dir = std::env::temp_dir().join("mio_player");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

    let path = temp_dir.join(format!("audio_{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;

    Ok(path)
}

/// Stream audio to a temp file: buffer initial data, return immediately, continue in background.
fn stream_to_temp(url: &str) -> Result<(PathBuf, SharedDownloadState), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let referer = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| {
            let host = u.host_str()?;
            let parts: Vec<&str> = host.split('.').collect();
            if parts.len() >= 2 {
                Some(format!(
                    "http://{}.{}",
                    parts[parts.len() - 2],
                    parts[parts.len() - 1]
                ))
            } else {
                Some(format!("http://{}/", host))
            }
        })
        .unwrap_or_else(|| "http://localhost/".to_string());

    let resp = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
        )
        .header("Referer", &referer)
        .send()
        .map_err(|e| format!("下载音频失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("下载音频失败: HTTP {}", resp.status()));
    }

    let total = resp.content_length();
    let temp_dir = std::env::temp_dir().join("mio_player");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;
    let path = temp_dir.join(format!("stream_{}", uuid::Uuid::new_v4()));

    let mut file = std::fs::File::create(&path).map_err(|e| format!("创建临时文件失败: {e}"))?;

    let state = Arc::new((
        Mutex::new(DownloadState {
            written: 0,
            total,
            done: false,
            error: None,
        }),
        Condvar::new(),
    ));

    let initial_size: u64 = 512 * 1024;
    let mut resp = resp;
    let mut buf = [0u8; 32768];
    let mut downloaded: u64 = 0;

    // Synchronously buffer initial data
    while downloaded < initial_size {
        use std::io::Read;
        let n = resp
            .read(&mut buf)
            .map_err(|e| format!("下载音频失败: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        downloaded += n as u64;
    }

    file.sync_all().map_err(|e| format!("同步文件失败: {e}"))?;

    // Update state with initial data
    {
        let (lock, cvar) = &*state;
        let mut s = lock.lock().unwrap();
        s.written = downloaded;
        cvar.notify_all();
    }

    // Check if already fully downloaded
    let fully_downloaded = total.map_or(false, |t| downloaded >= t) || downloaded == 0;

    if !fully_downloaded {
        let bg_state = state.clone();
        let bg_path = path.clone();
        std::thread::spawn(move || {
            let mut bg_file = match std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&bg_path)
            {
                Ok(f) => f,
                Err(_) => {
                    let (lock, cvar) = &*bg_state;
                    let mut s = lock.lock().unwrap();
                    s.done = true;
                    s.error = Some("无法打开临时文件".into());
                    cvar.notify_all();
                    return;
                }
            };

            let mut bg_buf = [0u8; 32768];
            use std::io::Read;
            loop {
                match resp.read(&mut bg_buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if bg_file.write_all(&bg_buf[..n]).is_err() {
                            break;
                        }
                        let (lock, cvar) = &*bg_state;
                        let mut s = lock.lock().unwrap();
                        s.written += n as u64;
                        cvar.notify_all();
                    }
                    Err(_) => break,
                }
            }

            let _ = bg_file.sync_all();
            let (lock, cvar) = &*bg_state;
            let mut s = lock.lock().unwrap();
            s.done = true;
            cvar.notify_all();
        });
    } else {
        let (lock, cvar) = &*state;
        let mut s = lock.lock().unwrap();
        s.done = true;
        cvar.notify_all();
    }

    Ok((path, state))
}

fn data_uri_to_temp(url: &str) -> Result<PathBuf, String> {
    let parts: Vec<&str> = url.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err("无效的 data URI".into());
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(parts[1].trim())
        .map_err(|e| format!("解码 base64 失败: {e}"))?;

    let temp_dir = std::env::temp_dir().join("mio_player");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;

    let path = temp_dir.join(format!("data_{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, &bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;

    Ok(path)
}

// ==================== 流式解码：rodio 路径（MP3/WAV/FLAC/OGG） ====================

type StreamDecodeResult = (u16, u32, f64, Box<dyn Source<Item = f32> + Send>);

fn decode_reader_rodio<R: IoRead + IoSeek + Send + Sync + 'static>(
    reader: R,
) -> Result<StreamDecodeResult, String> {
    let buf_reader = BufReader::new(reader);
    let decoder = Decoder::new(buf_reader).map_err(|e| format!("rodio 解码失败: {e}"))?;

    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let duration_secs = decoder
        .total_duration()
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let source: Box<dyn Source<Item = f32> + Send> = Box::new(decoder.convert_samples());

    Ok((channels, sample_rate, duration_secs, source))
}

fn decode_streaming_rodio(file_path: &std::path::Path) -> Result<StreamDecodeResult, String> {
    let file = std::fs::File::open(file_path).map_err(|e| format!("打开音频文件失败: {e}"))?;
    decode_reader_rodio(file)
}

// ==================== 流式解码：symphonia 路径（AAC/M4A 等） ====================

/// 流式 symphonia 解码器，按需解码 packet 而非全量加载到内存
struct SymphoniaStreamSource {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaDecoderTrait>,
    track_id: u32,
    channels: u16,
    sample_rate: u32,
    duration_secs: f64,
    sample_buffer: Vec<f32>,
    buffer_pos: usize,
    eof: bool,
}

// Safety: IsoMp4Reader, AacDecoder 等具体实现都是 Send 的
// FormatReader 和 Decoder trait 本身没有 Send bound，但实际使用的实现都是线程安全的
unsafe impl Send for SymphoniaStreamSource {}

impl Iterator for SymphoniaStreamSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        // 从缓冲区返回下一个采样
        if self.buffer_pos < self.sample_buffer.len() {
            let sample = self.sample_buffer[self.buffer_pos];
            self.buffer_pos += 1;
            return Some(sample);
        }

        // 缓冲区耗尽且已到 EOF
        if self.eof {
            return None;
        }

        // 解码下一个 packet 填充缓冲区
        loop {
            match self.format.next_packet() {
                Ok(packet) => {
                    if packet.track_id() != self.track_id {
                        continue;
                    }
                    match self.decoder.decode(&packet) {
                        Ok(decoded) => {
                            let spec = *decoded.spec();
                            let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                            buf.copy_interleaved_ref(decoded);
                            self.sample_buffer = buf.samples().to_vec();
                            self.buffer_pos = 0;
                            if self.sample_buffer.is_empty() {
                                continue;
                            }
                            return self.next();
                        }
                        Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                        Err(_) => {
                            self.eof = true;
                            return None;
                        }
                    }
                }
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.eof = true;
                    return None;
                }
                Err(symphonia::core::errors::Error::IoError(_)) => {
                    self.eof = true;
                    return None;
                }
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.eof {
            (0, Some(0))
        } else {
            (0, None)
        }
    }
}

impl Source for SymphoniaStreamSource {
    fn current_frame_len(&self) -> Option<usize> {
        // 返回当前缓冲区中剩余的采样数
        Some(self.sample_buffer.len().saturating_sub(self.buffer_pos))
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        if self.duration_secs > 0.0 {
            Some(Duration::from_secs_f64(self.duration_secs))
        } else {
            None
        }
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let seek_to = SeekTo::Time {
            time: Time::from(pos.as_secs_f64()),
            track_id: Some(self.track_id),
        };
        self.format
            .seek(symphonia::core::formats::SeekMode::Accurate, seek_to)
            .map_err(|_| rodio::source::SeekError::NotSupported {
                underlying_source: "SymphoniaStreamSource",
            })?;
        // seek 后清空缓冲区
        self.sample_buffer.clear();
        self.buffer_pos = 0;
        Ok(())
    }
}

fn decode_reader_symphonia(
    source: Box<dyn symphonia::core::io::MediaSource>,
    hint_ext: Option<&str>,
) -> Result<StreamDecodeResult, String> {
    let mss = MediaSourceStream::new(source, Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = hint_ext {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("探测音频格式失败: {e}"))?;

    let format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "音频文件无默认音轨".to_string())?
        .to_owned();

    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    // 从元数据计算时长，无需全量解码
    let duration_secs = if let Some(tb) = track.codec_params.time_base {
        track
            .codec_params
            .n_frames
            .map(|n| n as f64 * tb.numer as f64 / tb.denom as f64)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let track_id = track.id;
    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("创建 symphonia 解码器失败: {e}"))?;

    let source = SymphoniaStreamSource {
        format,
        decoder,
        track_id,
        channels,
        sample_rate,
        duration_secs,
        sample_buffer: Vec::new(),
        buffer_pos: 0,
        eof: false,
    };

    let source: Box<dyn Source<Item = f32> + Send> = Box::new(source);

    Ok((channels, sample_rate, duration_secs, source))
}

fn decode_streaming_symphonia(file_path: &std::path::Path) -> Result<StreamDecodeResult, String> {
    let file = std::fs::File::open(file_path).map_err(|e| format!("打开音频文件失败: {e}"))?;
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());
    decode_reader_symphonia(
        Box::new(file) as Box<dyn symphonia::core::io::MediaSource>,
        ext.as_deref(),
    )
}

// ==================== 交叉淡入淡出 ====================

struct CrossfadeState {
    active: bool,
    start: Instant,
    duration_ms: u64,
}

fn cancel_crossfade(crossfade: &mut Option<CrossfadeState>) -> bool {
    crossfade.take().is_some()
}

fn clear_slot<T>(slot: AudioSlot, primary: &mut Option<T>, secondary: &mut Option<T>) {
    match slot {
        AudioSlot::A => *primary = None,
        AudioSlot::B => *secondary = None,
    }
}

// ==================== 预加载/异步播放结果 ====================

struct AsyncPipelineResult {
    generation: u64,
    outcome: AsyncPipelineOutcome,
}

enum AsyncPipelineOutcome {
    Ready(SlotPipeline),
    Error(String),
}

#[derive(Default)]
struct PipelineGenerations {
    play: u64,
    preload: u64,
}

impl PipelineGenerations {
    fn next_play(&mut self) -> u64 {
        self.play = self.play.wrapping_add(1);
        self.play
    }

    fn next_preload(&mut self) -> u64 {
        self.preload = self.preload.wrapping_add(1);
        self.preload
    }

    fn invalidate_slot(&mut self, slot: AudioSlot) {
        match slot {
            AudioSlot::A => {
                self.next_play();
            }
            AudioSlot::B => {
                self.next_preload();
            }
        }
    }
}

fn is_current_generation(result_generation: u64, current_generation: u64) -> bool {
    result_generation == current_generation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_stale_play_result() {
        assert!(!is_current_generation(1, 2));
        assert!(is_current_generation(2, 2));
    }

    #[test]
    fn play_and_preload_generations_are_independent() {
        let mut generations = PipelineGenerations::default();
        let play = generations.next_play();
        let preload = generations.next_preload();

        assert_eq!(play, 1);
        assert_eq!(preload, 1);
    }

    #[test]
    fn stopping_secondary_invalidates_only_preload_generation() {
        let mut generations = PipelineGenerations::default();
        let play = generations.next_play();
        let preload = generations.next_preload();

        generations.invalidate_slot(AudioSlot::B);

        assert_eq!(generations.play, play);
        assert_eq!(generations.preload, preload + 1);
    }

    #[test]
    fn cancel_crossfade_removes_pending_swap() {
        let mut crossfade = Some(CrossfadeState {
            active: true,
            start: Instant::now(),
            duration_ms: 1,
        });

        assert!(cancel_crossfade(&mut crossfade));

        assert!(crossfade.is_none());
    }

    #[test]
    fn stopping_secondary_preserves_primary() {
        let mut primary = Some("primary");
        let mut secondary = Some("secondary");

        clear_slot(AudioSlot::B, &mut primary, &mut secondary);

        assert_eq!(primary, Some("primary"));
        assert_eq!(secondary, None);
    }

    #[test]
    fn snapshot_fields_serialize_as_camel_case() {
        let snapshot = PlayerSnapshot {
            state: PlaybackState::Stopped,
            position: 0.0,
            duration: 0.0,
            volume: 80.0,
            primary_slot: AudioSlot::A,
            url: String::new(),
            is_playing: false,
        };
        let value = serde_json::to_value(snapshot).expect("snapshot should serialize");

        assert!(value.get("primarySlot").is_some());
        assert!(value.get("isPlaying").is_some());
        assert!(value.get("primary_slot").is_none());
        assert!(value.get("is_playing").is_none());
    }
}

// ==================== 播放引擎 ====================

pub struct PlayerEngine {
    stream_handle: Option<OutputStreamHandle>,
    primary: Option<SlotPipeline>,
    secondary: Option<SlotPipeline>,
    volume: f64,
    shutdown: bool,
    crossfade: Option<CrossfadeState>,
    #[allow(dead_code)]
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
    app_handle: tauri::AppHandle,
    poll_tick: u32,
    // 异步播放通道
    play_tx: std::sync::mpsc::Sender<AsyncPipelineResult>,
    play_rx: std::sync::mpsc::Receiver<AsyncPipelineResult>,
    // 预加载通道
    preload_tx: std::sync::mpsc::Sender<AsyncPipelineResult>,
    preload_rx: std::sync::mpsc::Receiver<AsyncPipelineResult>,
    pipeline_generations: PipelineGenerations,
    // 无缝换曲配置
    crossfade_duration_ms: u64,
    auto_crossfade_enabled: bool,
    // player:ended 防重复
    ended_emitted: bool,
    // 音频缓存配置
    cache_dir: Option<String>,
    cache_max_size: u64,
    // 全局 EQ 快照：新建播放管线、预加载管线、seek 重建管线后都立即应用。
    eq_settings: EqSettings,
    native_queue: VecDeque<(String, Option<String>)>,
}

impl PlayerEngine {
    pub fn new(
        app_handle: tauri::AppHandle,
        stream_handle: Option<OutputStreamHandle>,
        shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
    ) -> Self {
        let (play_tx, play_rx) = std::sync::mpsc::channel();
        let (preload_tx, preload_rx) = std::sync::mpsc::channel();
        Self {
            stream_handle,
            primary: None,
            secondary: None,
            volume: 80.0,
            shutdown: false,
            crossfade: None,
            shutdown_tx,
            app_handle,
            poll_tick: 0,
            play_tx,
            play_rx,
            preload_tx,
            preload_rx,
            pipeline_generations: PipelineGenerations::default(),
            crossfade_duration_ms: 3000,
            auto_crossfade_enabled: false,
            ended_emitted: false,
            cache_dir: None,
            cache_max_size: 1073741824, // 1GB default
            eq_settings: EqSettings::default(),
            native_queue: VecDeque::new(),
        }
    }

    /// Try to (re-)initialize audio output if not already available.
    fn ensure_stream_handle(&mut self) -> Result<(), String> {
        if self.stream_handle.is_some() {
            return Ok(());
        }
        match create_output_stream() {
            Ok((handle, shutdown_tx)) => {
                self.stream_handle = Some(handle);
                self.shutdown_tx = Some(shutdown_tx);
                Ok(())
            }
            Err(e) => Err(format!("音频输出不可用: {e}")),
        }
    }

    /// 异步播放：在后台线程下载 + 解码，完成后自动赋值 primary 并发出事件
    pub fn play_async(
        &mut self,
        url: &str,
        _slot: Option<AudioSlot>,
        _cache_key: Option<String>,
    ) -> Result<(), String> {
        let generation = self.pipeline_generations.next_play();
        self.primary = None;
        self.clear_secondary();
        self.crossfade = None;
        self.ended_emitted = false;

        self.ensure_stream_handle()?;
        let stream_handle = self.stream_handle.clone().unwrap();
        let url = url.to_string();
        let volume = self.volume / 100.0;
        let eq_settings = self.eq_settings.clone();
        let tx = self.play_tx.clone();

        let is_http = url.starts_with("http://") || url.starts_with("https://");

        std::thread::spawn(move || {
            let outcome = if is_http {
                match stream_to_temp(&url) {
                    Ok((path, state)) => {
                        match SlotPipeline::from_spool(&stream_handle, &path, state, &url, volume) {
                            Ok(pipeline) => {
                                pipeline.apply_eq_settings(&eq_settings);
                                AsyncPipelineOutcome::Ready(pipeline)
                            }
                            Err(e) => AsyncPipelineOutcome::Error(e),
                        }
                    }
                    Err(e) => AsyncPipelineOutcome::Error(e),
                }
            } else {
                match resolve_audio_file(&url) {
                    Ok(path) => {
                        match SlotPipeline::from_file(&stream_handle, &path, &url, volume) {
                            Ok(pipeline) => {
                                pipeline.apply_eq_settings(&eq_settings);
                                AsyncPipelineOutcome::Ready(pipeline)
                            }
                            Err(e) => AsyncPipelineOutcome::Error(e),
                        }
                    }
                    Err(e) => AsyncPipelineOutcome::Error(e),
                }
            };
            let _ = tx.send(AsyncPipelineResult {
                generation,
                outcome,
            });
        });
        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(ref p) = self.primary {
            p.pause()
        }
        self.emit_state();
    }

    pub fn resume(&mut self) {
        if let Some(ref p) = self.primary {
            p.resume()
        }
        self.emit_state();
    }

    pub fn stop_slot(&mut self, slot: AudioSlot) {
        clear_slot(slot, &mut self.primary, &mut self.secondary);
        self.pipeline_generations.invalidate_slot(slot);
        self.cancel_active_crossfade();
        self.emit_state();
    }

    pub fn stop_all(&mut self) {
        self.primary = None;
        self.pipeline_generations.next_play();
        self.clear_secondary();
        self.emit_state();
    }

    pub fn seek(&mut self, position_secs: f64) {
        let pos = Duration::from_secs_f64(position_secs);

        // 先尝试直接 seek
        if let Some(ref p) = self.primary {
            if p.seek(pos) {
                return;
            }
        }

        // try_seek 失败（如 rodio FLAC 解码器），使用 symphonia 重建管线
        let url = match self.primary.as_ref() {
            Some(p) => p.url().to_string(),
            None => return,
        };
        let was_playing = self
            .primary
            .as_ref()
            .map(|p| p.is_playing())
            .unwrap_or(false);
        let volume = self.volume / 100.0;

        // 销毁旧管线
        self.primary = None;

        let file_path = match resolve_audio_file(&url) {
            Ok(path) => path,
            Err(_) => return,
        };

        // 强制使用 symphonia 解码器（支持 seek）
        if self.ensure_stream_handle().is_err() {
            return;
        }
        let handle = self.stream_handle.as_ref().unwrap();
        let new_pipeline = match Self::build_pipeline_symphonia(handle, &file_path, &url, volume) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[PlayerEngine] symphonia rebuild failed: {}", e);
                return;
            }
        };

        // 在新管线上应用全局 EQ 快照并 seek
        new_pipeline.apply_eq_settings(&self.eq_settings);
        let _ = new_pipeline.sink.try_seek(pos);
        if !was_playing {
            new_pipeline.sink.pause();
        }
        self.primary = Some(new_pipeline);
        self.emit_state();
    }

    /// 使用 symphonia 解码器构建管线（确保支持 seek）
    fn build_pipeline_symphonia(
        stream_handle: &OutputStreamHandle,
        file_path: &std::path::Path,
        url: &str,
        volume: f64,
    ) -> Result<SlotPipeline, String> {
        let (channels, sample_rate, duration, source) = decode_streaming_symphonia(file_path)?;

        let eq_state = Arc::new(EqState::new(channels, sample_rate));
        let spectrum_state = Arc::new(SpectrumState::new());
        let position_state = Arc::new(PositionState::new());
        let balance = Arc::new(AtomicF64::new(0.0));

        position_state.set_initial(sample_rate, channels, duration);

        let source = EqSource::new(source, eq_state.clone());
        let source = BalanceSource::new(source, balance.clone());
        let source = SpectrumSource::new(source, spectrum_state.clone());
        let source = PositionSource::new(source, position_state.clone());

        let sink = Sink::try_new(stream_handle).map_err(|e| format!("创建 Sink 失败: {e}"))?;
        sink.set_volume(volume as f32);
        sink.append(source);

        Ok(SlotPipeline {
            sink,
            url: url.to_string(),
            eq_state,
            spectrum_state,
            position_state,
            balance,
            temp_file: None,
        })
    }

    pub fn set_volume(&mut self, vol: f64) {
        self.volume = vol.clamp(0.0, 100.0);
        let n = self.volume / 100.0;
        if let Some(ref p) = self.primary {
            p.set_volume(n)
        }
        if let Some(ref s) = self.secondary {
            s.set_volume(n)
        }
    }

    #[allow(dead_code)]
    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn crossfade_to(&mut self, url: &str, duration_ms: u64) -> Result<(), String> {
        self.pipeline_generations.next_preload();
        let file_path = resolve_audio_file(url)?;
        self.ensure_stream_handle()?;
        let handle = self.stream_handle.as_ref().unwrap();
        let pipeline = SlotPipeline::from_file(&handle, &file_path, url, 0.0)?;
        pipeline.apply_eq_settings(&self.eq_settings);
        self.secondary = Some(pipeline);
        self.crossfade = Some(CrossfadeState {
            active: true,
            start: Instant::now(),
            duration_ms,
        });
        Ok(())
    }

    pub fn swap_primary(&mut self) {
        std::mem::swap(&mut self.primary, &mut self.secondary);
        self.emit_state();
    }

    pub fn set_eq_settings(&mut self, settings: EqSettings) {
        self.eq_settings = settings.sanitized();
        if let Some(ref p) = self.primary {
            p.apply_eq_settings(&self.eq_settings)
        }
        if let Some(ref s) = self.secondary {
            s.apply_eq_settings(&self.eq_settings)
        }
    }

    pub fn set_eq_band(&mut self, index: usize, gain: f64) {
        if index >= NUM_EQ_BANDS {
            return;
        }
        self.eq_settings.bands[index].gain = gain;
        self.set_eq_settings(self.eq_settings.clone());
    }

    pub fn get_eq_bands(&self) -> Vec<f64> {
        self.eq_settings.effective_bands().to_vec()
    }

    pub fn get_eq_global_gain(&self) -> f64 {
        self.eq_settings.effective_global_gain()
    }

    pub fn set_balance(&self, value: f64) {
        if let Some(ref p) = self.primary {
            p.balance.store(value)
        }
        if let Some(ref s) = self.secondary {
            s.balance.store(value)
        }
    }

    /// 异步预加载：在后台线程下载 + 解码，完成后通过 channel 送回
    pub fn preload(&mut self, url: String, cache_key: Option<String>) {
        self.clear_secondary();
        self.start_preload_job(url, cache_key);
    }

    fn start_preload_job(&mut self, url: String, cache_key: Option<String>) {
        let generation = self.pipeline_generations.next_preload();
        let stream_handle = match self.ensure_stream_handle() {
            Ok(()) => self.stream_handle.clone().unwrap(),
            Err(_) => return,
        };
        let tx = self.preload_tx.clone();
        let cache_dir = self.cache_dir.clone();
        let cache_max_size = self.cache_max_size;
        let eq_settings = self.eq_settings.clone();

        std::thread::spawn(move || {
            let file_path = resolve_audio_file_cached(
                &url,
                cache_dir.as_deref(),
                cache_key.as_deref(),
                cache_max_size,
            );
            let outcome = match file_path {
                Ok(path) => match SlotPipeline::from_file_paused(&stream_handle, &path, &url) {
                    Ok(pipeline) => {
                        pipeline.apply_eq_settings(&eq_settings);
                        AsyncPipelineOutcome::Ready(pipeline)
                    }
                    Err(e) => AsyncPipelineOutcome::Error(e),
                },
                Err(e) => AsyncPipelineOutcome::Error(e),
            };
            let _ = tx.send(AsyncPipelineResult {
                generation,
                outcome,
            });
        });
    }

    pub fn set_native_queue(&mut self, items: Vec<(String, Option<String>)>) {
        self.native_queue = VecDeque::from(items);
        if self.secondary.is_none() {
            self.preload_next_from_queue();
        }
    }

    fn preload_next_from_queue(&mut self) {
        if self.secondary.is_some() {
            return;
        }
        if let Some((url, cache_key)) = self.native_queue.pop_front() {
            self.start_preload_job(url, cache_key);
        }
    }

    /// Gapless 即时切换：将预加载的 secondary 提升为 primary 并开始播放
    pub fn gapless_swap(&mut self) -> bool {
        if let Some(secondary) = self.secondary.take() {
            let vol = self.volume / 100.0;
            self.primary = Some(secondary);
            if let Some(ref p) = self.primary {
                p.set_volume(vol);
                p.resume();
            }
            self.ended_emitted = false;
            self.emit_state();
            return true;
        }
        false
    }

    /// 从已预加载的 secondary 启动 crossfade 渐变
    fn start_crossfade_from_preloaded(&mut self) {
        if self.secondary.is_none() {
            return;
        }
        if let Some(ref s) = self.secondary {
            s.set_volume(0.0);
            s.resume();
        }
        self.crossfade = Some(CrossfadeState {
            active: true,
            start: Instant::now(),
            duration_ms: self.crossfade_duration_ms,
        });
    }

    /// 清除 secondary slot，释放内存
    pub fn clear_secondary(&mut self) {
        self.secondary = None;
        self.pipeline_generations.next_preload();
        self.cancel_active_crossfade();
    }

    fn cancel_active_crossfade(&mut self) {
        if !cancel_crossfade(&mut self.crossfade) {
            return;
        }
        if let Some(ref primary) = self.primary {
            primary.set_volume(self.volume / 100.0);
        }
        if let Some(ref secondary) = self.secondary {
            secondary.pause();
            secondary.set_volume(0.0);
        }
    }

    /// 配置无缝换曲参数
    pub fn set_seamless_config(&mut self, auto_crossfade: bool, duration_ms: u64) {
        self.auto_crossfade_enabled = auto_crossfade;
        self.crossfade_duration_ms = duration_ms.max(500);
    }

    /// 配置音频缓存
    pub fn set_cache_config(&mut self, cache_dir: Option<String>, max_size: u64) {
        self.cache_dir = cache_dir;
        self.cache_max_size = max_size;
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        let p = self.primary.as_ref();
        let state = p
            .map(|p| {
                if p.is_playing() {
                    PlaybackState::Playing
                } else {
                    PlaybackState::Paused
                }
            })
            .unwrap_or(PlaybackState::Stopped);

        PlayerSnapshot {
            state,
            position: p.map(|p| p.position()).unwrap_or(0.0),
            duration: p.map(|p| p.duration()).unwrap_or(0.0),
            volume: self.volume,
            primary_slot: AudioSlot::A,
            url: p.map(|p| p.url().to_string()).unwrap_or_default(),
            is_playing: state == PlaybackState::Playing,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }
    pub fn shutdown(&mut self) {
        self.shutdown = true;
        self.stop_all()
    }

    pub fn poll(&mut self) {
        if self.shutdown {
            return;
        }
        self.poll_tick = self.poll_tick.wrapping_add(1);

        // 排空异步播放结果，只接受当前播放请求的结果。
        while let Ok(result) = self.play_rx.try_recv() {
            if !is_current_generation(result.generation, self.pipeline_generations.play) {
                continue;
            }

            match result.outcome {
                AsyncPipelineOutcome::Ready(pipeline) => {
                    pipeline.apply_eq_settings(&self.eq_settings);
                    self.primary = Some(pipeline);
                    self.ended_emitted = false;
                    self.emit_state();
                }
                AsyncPipelineOutcome::Error(e) => {
                    let _ = self
                        .app_handle
                        .emit("player:error", serde_json::json!({ "error": e }));
                }
            }
        }

        // 排空预加载结果，只接受当前预加载请求的结果。
        while let Ok(result) = self.preload_rx.try_recv() {
            if !is_current_generation(result.generation, self.pipeline_generations.preload) {
                continue;
            }

            match result.outcome {
                AsyncPipelineOutcome::Ready(pipeline) => {
                    pipeline.apply_eq_settings(&self.eq_settings);
                    self.secondary = Some(pipeline);
                    let _ = self
                        .app_handle
                        .emit("player:preload_ready", serde_json::json!({}));
                }
                AsyncPipelineOutcome::Error(e) => {
                    eprintln!("预加载失败: {e}");
                }
            }
        }

        self.tick_crossfade();

        if let Some(ref p) = self.primary {
            // 频谱：每 3 tick 发一次（~30Hz poll / 3 ≈ 10FPS）
            if self.poll_tick % 3 == 0 {
                if let Some(bands) = p.take_spectrum() {
                    let _ = self
                        .app_handle
                        .emit("player:spectrum", serde_json::json!({ "bands": bands }));
                }
            }

            // 时间：每 6 tick 发一次（~5Hz，200ms 足够 UI 更新）
            if self.poll_tick % 6 == 0 && p.is_playing() {
                let pos = p.position();
                let dur = p.duration();
                let _ = self.app_handle.emit(
                    "player:time",
                    serde_json::json!({
                        "position": pos, "duration": dur,
                    }),
                );
                crate::player::media_control::MEDIA_CONTROL
                    .lock()
                    .update_playback_position(pos, dur);
            }
        }

        // 自动 crossfade：提取 primary 状态后调用 mutable 方法
        let should_crossfade = {
            let p = self.primary.as_ref();
            p.map_or(false, |p| {
                if !self.auto_crossfade_enabled
                    || self.crossfade.is_some()
                    || self.secondary.is_none()
                {
                    return false;
                }
                let pos = p.position();
                let dur = p.duration();
                let threshold = self.crossfade_duration_ms as f64 / 1000.0;
                dur > 0.0 && pos > 0.0 && dur - pos <= threshold && dur > threshold
            })
        };
        if should_crossfade {
            self.start_crossfade_from_preloaded();
        }

        let ended = self.primary.as_ref().map_or(false, |p| {
            p.sink.empty() && !p.sink.is_paused() && self.crossfade.is_none() && !self.ended_emitted
        });

        if ended {
            if self.secondary.is_some() {
                // Auto-advance to preloaded secondary (does not need WebView JS)
                let vol = self.volume / 100.0;
                self.primary = self.secondary.take();
                if let Some(ref new_p) = self.primary {
                    new_p.set_volume(vol);
                    new_p.resume();
                }
                self.ended_emitted = false;
                self.emit_state();
                self.preload_next_from_queue();
                let _ = self
                    .app_handle
                    .emit("player:auto_advanced", serde_json::json!({}));
            } else {
                self.ended_emitted = true;
                let _ = self
                    .app_handle
                    .emit("player:ended", serde_json::json!({ "slot": "A" }));
            }
        }
    }

    fn tick_crossfade(&mut self) {
        let Some(ref mut cf) = self.crossfade else {
            return;
        };
        if !cf.active {
            return;
        }

        let mut done = false;
        let progress = (cf.start.elapsed().as_millis() as f64 / cf.duration_ms as f64).min(1.0);
        let vol = self.volume / 100.0;

        if let Some(ref p) = self.primary {
            p.set_volume(vol * (1.0 - progress))
        }
        if let Some(ref s) = self.secondary {
            s.set_volume(vol * progress)
        }

        if progress >= 1.0 {
            self.primary = self.secondary.take();
            self.ended_emitted = false;
            self.emit_snapshot("player:crossfade_swap");
            done = true;
        }

        if done {
            self.crossfade = None;
        }
    }

    fn emit_state(&self) {
        self.emit_snapshot("player:state");
    }

    fn emit_snapshot(&self, event: &str) {
        let s = self.snapshot();
        let _ = self.app_handle.emit(
            event,
            serde_json::json!({
                "state": format!("{:?}", s.state), "position": s.position, "duration": s.duration,
                "volume": s.volume, "url": s.url, "isPlaying": s.is_playing,
            }),
        );
    }
}

// ==================== 初始化：专用音频线程保持 OutputStream 存活 ====================

pub fn start_bus_poller(shared: SharedPlayer) {
    std::thread::spawn(move || loop {
        {
            let mut engine = shared.lock();
            if engine.is_shutdown() {
                break;
            }
            engine.poll();
        }
        std::thread::sleep(Duration::from_millis(33));
    });
}

pub fn create_output_stream() -> Result<(OutputStreamHandle, std::sync::mpsc::Sender<()>), String> {
    let (handle_tx, handle_rx) = std::sync::mpsc::channel();
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| OutputStream::try_default());
        match result {
            Ok(Ok((_stream, handle))) => {
                if handle_tx.send(Ok(handle)).is_err() {
                    return;
                }
                let _ = shutdown_rx.recv();
                drop(_stream);
            }
            Ok(Err(e)) => {
                let _ = handle_tx.send(Err(format!("{e}")));
            }
            Err(panic_val) => {
                let msg = if let Some(s) = panic_val.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_val.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "音频输出初始化 panic".to_string()
                };
                let _ = handle_tx.send(Err(msg));
            }
        }
    });

    let timeout = Duration::from_secs(5);
    let handle = handle_rx
        .recv_timeout(timeout)
        .map_err(|e| format!("音频输出初始化超时({}s): {e}", timeout.as_secs()))?
        .map_err(|e| format!("{e}"))?;
    Ok((handle, shutdown_tx))
}

/// 清理上次运行遗留的临时音频文件
pub fn cleanup_temp_files() {
    let temp_dir = std::env::temp_dir().join("mio_player");
    if !temp_dir.exists() {
        return;
    }
    let threshold = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    if let Ok(mtime) = meta.modified() {
                        if mtime < threshold {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}
