#[cfg(target_os = "android")]
use jni::objects::{JClass, JObject, JValue};

#[cfg(target_os = "android")]
const HELPER_CLASS: &str = "com.vant.Mio.Music.DlnaMulticastLock";

#[cfg(target_os = "android")]
const ACQUIRE_SIGNATURE: &str = "(Landroid/content/Context;)V";

#[cfg(target_os = "android")]
const RELEASE_SIGNATURE: &str = "()V";

#[cfg(target_os = "android")]
const LOCAL_FRAME_CAPACITY: i32 = 16;

#[cfg(target_os = "android")]
#[derive(Debug)]
struct AdapterError(String);

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for AdapterError {
    fn from(error: jni::errors::Error) -> Self {
        Self(format!("JNI 本地引用帧失败: {error}"))
    }
}

#[cfg(target_os = "android")]
pub fn acquire_multicast_lock() -> Result<(), String> {
    with_env("获取 DLNA 组播锁", |env| {
        let (class, context) = helper_class_and_context(env)?;
        call_static(
            env,
            &class,
            "acquire",
            ACQUIRE_SIGNATURE,
            &[JValue::Object(&context)],
        )
    })
}

#[cfg(not(target_os = "android"))]
pub fn acquire_multicast_lock() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "android")]
pub fn release_multicast_lock() {
    if let Err(error) = with_env("释放 DLNA 组播锁", |env| {
        let (class, _) = helper_class_and_context(env)?;
        call_static(env, &class, "release", RELEASE_SIGNATURE, &[])
    }) {
        eprintln!("[DLNA] {error}");
    }
}

#[cfg(not(target_os = "android"))]
pub fn release_multicast_lock() {}

#[cfg(target_os = "android")]
fn with_env<F>(operation: &str, call: F) -> Result<(), String>
where
    F: FnOnce(&mut jni::JNIEnv) -> Result<(), AdapterError>,
{
    let vm_pointer = ndk_context::android_context().vm() as *mut _;
    let vm = unsafe { jni::JavaVM::from_raw(vm_pointer) }
        .map_err(|error| format!("{operation} 时获取 JavaVM 失败: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("{operation} 时附加 Java 线程失败: {error}"))?;
    match env.with_local_frame(LOCAL_FRAME_CAPACITY, call) {
        Ok(()) => Ok(()),
        Err(error) => Err(format!(
            "{operation}失败: {}{}",
            error.0,
            describe_and_clear_pending_exception(&mut env)
        )),
    }
}

#[cfg(target_os = "android")]
fn helper_class_and_context<'local>(
    env: &mut jni::JNIEnv<'local>,
) -> Result<(JClass<'local>, JObject<'local>), AdapterError> {
    let activity =
        unsafe { JObject::from_raw(ndk_context::android_context().context() as jni::sys::jobject) };
    let context_result = env.call_method(
        &activity,
        "getApplicationContext",
        "()Landroid/content/Context;",
        &[],
    );
    let context = object_result(env, "读取 Android application context", context_result)?;
    if context.is_null() {
        return Err(AdapterError(
            "读取 Android application context 时返回空对象".to_string(),
        ));
    }
    let loader_result =
        env.call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]);
    let loader = object_result(env, "读取 Android application class loader", loader_result)?;
    let class_name = env
        .new_string(HELPER_CLASS)
        .map_err(|error| jni_error(env, "创建 DLNA 组播锁类名", error))?;
    let class_result = env.call_method(
        &loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&class_name)],
    );
    let class = object_result(
        env,
        "从 Android application class loader 加载 DLNA 组播锁类",
        class_result,
    )?;
    if class.is_null() {
        return Err(AdapterError("加载 DLNA 组播锁类时返回空对象".to_string()));
    }
    Ok((JClass::from(class), context))
}

#[cfg(target_os = "android")]
fn object_result<'local>(
    env: &mut jni::JNIEnv<'local>,
    operation: &str,
    result: jni::errors::Result<jni::objects::JValueOwned<'local>>,
) -> Result<JObject<'local>, AdapterError> {
    result
        .map_err(|error| jni_error(env, operation, error))?
        .l()
        .map_err(|error| jni_error(env, operation, error))
}

#[cfg(target_os = "android")]
fn call_static(
    env: &mut jni::JNIEnv,
    class: &JClass,
    method: &str,
    signature: &str,
    arguments: &[JValue],
) -> Result<(), AdapterError> {
    env.call_static_method(class, method, signature, arguments)
        .map(|_| ())
        .map_err(|error| jni_error(env, "调用 DLNA 组播锁辅助类", error))
}

#[cfg(target_os = "android")]
fn jni_error(env: &mut jni::JNIEnv, operation: &str, error: jni::errors::Error) -> AdapterError {
    let has_exception = env.exception_check().unwrap_or(false);
    if has_exception {
        let _ = env.exception_clear();
    }
    let detail = if has_exception {
        "（Java 异常已清除）"
    } else {
        ""
    };
    AdapterError(format!("{operation}失败: {error}{detail}"))
}

#[cfg(target_os = "android")]
fn describe_and_clear_pending_exception(env: &mut jni::JNIEnv) -> String {
    let throwable = match env.exception_occurred() {
        Ok(throwable) => throwable,
        Err(error) => return format!("（无法读取待处理 Java 异常: {error}）"),
    };
    if throwable.is_null() {
        return String::new();
    }

    let mut cleanup_errors = Vec::new();
    if let Err(error) = env.exception_describe() {
        cleanup_errors.push(format!("描述 Java 异常失败: {error}"));
    }
    if let Err(error) = env.exception_clear() {
        cleanup_errors.push(format!("清除 Java 异常失败: {error}"));
    }
    if let Err(error) = env.delete_local_ref(throwable) {
        cleanup_errors.push(format!("释放 Java throwable 引用失败: {error}"));
    }

    if cleanup_errors.is_empty() {
        "（已描述并清除 Java 异常）".to_string()
    } else {
        format!("（Java 异常清理不完整: {}）", cleanup_errors.join("；"))
    }
}
