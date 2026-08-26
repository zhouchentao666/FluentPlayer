use crate::player::SharedPlayer;
use parking_lot::Mutex;

pub struct MediaControl {
    initialized: bool,
}

impl MediaControl {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn update_now_playing(
        &mut self,
        title: &str,
        artist: &str,
        album: &str,
        duration_secs: f64,
        cover_url: Option<&str>,
    ) {
        #[cfg(target_os = "macos")]
        {
            self.update_now_playing_macos(title, artist, album, duration_secs, cover_url);
        }
        #[cfg(target_os = "android")]
        {
            self.update_now_playing_android(title, artist, album, duration_secs, cover_url);
        }
        self.initialized = true;
    }

    pub fn set_playback_state(&self, playing: bool) {
        #[cfg(target_os = "macos")]
        {
            self.set_playback_state_macos(playing);
        }
        #[cfg(target_os = "android")]
        {
            self.set_playback_state_android(playing);
        }
    }

    pub fn update_playback_position(&self, position_secs: f64, duration_secs: f64) {
        #[cfg(target_os = "android")]
        {
            self.update_playback_position_android(position_secs, duration_secs);
        }
        let _ = (position_secs, duration_secs);
    }

    #[allow(dead_code)]
    pub fn setup_command_handlers(&self, _player: SharedPlayer) {
        #[cfg(target_os = "macos")]
        {
            self.setup_macos_command_handlers(_player);
        }
    }
}

// ---------------------------------------------------------------------------
// Android — JNI calls to MusicService static methods
// ---------------------------------------------------------------------------

#[cfg(target_os = "android")]
impl MediaControl {
    fn with_env<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&mut jni::JNIEnv, &jni::objects::JClass) -> R,
    {
        let cached = match crate::MUSIC_SERVICE_CLASS.get() {
            Some(g) => g,
            None => { eprintln!("[MediaControl] MUSIC_SERVICE_CLASS not cached"); return None; }
        };
        let vm_ptr = ndk_context::android_context().vm() as *mut _;
        let vm = match unsafe { jni::JavaVM::from_raw(vm_ptr) } {
            Ok(vm) => vm,
            Err(e) => { eprintln!("[MediaControl] get_java_vm: {e}"); return None; }
        };
        let mut env = match vm.attach_current_thread() {
            Ok(env) => env,
            Err(e) => { eprintln!("[MediaControl] attach_thread: {e}"); return None; }
        };
        let obj = cached.as_obj();
        let class: &jni::objects::JClass = obj.into();
        let result = f(&mut env, class);
        Some(result)
    }

    fn update_now_playing_android(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration_secs: f64,
        cover_url: Option<&str>,
    ) {
        Self::with_env(|env, class| {
            let _ = Self::call_start_with_class(env, class);
            let j_title = env.new_string(title).unwrap_or_default();
            let j_artist = env.new_string(artist).unwrap_or_default();
            let j_album = env.new_string(album).unwrap_or_default();
            let j_cover = env.new_string(cover_url.unwrap_or("")).unwrap_or_default();

            let _ = env.call_static_method(
                class,
                "updateNowPlaying",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;)V",
                &[
                    jni::objects::JValue::Object(&j_title),
                    jni::objects::JValue::Object(&j_artist),
                    jni::objects::JValue::Object(&j_album),
                    jni::objects::JValue::Long((duration_secs * 1000.0) as i64),
                    jni::objects::JValue::Object(&j_cover),
                ],
            );
        });
    }

    fn set_playback_state_android(&self, playing: bool) {
        Self::with_env(|env, class| {
            let _ = env.call_static_method(
                class,
                "setPlaying",
                "(Z)V",
                &[jni::objects::JValue::Bool(if playing { 1 } else { 0 })],
            );
        });
    }

    fn call_start_with_class(env: &mut jni::JNIEnv, class: &jni::objects::JClass) -> Result<(), jni::errors::Error> {
        let activity = unsafe {
            jni::objects::JObject::from_raw(
                ndk_context::android_context().context() as jni::sys::jobject,
            )
        };
        env.call_static_method(
            class, "start", "(Landroid/content/Context;)V",
            &[jni::objects::JValue::Object(&activity)],
        )?;
        Ok(())
    }

    fn update_playback_position_android(&self, position_secs: f64, duration_secs: f64) {
        Self::with_env(|env, class| {
            let _ = env.call_static_method(
                class,
                "updatePlaybackPosition",
                "(JJ)V",
                &[
                    jni::objects::JValue::Long((position_secs * 1000.0) as i64),
                    jni::objects::JValue::Long((duration_secs * 1000.0) as i64),
                ],
            );
        });
    }
}

