use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use rodio::Source;

// ==================== Atomic f64 helper ====================

/// Lock-free f64 通过 AtomicU64 实现，用于音频热路径上的参数共享。
/// Relaxed ordering 足够：单值读写，无复杂同步协议。
pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    pub fn new(v: f64) -> Self { Self(AtomicU64::new(v.to_bits())) }
    pub fn load(&self) -> f64 { f64::from_bits(self.0.load(Ordering::Relaxed)) }
    pub fn store(&self, v: f64) { self.0.store(v.to_bits(), Ordering::Relaxed) }
}

// ==================== 滤波器类型枚举 ====================

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    Peak,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

impl FilterType {
    /// 默认 Q 值：Peak 用 1.414（2^0.5，经典的 Butterworth 峰值宽度），
    /// Shelf/LP/HP/Notch 用 0.707（Butterworth 平坦响应）。
    pub fn default_q(self) -> f64 {
        match self {
            Self::Peak => 1.414,
            _ => 0.707,
        }
    }
}

impl TryFrom<u32> for FilterType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Peak),
            1 => Ok(Self::LowShelf),
            2 => Ok(Self::HighShelf),
            3 => Ok(Self::LowPass),
            4 => Ok(Self::HighPass),
            5 => Ok(Self::Notch),
            _ => Err(()),
        }
    }
}

impl From<FilterType> for u32 {
    fn from(ft: FilterType) -> Self {
        match ft {
            FilterType::Peak => 0,
            FilterType::LowShelf => 1,
            FilterType::HighShelf => 2,
            FilterType::LowPass => 3,
            FilterType::HighPass => 4,
            FilterType::Notch => 5,
        }
    }
}

// ==================== 双二阶滤波器 ====================

