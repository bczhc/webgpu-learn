use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jstring;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_pers_zhc_android_myapplication_JNI_greet(env: JNIEnv, _c: JClass) -> jstring {
    let js = env.new_string("world").unwrap();
    js.into_raw()
}
