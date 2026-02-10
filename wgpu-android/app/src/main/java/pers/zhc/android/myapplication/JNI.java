package pers.zhc.android.myapplication;

public class JNI {
    static  {
        System.loadLibrary("app_jni");
    }

    public static native String greet();
}
