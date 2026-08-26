use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::mpsc::{self, Sender};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

static STOP_SENDER: Mutex<Option<Sender<()>>> = Mutex::new(None);

// --- macOS Microphone Permission via AVFoundation ---

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn request_mic_permission() -> Result<bool, String> {
    unsafe {
        use objc::{class, msg_send, sel, sel_impl};

        let c_str = b"soun\0";
        let media_type: *mut objc::runtime::Object =
            msg_send![class!(NSString), stringWithUTF8String: c_str.as_ptr()];

        let status: i64 =
            msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: media_type];

        match status {
            3 => Ok(true),
            0 => {
                let (tx, rx) = std::sync::mpsc::channel::<bool>();
                let block = block::ConcreteBlock::new(move |granted: i8| {
                    let _ = tx.send(granted != 0);
                });
                let block = block.copy();
                let _: () = msg_send![class!(AVCaptureDevice),
                    requestAccessForMediaType:media_type
                    completionHandler:block];
                rx.recv().map_err(|_| "权限请求超时".to_string())
            }
            2 => Err("麦克风权限被拒绝，请在系统设置 > 隐私与安全性 > 麦克风中开启".to_string()),
            _ => Err("麦克风不可用".to_string()),
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn request_mic_permission() -> Result<bool, String> {
    Ok(true)
}

// --- Audio Capture ---

#[tauri::command]
pub fn audio_capture_start(
    app: AppHandle,
    chunk_duration_ms: Option<u64>,
) -> Result<(), String> {
    stop_internal();

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    *STOP_SENDER.lock() = Some(stop_tx);

    let chunk_ms = chunk_duration_ms.unwrap_or(3000);

    std::thread::spawn(move || {
        if let Err(e) = capture_thread(app.clone(), chunk_ms, stop_rx) {
            eprintln!("Audio capture thread error: {}", e);
            let _ = app.emit(
                "audio-capture:error",
                serde_json::json!({ "error": e }),
            );
        }
    });

    Ok(())
}

fn capture_thread(
    app: AppHandle,
    chunk_duration_ms: u64,
    stop_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    let host = cpal::default_host();

    let device = match host.default_input_device() {
        Some(d) => {
            let name = d.name().unwrap_or_default();
            if name.is_empty() {
                return Err("未找到可用的麦克风设备，请连接麦克风后重试".to_string());
            }
            eprintln!("Audio capture using device: {}", name);
            d
        }
        None => return Err("未找到麦克风设备，请连接麦克风后重试".to_string()),
    };

    eprintln!(
        "Audio capture using device: {}",
        device.name().unwrap_or_default()
    );

    let config = match device.default_input_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("default_input_config failed: {}, trying fallback...", e);
            let supported_configs = device
                .supported_input_configs()
                .map_err(|_| {
                    "无法访问麦克风。可能原因：1) 系统设置中未授予麦克风权限；2) 麦克风设备异常".to_string()
                })?;

            let mut configs: Vec<_> = supported_configs.collect();
            configs.sort_by(|a, b| {
                let prio = |r: u32| match r { 48000 => 0, 44100 => 1, 16000 => 2, _ => 3 };
                prio(a.min_sample_rate().0).cmp(&prio(b.min_sample_rate().0))
            });

            let fallback = configs
                .into_iter()
                .find(|c| matches!(c.sample_format(),
                    cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16))
                .ok_or_else(|| format!("获取音频配置失败: {}", e))?;

            eprintln!(
                "Fallback config: rate={}, channels={}, format={:?}",
                fallback.min_sample_rate().0, fallback.channels(), fallback.sample_format()
            );
            fallback.with_max_sample_rate()
        }
    };

    let native_sample_rate = config.sample_rate().0 as f64;
    let channels = config.channels() as usize;
    let chunk_samples_8k = (chunk_duration_ms as f64 * 8000.0 / 1000.0) as usize;

    eprintln!(
        "Audio config: rate={}, channels={}, format={:?}",
        config.sample_rate().0, config.channels(), config.sample_format()
    );

    let buffer: parking_lot::Mutex<Vec<f32>> =
        parking_lot::Mutex::new(Vec::with_capacity(chunk_samples_8k * 2));
    let last_level_emit: Mutex<Instant> = Mutex::new(Instant::now());

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    process_audio_data(data, channels, native_sample_rate, chunk_samples_8k, &buffer, &last_level_emit, &app);
                },
                |err| eprintln!("Audio capture error: {}", err),
                None,
            )
            .map_err(|e| format!("创建音频流失败: {}", e))?,
        cpal::SampleFormat::I16 => {
            let buf = parking_lot::Mutex::new(Vec::with_capacity(chunk_samples_8k * 2));
            device
                .build_input_stream(
                    &config.config(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        process_audio_data(&f32_data, channels, native_sample_rate, chunk_samples_8k, &buf, &last_level_emit, &app);
                    },
                    |err| eprintln!("Audio capture error: {}", err),
                    None,
                )
                .map_err(|e| format!("创建音频流失败: {}", e))?
        }
        cpal::SampleFormat::U16 => {
            let buf = parking_lot::Mutex::new(Vec::with_capacity(chunk_samples_8k * 2));
            device
                .build_input_stream(
                    &config.config(),
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let f32_data: Vec<f32> = data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect();
                        process_audio_data(&f32_data, channels, native_sample_rate, chunk_samples_8k, &buf, &last_level_emit, &app);
                    },
                    |err| eprintln!("Audio capture error: {}", err),
                    None,
                )
                .map_err(|e| format!("创建音频流失败: {}", e))?
        }
        fmt => return Err(format!("不支持的音频格式: {:?}", fmt)),
    };

    stream.play().map_err(|e| format!("启动音频流失败: {}", e))?;
    eprintln!("Audio capture stream started successfully");

    let _ = stop_rx.recv();
    eprintln!("Audio capture stream stopped");
    Ok(())
}