// ---------------------------------------------------------------------------
// macOS — MPNowPlayingInfoCenter
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
use objc::runtime::Object;

#[cfg(target_os = "macos")]
impl MediaControl {
    fn update_now_playing_macos(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration_secs: f64,
        _cover_url: Option<&str>,
    ) {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let cls = match Class::get("MPNowPlayingInfoCenter") {
                Some(c) => c,
                None => return,
            };
            let center: *mut Object = msg_send![cls, defaultCenter];
            if center.is_null() {
                return;
            }

            let dict_cls = match Class::get("NSMutableDictionary") {
                Some(c) => c,
                None => return,
            };
            let dict: *mut Object = msg_send![dict_cls, dictionary];
            if dict.is_null() {
                return;
            }

            let set_str = |dict: *mut Object, key: &str, val: &str| {
                let ns_key = Self::ns_string(key);
                let ns_val = Self::ns_string(val);
                let _: () = msg_send![dict, setObject: ns_val forKey: ns_key];
            };

            set_str(dict, "title", title);
            set_str(dict, "artist", artist);
            set_str(dict, "album", album);

            let num_cls = match Class::get("NSNumber") {
                Some(c) => c,
                None => return,
            };
            let duration_ms = (duration_secs * 1000.0) as i64;
            let dur_val: *mut Object =
                msg_send![num_cls, numberWithLongLong: duration_ms];
            let dur_key = Self::ns_string("elapsedPlaybackTime");
            let _: () = msg_send![dict, setObject: dur_val forKey: dur_key];

            let rate_val: *mut Object = msg_send![num_cls, numberWithDouble: 1.0];
            let rate_key = Self::ns_string("playbackRate");
            let _: () = msg_send![dict, setObject: rate_val forKey: rate_key];

            let _: () = msg_send![center, setNowPlayingInfo: dict];
        }
    }

    fn set_playback_state_macos(&self, playing: bool) {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let cls = match Class::get("MPNowPlayingInfoCenter") {
                Some(c) => c,
                None => return,
            };
            let center: *mut Object = msg_send![cls, defaultCenter];
            if center.is_null() {
                return;
            }

            let state: u64 = if playing { 1 } else { 2 };
            let num_cls = match Class::get("NSNumber") {
                Some(c) => c,
                None => return,
            };
            let ns_state: *mut Object = msg_send![num_cls, numberWithUnsignedLongLong: state];
            let _: () = msg_send![center, setPlaybackState: ns_state];
        }
    }

    #[allow(dead_code)]
    fn setup_macos_command_handlers(&self, player: SharedPlayer) {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            let cls = match Class::get("MPRemoteCommandCenter") {
                Some(c) => c,
                None => return,
            };
            let center: *mut Object = msg_send![cls, sharedCommandCenter];
            if center.is_null() {
                return;
            }

            for _cmd_sel in ["playCommand", "pauseCommand", "nextTrackCommand", "previousTrackCommand"] {
                let _sel = sel!(playCommand);
                let cmd: *mut Object = msg_send![center, playCommand];
                if !cmd.is_null() {
                    let _: () = msg_send![cmd, setEnabled: true];
                }
            }

            let _ = player;
        }
    }

    unsafe fn ns_string(s: &str) -> *mut Object {
        use objc::runtime::Class;
        use objc::{msg_send, sel, sel_impl};
        use std::ffi::CString;

        let c_str = CString::new(s).unwrap_or_default();
        let cls = Class::get("NSString").unwrap();
        msg_send![cls, stringWithUTF8String: c_str.as_ptr()]
    }
}

lazy_static::lazy_static! {
    pub static ref MEDIA_CONTROL: Mutex<MediaControl> = Mutex::new(MediaControl::new());
}
