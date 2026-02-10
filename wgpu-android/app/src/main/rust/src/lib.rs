#![feature(decl_macro)]
#![feature(try_blocks)]

mod compute_demo;
mod hello_triangle;
mod sha256_miner;

use crate::hello_triangle::State;
use jni::objects::{JClass, JObject};
use jni::sys::{jint, jstring};
use jni::JNIEnv;
use log::{debug, error, info, LevelFilter};
use once_cell::sync::Lazy;
use raw_window_handle::{
    AndroidNdkWindowHandle, DisplayHandle, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};
use std::ptr::NonNull;
use std::sync::Mutex;
use wgpu::{Instance, SurfaceTarget};

pub macro default() {
    Default::default()
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_initLogger(
    _env: JNIEnv,
    _c: JClass,
) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("Android wgpu demo")
            .with_max_level(LevelFilter::Info),
    );
    info!("Android logger initialized.");
}

/// 模拟一个持有 NativeWindow 的结构体
pub struct AndroidWindow {
    native_window: *mut ndk_sys::ANativeWindow,
    width: u32,
    height: u32,
}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<WindowHandle, raw_window_handle::HandleError> {
        let mut handle = AndroidNdkWindowHandle::new(
            NonNull::new(self.native_window as *mut _).expect("Window handle is null"),
        );
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AndroidNdk(handle)) })
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle, raw_window_handle::HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Android(
                raw_window_handle::AndroidDisplayHandle::new(),
            ))
        })
    }
}

unsafe impl Send for AndroidWindow {}
unsafe impl Sync for AndroidWindow {}

static STATE: Lazy<Mutex<Option<State>>> = Lazy::new(|| default!());

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_initWgpu(
    env: JNIEnv,
    _c: JClass,
    surface: JObject,
) {
    info!("initWgpu called");

    unsafe {
        let window_ptr =
            ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface.as_raw());
        let width = ndk_sys::ANativeWindow_getWidth(window_ptr);
        let height = ndk_sys::ANativeWindow_getHeight(window_ptr);

        if window_ptr.is_null() {
            error!("window_ptr is null");
            return; // 或者抛出 Java 异常
        }

        let android_window = AndroidWindow {
            native_window: window_ptr,
            width: width as _,
            height: height as _,
        };

        pollster::block_on(async {
            let state = State::new(android_window).await.unwrap();
            *STATE.lock().unwrap() = Some(state);
        });
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_resize(
    _env: JNIEnv,
    _c: JClass,
    width: jint,
    height: jint,
) {
    info!("resize called");
    let mut guard = STATE.lock().unwrap();
    let state = guard.as_mut().unwrap();
    state.update_size((width as _, height as _));
    state.configure_surface();
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_cleanup(_env: JNIEnv, _c: JClass) {
    info!("cleanup called");
    let mut guard = STATE.lock().unwrap();
    *guard = None;
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_simpleCompute(
    env: JNIEnv,
    _c: JClass,
) -> jstring {
    let result = compute_demo::compute();
    let result = pollster::block_on(result).unwrap();
    env.new_string(format!("{:?}", result)).unwrap().into_raw()
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_update(env: JNIEnv, _c: JClass) {
    info!("update called");
    let guard = STATE.lock().unwrap();
    let state = guard.as_ref().unwrap();
    state.render().unwrap();
}
