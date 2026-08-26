use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use parking_lot::Mutex;
use rodio::Source;
use rustfft::{FftPlanner, num_complex::Complex};

const FFT_SIZE: usize = 1024;
const OUTPUT_BANDS: usize = 128;

// ==================== Spectrum State ====================

struct SpectrumBuffer {
    data: Vec<f32>,
    write_pos: usize,
    ready: bool,
}

pub struct SpectrumState {
    buffer: Mutex<SpectrumBuffer>,
    fft: Arc<dyn rustfft::Fft<f64>>,
    window: Vec<f64>,
}

impl SpectrumState {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let window: Vec<f64> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / FFT_SIZE as f64).cos()))
            .collect();

        Self {
            buffer: Mutex::new(SpectrumBuffer {
                data: vec![0.0; FFT_SIZE],
                write_pos: 0,
                ready: false,
            }),
            fft,
            window,
        }
    }

    /// 提取频谱数据：锁内仅拷贝 4KB 数据，FFT 在锁外计算，
    /// 避免音频线程因 poller 持锁计算 FFT 而等待。
    pub fn take_spectrum(&self) -> Option<Vec<f64>> {
        let data = {
            let mut buf = self.buffer.lock();
            if !buf.ready { return None }
            buf.ready = false;
            buf.write_pos = 0;
            buf.data.clone()
        };

        let mut input: Vec<Complex<f64>> = data
            .iter()
            .zip(self.window.iter())
            .map(|(&s, &w)| Complex::new(s as f64 * w, 0.0))
            .collect();

        self.fft.process(&mut input);

        let magnitudes: Vec<f64> = (0..OUTPUT_BANDS)
            .map(|band| {
                let lo = self.band_start(band);
                let hi = self.band_start(band + 1).max(lo + 1);
                let mut max_sq = 0.0f64;
                for i in lo..hi {
                    if i + 1 >= input.len() { break }
                    let c = &input[i + 1];
                    let sq = c.re * c.re + c.im * c.im;
                    if sq > max_sq { max_sq = sq; }
                }
                10.0 * (max_sq / FFT_SIZE as f64 + 1e-10).log10()
            })
            .collect();

        Some(magnitudes)
    }

    fn band_start(&self, band: usize) -> usize {
        let max_bin = FFT_SIZE / 2 - 1;
        let t = band as f64 / OUTPUT_BANDS as f64;
        (t.powf(1.6) * max_bin as f64).round() as usize
    }
}

// ==================== Spectrum Source 包装器 ====================

pub struct SpectrumSource<S> {
    inner: S,
    state: Arc<SpectrumState>,
    sample_count: usize,
    channel_count: u16,
    stride: usize,
    stride_counter: usize,
}

impl<S: Source<Item = f32>> SpectrumSource<S> {
    pub fn new(inner: S, state: Arc<SpectrumState>) -> Self {
        let channels = inner.channels();
        let sr = inner.sample_rate();
        let target_fills_per_sec = 30;
        let mono_per_fill = sr as usize / target_fills_per_sec;
        let stride = if mono_per_fill > FFT_SIZE { mono_per_fill / FFT_SIZE } else { 1 };
        Self { inner, state, sample_count: 0, channel_count: channels, stride, stride_counter: 0 }
    }
}

impl<S> Iterator for SpectrumSource<S>
where S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.sample_count += 1;

        if (self.sample_count - 1) % self.channel_count as usize == 0 {
            self.stride_counter += 1;
            if self.stride_counter >= self.stride {
                self.stride_counter = 0;
                let mut st = self.state.buffer.lock();
                let pos = st.write_pos;
                st.data[pos] = sample;
                st.write_pos = pos + 1;
                if st.write_pos >= FFT_SIZE {
                    st.write_pos = 0;
                    st.ready = true;
                }
            }
        }

        Some(sample)
    }
}

impl<S> Source for SpectrumSource<S>
where S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { self.inner.total_duration() }
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> { self.inner.try_seek(pos) }
}

// ==================== Position State (lock-free) ====================

/// 播放位置状态：全部使用原子操作，音频线程零锁写入，
/// poller 线程零锁读取。
pub struct PositionState {
    samples_played: AtomicU64,
    seek_offset_secs: AtomicU64,
    sample_rate: AtomicU32,
    channels: AtomicU32,
    duration_secs: AtomicU64,
}

impl PositionState {
    pub fn new() -> Self {
        Self {
            samples_played: AtomicU64::new(0),
            seek_offset_secs: AtomicU64::new(0.0f64.to_bits()),
            sample_rate: AtomicU32::new(44100),
            channels: AtomicU32::new(2),
            duration_secs: AtomicU64::new(0.0f64.to_bits()),
        }
    }

    pub fn set_initial(&self, sample_rate: u32, channels: u16, duration: f64) {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        self.channels.store(channels as u32, Ordering::Relaxed);
        self.duration_secs.store(duration.to_bits(), Ordering::Relaxed);
    }

    pub fn position_secs(&self) -> f64 {
        let samples = self.samples_played.load(Ordering::Relaxed) as f64;
        let sr = self.sample_rate.load(Ordering::Relaxed) as f64;
        let ch = self.channels.load(Ordering::Relaxed) as f64;
        let offset = f64::from_bits(self.seek_offset_secs.load(Ordering::Relaxed));
        samples / (sr * ch) + offset
    }

    pub fn duration_secs(&self) -> f64 {
        f64::from_bits(self.duration_secs.load(Ordering::Relaxed))
    }
}

// ==================== Position Source ====================

pub struct PositionSource<S> {
    inner: S,
    state: Arc<PositionState>,
}

impl<S> PositionSource<S> {
    pub fn new(inner: S, state: Arc<PositionState>) -> Self {
        Self { inner, state }
    }
}

impl<S> Iterator for PositionSource<S>
where S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.state.samples_played.fetch_add(1, Ordering::Relaxed);
        Some(sample)
    }
}

impl<S> Source for PositionSource<S>
where S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { self.inner.total_duration() }
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> {
        let result = self.inner.try_seek(pos);
        if result.is_ok() {
            self.state.samples_played.store(0, Ordering::Relaxed);
            self.state.seek_offset_secs.store(pos.as_secs_f64().to_bits(), Ordering::Relaxed);
        }
        result
    }
}