fn process_audio_data(
    data: &[f32],
    channels: usize,
    native_sample_rate: f64,
    chunk_samples_8k: usize,
    buffer: &parking_lot::Mutex<Vec<f32>>,
    last_level_emit: &Mutex<Instant>,
    app: &AppHandle,
) {
    let rms = {
        let sum: f64 = data.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / data.len() as f64).sqrt()
    };
    let level = (rms * 5.0).min(1.0) as f32;

    {
        let mut last = last_level_emit.lock();
        if last.elapsed().as_millis() >= 66 {
            let _ = app.emit("audio-capture:level", serde_json::json!({ "level": level }));
            *last = Instant::now();
        }
    }

    let mono: Vec<f32> = if channels > 1 {
        data.chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        data.to_vec()
    };

    let ratio = 8000.0 / native_sample_rate;
    let target_len = (mono.len() as f64 * ratio) as usize;
    let resampled: Vec<f32> = if target_len > 0 {
        (0..target_len)
            .map(|i| {
                let src_idx = i as f64 / ratio;
                let idx = src_idx as usize;
                if idx + 1 < mono.len() {
                    let frac = src_idx - idx as f64;
                    (mono[idx] as f64 * (1.0 - frac) + mono[idx + 1] as f64 * frac) as f32
                } else {
                    mono[idx.min(mono.len() - 1)]
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut buf = buffer.lock();
    buf.extend_from_slice(&resampled);

    if buf.len() >= chunk_samples_8k {
        let chunk: Vec<f32> = buf.drain(..chunk_samples_8k).collect();
        let _ = app.emit(
            "audio-capture:chunk",
            serde_json::json!({ "data": chunk, "sampleRate": 8000u32 }),
        );
    }
}

#[tauri::command]
pub fn audio_capture_stop() -> Result<(), String> {
    stop_internal();
    Ok(())
}

fn stop_internal() {
    if let Some(sender) = STOP_SENDER.lock().take() {
        let _ = sender.send(());
    }
}
