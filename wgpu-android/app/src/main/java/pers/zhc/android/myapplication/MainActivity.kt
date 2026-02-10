package pers.zhc.android.myapplication

import android.content.Intent
import android.os.Bundle
import android.view.SurfaceHolder
import androidx.appcompat.app.AppCompatActivity
import pers.zhc.android.myapplication.databinding.ActivityMainBinding

class MainActivity : AppCompatActivity(), SurfaceHolder.Callback {
    private lateinit var appendLog: (line: String) -> Unit

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bindings = ActivityMainBinding.inflate(layoutInflater).also { setContentView(it.root) }
        appendLog = {line: String->
            bindings.logTv.apply {
                append(line)
                append("\n")
            }
        }

        appendLog("simpleCompute result: ${JNI.simpleCompute()}")

        bindings.surfaceView.holder.addCallback(this)

        bindings.sha256MinerBtn.setOnClickListener {
            startActivity(Intent(this, Sha256MinerActivity::class.java))
        }
        bindings.btnUpdateSurface.setOnClickListener {
            JNI.update()
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        val surface = holder.surface
        JNI.initWgpu(surface)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        JNI.resize(width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        JNI.cleanup()
    }
}