#[derive(Clone)]
pub struct BiquadFilter {
    b0: f64, b1: f64, b2: f64, a1: f64, a2: f64,
    x1: f64, x2: f64, y1: f64, y2: f64,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Peaking EQ 滤波器系数
    pub fn peaking_eq(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        let a = 10.0_f64.powf(gain_db / 40.0);
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha / a;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Low Shelf 滤波器系数（支持自定义 Q 值）
    pub fn low_shelf(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        let a = 10.0_f64.powf(gain_db / 40.0);
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let sqrt_a = a.sqrt();

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w + 2.0 * sqrt_a * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w + 2.0 * sqrt_a * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w - 2.0 * sqrt_a * alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// High Shelf 滤波器系数
    pub fn high_shelf(sample_rate: f64, freq: f64, gain_db: f64, q: f64) -> Self {
        let a = 10.0_f64.powf(gain_db / 40.0);
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let sqrt_a = a.sqrt();

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w + 2.0 * sqrt_a * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w + 2.0 * sqrt_a * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w - 2.0 * sqrt_a * alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Low Pass 滤波器系数
    pub fn low_pass(sample_rate: f64, freq: f64, q: f64) -> Self {
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let b0 = (1.0 - cos_w) / 2.0;
        let b1 = 1.0 - cos_w;
        let b2 = (1.0 - cos_w) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// High Pass 滤波器系数
    pub fn high_pass(sample_rate: f64, freq: f64, q: f64) -> Self {
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let b0 = (1.0 + cos_w) / 2.0;
        let b1 = -(1.0 + cos_w);
        let b2 = (1.0 + cos_w) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Notch 滤波器系数
    pub fn notch(sample_rate: f64, freq: f64, q: f64) -> Self {
        let omega = 2.0 * std::f64::consts::PI * freq / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let b0 = 1.0;
        let b1 = -2.0 * cos_w;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// 从另一个滤波器更新系数，保留内部状态 (x1, x2, y1, y2)，
    /// 避免 gain 变化时的音频爆破噪声。
    fn update_coeffs_from(&mut self, other: &BiquadFilter) {
        self.b0 = other.b0;
        self.b1 = other.b1;
        self.b2 = other.b2;
        self.a1 = other.a1;
        self.a2 = other.a2;
    }

    pub fn run(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

// ==================== EQ 参数校验 ====================

pub const NUM_EQ_BANDS: usize = 10;
pub const EQ_GAIN_MIN: f64 = -24.0;
pub const EQ_GAIN_MAX: f64 = 24.0;
pub const EQ_FREQS: [f64; NUM_EQ_BANDS] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub fn clamp_eq_gain(gain_db: f64) -> f64 {
    if gain_db.is_finite() {
        gain_db.clamp(EQ_GAIN_MIN, EQ_GAIN_MAX)
    } else {
        0.0
    }
}

pub fn clamp_eq_freq(freq: f64) -> f64 {
    if freq.is_finite() {
        freq.clamp(20.0, 20000.0)
    } else {
        1000.0
    }
}

pub fn clamp_eq_q(q: f64) -> f64 {
    if q.is_finite() {
        q.clamp(0.1, 10.0)
    } else {
        1.0
    }
}

// ==================== EQ 频段参数 ====================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBand {
    pub frequency: f64,
    pub gain: f64,
    pub q: f64,
    #[serde(rename = "type")]
    pub filter_type: FilterType,
    pub enabled: bool,
}

impl EqBand {
    pub fn default_at(freq: f64) -> Self {
        Self {
            frequency: freq,
            gain: 0.0,
            q: FilterType::Peak.default_q(),
            filter_type: FilterType::Peak,
            enabled: true,
        }
    }

    pub fn sanitized(&self) -> Self {
        Self {
            frequency: clamp_eq_freq(self.frequency),
            gain: clamp_eq_gain(self.gain),
            q: clamp_eq_q(self.q),
            filter_type: self.filter_type,
            enabled: self.enabled,
        }
    }
}

// ==================== EQ 设置 ====================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqSettings {
    pub enabled: bool,
    pub global_gain: f64,
    pub bands: [EqBand; NUM_EQ_BANDS],
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            global_gain: 0.0,
            bands: std::array::from_fn(|i| EqBand::default_at(EQ_FREQS[i])),
        }
    }
}

impl EqSettings {
    pub fn sanitized(&self) -> Self {
        Self {
            enabled: self.enabled,
            global_gain: clamp_eq_gain(self.global_gain),
            bands: std::array::from_fn(|i| self.bands[i].sanitized()),
        }
    }

    pub fn effective_global_gain(&self) -> f64 {
        if self.enabled { clamp_eq_gain(self.global_gain) } else { 0.0 }
    }

    pub fn effective_bands(&self) -> [f64; NUM_EQ_BANDS] {
        if self.enabled {
            std::array::from_fn(|i| {
                if self.bands[i].enabled {
                    clamp_eq_gain(self.bands[i].gain)
                } else {
                    0.0
                }
            })
        } else {
            [0.0; NUM_EQ_BANDS]
        }
    }
}

// ==================== EQ 共享状态 ====================

/// EQ 状态通过原子操作共享参数值，音频线程在本地持有滤波器实例。
/// IPC 线程调用 set_settings/set_band 写入参数 → 音频线程通过原子读取检测变化 → 本地重建系数。
pub struct EqState {
    gains: [AtomicU64; NUM_EQ_BANDS],
    freqs: [AtomicU64; NUM_EQ_BANDS],
    qs: [AtomicU64; NUM_EQ_BANDS],
    filter_types: [AtomicU32; NUM_EQ_BANDS],
    global_gain: AtomicU64,
    pub channels: u16,
    pub sample_rate: u32,
}

impl EqState {
    pub fn new(channels: u16, sample_rate: u32) -> Self {
        Self {
            gains: std::array::from_fn(|_| AtomicU64::new(0.0f64.to_bits())),
            freqs: std::array::from_fn(|i| AtomicU64::new(EQ_FREQS[i].to_bits())),
            qs: std::array::from_fn(|_| AtomicU64::new(FilterType::Peak.default_q().to_bits())),
            filter_types: std::array::from_fn(|_| AtomicU32::new(u32::from(FilterType::Peak))),
            global_gain: AtomicU64::new(0.0f64.to_bits()),
            channels,
            sample_rate,
        }
    }

    #[allow(dead_code)]
    pub fn set_band(&self, band: usize, gain_db: f64) {
        if band >= NUM_EQ_BANDS { return }
        self.gains[band].store(clamp_eq_gain(gain_db).to_bits(), Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn set_band_params(&self, band: usize, freq: f64, gain: f64, q: f64, filter_type: FilterType) {
        if band >= NUM_EQ_BANDS { return }
        self.gains[band].store(clamp_eq_gain(gain).to_bits(), Ordering::Relaxed);
        self.freqs[band].store(clamp_eq_freq(freq).to_bits(), Ordering::Relaxed);
        self.qs[band].store(clamp_eq_q(q).to_bits(), Ordering::Relaxed);
        self.filter_types[band].store(u32::from(filter_type), Ordering::Relaxed);
    }

    pub fn set_global_gain(&self, gain_db: f64) {
        self.global_gain.store(clamp_eq_gain(gain_db).to_bits(), Ordering::Relaxed);
    }

    pub fn set_settings(&self, settings: &EqSettings) {
        self.set_global_gain(settings.effective_global_gain());
        for (index, band) in settings.bands.iter().enumerate() {
            let gain = if settings.enabled && band.enabled {
                clamp_eq_gain(band.gain)
            } else {
                0.0
            };
            self.freqs[index].store(clamp_eq_freq(band.frequency).to_bits(), Ordering::Relaxed);
            self.qs[index].store(clamp_eq_q(band.q).to_bits(), Ordering::Relaxed);
            self.filter_types[index].store(u32::from(band.filter_type), Ordering::Relaxed);
            self.gains[index].store(gain.to_bits(), Ordering::Relaxed);
        }
    }
}

// ==================== EQ Source 包装器 ====================

/// EQ 滤波器 Source：本地持有滤波器状态，通过原子操作检测参数变化。
/// 零锁竞争：音频热路径完全不涉及 Mutex。
pub struct EqSource<S> {
    inner: S,
    eq_state: Arc<EqState>,
    filters: Vec<Vec<BiquadFilter>>,
    cached_gains: [f64; NUM_EQ_BANDS],
    cached_freqs: [f64; NUM_EQ_BANDS],
    cached_qs: [f64; NUM_EQ_BANDS],
    cached_filter_types: [FilterType; NUM_EQ_BANDS],
    cached_global_gain: f64,
    cached_global_gain_factor: f64,
    ch: usize,
}

impl<S> EqSource<S> {
    pub fn new(inner: S, eq_state: Arc<EqState>) -> Self {
        let channels = eq_state.channels as usize;
        let filters = (0..NUM_EQ_BANDS)
            .map(|_| (0..channels).map(|_| BiquadFilter::new()).collect())
            .collect();
        Self {
            inner,
            eq_state,
            filters,
            cached_gains: [0.0; NUM_EQ_BANDS],
            cached_freqs: EQ_FREQS,
            cached_qs: [FilterType::Peak.default_q(); NUM_EQ_BANDS],
            cached_filter_types: [FilterType::Peak; NUM_EQ_BANDS],
            cached_global_gain: 0.0,
            cached_global_gain_factor: 1.0,
            ch: 0,
        }
    }
}

impl<S> Iterator for EqSource<S>
where S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        let ch = self.ch % self.eq_state.channels as usize;
        self.ch += 1;

        let sr = self.eq_state.sample_rate as f64;

        // 通过原子读取检测参数变化，只在变化时重建滤波器系数
        for band in 0..NUM_EQ_BANDS {
            let gain = f64::from_bits(self.eq_state.gains[band].load(Ordering::Relaxed));
            let freq = f64::from_bits(self.eq_state.freqs[band].load(Ordering::Relaxed));
            let q = f64::from_bits(self.eq_state.qs[band].load(Ordering::Relaxed));
            let filter_type = FilterType::try_from(
                self.eq_state.filter_types[band].load(Ordering::Relaxed)
            ).unwrap_or(FilterType::Peak);

            if gain != self.cached_gains[band]
                || freq != self.cached_freqs[band]
                || q != self.cached_qs[band]
                || filter_type != self.cached_filter_types[band]
            {
                self.cached_gains[band] = gain;
                self.cached_freqs[band] = freq;
                self.cached_qs[band] = q;
                self.cached_filter_types[band] = filter_type;

                let new_filter = match filter_type {
                    FilterType::Peak => BiquadFilter::peaking_eq(sr, freq, gain, q),
                    FilterType::LowShelf => BiquadFilter::low_shelf(sr, freq, gain, q),
                    FilterType::HighShelf => BiquadFilter::high_shelf(sr, freq, gain, q),
                    FilterType::LowPass => BiquadFilter::low_pass(sr, freq, q),
                    FilterType::HighPass => BiquadFilter::high_pass(sr, freq, q),
                    FilterType::Notch => BiquadFilter::notch(sr, freq, q),
                };
                for c in 0..self.eq_state.channels as usize {
                    self.filters[band][c].update_coeffs_from(&new_filter);
                }
            }
        }

        let mut val = sample as f64;
        for band in 0..NUM_EQ_BANDS {
            val = self.filters[band][ch].run(val);
        }

        let global_gain = f64::from_bits(self.eq_state.global_gain.load(Ordering::Relaxed));
        if global_gain != self.cached_global_gain {
            self.cached_global_gain = global_gain;
            self.cached_global_gain_factor = if global_gain.abs() > 0.001 {
                10.0_f64.powf(clamp_eq_gain(global_gain) / 20.0)
            } else {
                1.0
            };
        }
        val *= self.cached_global_gain_factor;

        Some(val as f32)
    }
}

impl<S> Source for EqSource<S>
where S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { self.inner.total_duration() }
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> { self.inner.try_seek(pos) }
}

// ==================== Balance Source ====================

pub struct BalanceSource<S> {
    inner: S,
    balance: Arc<AtomicF64>,
    ch: usize,
}

impl<S> BalanceSource<S> {
    pub fn new(inner: S, balance: Arc<AtomicF64>) -> Self {
        Self { inner, balance, ch: 0 }
    }
}

impl<S> Iterator for BalanceSource<S>
where S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        let balance = self.balance.load();
        let ch = self.ch % 2;
        self.ch += 1;

        let val = if balance >= 0.0 {
            if ch == 0 { sample } else { (sample as f64 * (1.0 - balance)) as f32 }
        } else {
            if ch == 0 { (sample as f64 * (1.0 + balance)) as f32 } else { sample }
        };
        Some(val)
    }
}

impl<S> Source for BalanceSource<S>
where S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<std::time::Duration> { self.inner.total_duration() }
    fn try_seek(&mut self, pos: std::time::Duration) -> Result<(), rodio::source::SeekError> { self.inner.try_seek(pos) }
}
