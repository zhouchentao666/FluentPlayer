use core_foundation_sys::string::{
    CFStringGetCString, CFStringGetCStringPtr, CFStringGetLength, CFStringRef,
};
use coreaudio_sys::*;
use serde::Serialize;
use std::ffi::CStr;
use std::ptr;
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

const ENC_UTF8: CFStringEncoding = kCFStringEncodingUTF8 as u32;

fn is_ok(status: OSStatus) -> bool {
    status == 0
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: f64,
    pub channels: u32,
    pub volume: f32,
    pub volume_supported: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

unsafe fn cfstring_to_string(cf_str: CFStringRef) -> String {
    if cf_str.is_null() {
        return String::new();
    }
    let c_ptr = CFStringGetCStringPtr(cf_str, ENC_UTF8);
    if !c_ptr.is_null() {
        return CStr::from_ptr(c_ptr as *const i8)
            .to_string_lossy()
            .into_owned();
    }
    let len = CFStringGetLength(cf_str);
    let buf_size = (len as usize) * 4 + 1;
    let mut buf = vec![0i8; buf_size];
    if CFStringGetCString(cf_str, buf.as_mut_ptr(), buf_size as _, ENC_UTF8) != 0 {
        CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    } else {
        String::new()
    }
}

unsafe fn get_audio_device_ids() -> Result<Vec<AudioDeviceID>, String> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };

    let mut size: UInt32 = 0;
    let status = AudioObjectGetPropertyDataSize(
        kAudioObjectSystemObject,
        &address,
        0,
        ptr::null(),
        &mut size,
    );
    if !is_ok(status) {
        return Err(format!("AudioObjectGetPropertyDataSize failed: {status}"));
    }

    let count = size as usize / std::mem::size_of::<AudioDeviceID>();
    if count == 0 {
        return Ok(vec![]);
    }
    let mut ids = vec![0u32 as AudioDeviceID; count];

    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject,
        &address,
        0,
        ptr::null(),
        &mut size,
        ids.as_mut_ptr() as *mut _,
    );
    if !is_ok(status) {
        return Err(format!("AudioObjectGetPropertyData failed: {status}"));
    }

    Ok(ids)
}

unsafe fn get_default_output_device_id() -> Result<AudioDeviceID, String> {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultOutputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut id: AudioDeviceID = 0;
    let mut size = std::mem::size_of::<AudioDeviceID>() as UInt32;
    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject,
        &address,
        0,
        ptr::null(),
        &mut size,
        &mut id as *mut _ as *mut _,
    );
    if !is_ok(status) {
        return Err(format!("Failed to get default output device: {status}"));
    }
    Ok(id)
}

unsafe fn get_device_name(id: AudioDeviceID) -> String {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyDeviceNameCFString,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut name: CFStringRef = ptr::null_mut();
    let mut size = std::mem::size_of::<CFStringRef>() as UInt32;
    let status = AudioObjectGetPropertyData(
        id,
        &address,
        0,
        ptr::null(),
        &mut size,
        &mut name as *mut _ as *mut _,
    );
    if !is_ok(status) {
        return format!("Device {id}");
    }
    cfstring_to_string(name)
}

unsafe fn get_device_sample_rate(id: AudioDeviceID) -> f64 {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyNominalSampleRate,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut rate: Float64 = 0.0;
    let mut size = std::mem::size_of::<Float64>() as UInt32;
    let status = AudioObjectGetPropertyData(
        id,
        &address,
        0,
        ptr::null(),
        &mut size,
        &mut rate as *mut _ as *mut _,
    );
    if !is_ok(status) {
        0.0
    } else {
        rate
    }
}

unsafe fn get_device_output_channels(id: AudioDeviceID) -> u32 {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyStreamConfiguration,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size: UInt32 = 0;
    let status = AudioObjectGetPropertyDataSize(id, &address, 0, ptr::null(), &mut size);
    if !is_ok(status) || size == 0 {
        return 0;
    }
    let layout = malloc(size as _) as *mut AudioBufferList;
    if layout.is_null() {
        return 0;
    }
    let status =
        AudioObjectGetPropertyData(id, &address, 0, ptr::null(), &mut size, layout as *mut _);
    let channels = if is_ok(status) {
        let buffers_ptr = std::ptr::addr_of!((*layout).mBuffers).cast::<AudioBuffer>();
        let buffers_offset = buffers_ptr as usize - layout as usize;
        let buffer_count = ((*layout).mNumberBuffers as usize).min(
            (size as usize).saturating_sub(buffers_offset) / std::mem::size_of::<AudioBuffer>(),
        );
        let buffers = std::slice::from_raw_parts(buffers_ptr, buffer_count);
        buffers.iter().map(|buffer| buffer.mNumberChannels).sum()
    } else {
        0
    };
    free(layout as *mut _);
    channels
}

