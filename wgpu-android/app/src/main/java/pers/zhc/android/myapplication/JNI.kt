package pers.zhc.android.myapplication

import android.view.Surface

object JNI {
    init {
        System.loadLibrary("app_jni")

        initLogger()
    }

    private external fun initLogger()
    external fun initWgpu(surface: Surface)
    external fun resize(width: Int, height: Int)
    external fun cleanup()
    external fun update()

    external fun simpleCompute(): String

    external fun sha256Demo(
        workgroupSize: Int,
        dispatchX: Int,
        iterations: Int,
        difficulty: Int,
        logCallback: LogCallback,
    )

    abstract class LogCallback {
        abstract fun print(line: String)
    }
}