unsafe fn get_device_volume(id: AudioDeviceID) -> (f32, bool) {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyVolumeScalar,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut vol: Float32 = 0.0;
    let mut size = std::mem::size_of::<Float32>() as UInt32;
    let status = AudioObjectGetPropertyData(
        id,
        &address,
        0,
        ptr::null(),
        &mut size,
        &mut vol as *mut _ as *mut _,
    );
    if !is_ok(status) {
        (0.0, false)
    } else {
        (vol, true)
    }
}

unsafe fn device_has_output(id: AudioDeviceID) -> bool {
    let address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyStreamConfiguration,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size: UInt32 = 0;
    let status = AudioObjectGetPropertyDataSize(id, &address, 0, ptr::null(), &mut size);
    if !is_ok(status) || size == 0 {
        return false;
    }
    let layout = malloc(size as _) as *mut AudioBufferList;
    if layout.is_null() {
        return false;
    }
    let status =
        AudioObjectGetPropertyData(id, &address, 0, ptr::null(), &mut size, layout as *mut _);
    let has = if is_ok(status) {
        let buffers_ptr = std::ptr::addr_of!((*layout).mBuffers).cast::<AudioBuffer>();
        let buffers_offset = buffers_ptr as usize - layout as usize;
        let buffer_count = ((*layout).mNumberBuffers as usize).min(
            (size as usize).saturating_sub(buffers_offset) / std::mem::size_of::<AudioBuffer>(),
        );
        let buffers = std::slice::from_raw_parts(buffers_ptr, buffer_count);
        buffers.iter().any(|buffer| buffer.mNumberChannels > 0)
    } else {
        false
    };
    free(layout as *mut _);
    has
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn enumerate_devices() -> Result<Vec<DeviceInfo>, String> {
    unsafe {
        let default_id = get_default_output_device_id().unwrap_or(0);
        let ids = get_audio_device_ids()?;

        let mut devices = Vec::new();
        for &id in &ids {
            if !device_has_output(id) {
                continue;
            }
            let name = get_device_name(id);
            let (volume, volume_supported) = get_device_volume(id);
            devices.push(DeviceInfo {
                id,
                name,
                is_default: id == default_id,
                sample_rate: get_device_sample_rate(id),
                channels: get_device_output_channels(id),
                volume,
                volume_supported,
            });
        }
        Ok(devices)
    }
}

pub fn set_output_device(device_id: u32) -> Result<(), String> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        let mut id = device_id;
        let status = AudioObjectSetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            ptr::null(),
            std::mem::size_of::<AudioDeviceID>() as UInt32,
            &mut id as *mut _ as *mut _,
        );
        if !is_ok(status) {
            return Err(format!("Failed to set default output device: {status}"));
        }
        Ok(())
    }
}

pub fn get_device_volume_pub(device_id: u32) -> Result<f32, String> {
    unsafe {
        let (vol, supported) = get_device_volume(device_id);
        if !supported {
            return Err("Volume not supported for this device".into());
        }
        Ok(vol)
    }
}

pub fn set_device_volume(device_id: u32, volume: f32) -> Result<(), String> {
    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioDevicePropertyVolumeScalar,
            mScope: kAudioDevicePropertyScopeOutput,
            mElement: kAudioObjectPropertyElementMain,
        };
        let mut vol: Float32 = volume.clamp(0.0, 1.0);
        let status = AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            ptr::null(),
            std::mem::size_of::<Float32>() as UInt32,
            &mut vol as *mut _ as *mut _,
        );
        if !is_ok(status) {
            return Err(format!("Failed to set device volume: {status}"));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Device change listener
// ---------------------------------------------------------------------------

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

unsafe extern "C" fn hardware_property_changed(
    _object_id: AudioObjectID,
    _number_addresses: UInt32,
    _addresses: *const AudioObjectPropertyAddress,
    _user_data: *mut std::os::raw::c_void,
) -> OSStatus {
    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit("audio-device-changed", ());
    }
    0
}

pub fn start_listening(app_handle: AppHandle) {
    let _ = APP_HANDLE.set(app_handle);

    unsafe {
        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDevices,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        AudioObjectAddPropertyListener(
            kAudioObjectSystemObject,
            &address,
            Some(hardware_property_changed),
            ptr::null_mut(),
        );

        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        AudioObjectAddPropertyListener(
            kAudioObjectSystemObject,
            &address,
            Some(hardware_property_changed),
            ptr::null_mut(),
        );
    }
}
