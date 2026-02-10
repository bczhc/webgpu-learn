package pers.zhc.android.myapplication

import android.content.Intent
import android.os.Bundle
import android.view.Choreographer
import android.view.SurfaceHolder
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import pers.zhc.android.myapplication.databinding.ActivityMainBinding

class MainActivity : AppCompatActivity(), SurfaceHolder.Callback {
    private lateinit var tvFps: TextView
    private lateinit var appendLog: (line: String) -> Unit

    // address of the underlying JNI object
    private var addr: Long = 0

    private val defaultAnimation = JNI.Animations.ROTATING_TRIANGLE

    private var lastFrameTimeNanos: Long = 0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val bindings = ActivityMainBinding.inflate(layoutInflater).also { setContentView(it.root) }
        appendLog = { line: String ->
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
        tvFps = bindings.tvFps
        bindings.btnSelectAnimation.setOnClickListener {
            val animations = JNI.Animations.entries.toTypedArray()
            val items = animations.map { it.name }.toTypedArray()

            MaterialAlertDialogBuilder(this)
                .setTitle("选择动画模式")
                .setItems(items) { d, which ->
                    val selected = animations[which]
                    val oldAddr = addr
                    // set addr=0 to pause Choreographer's rendering
                    addr = 0
                    // then change animation
                    val newAddr = JNI.changeAnimation(oldAddr, selected.id)
                    addr = newAddr
                }
                .show()
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        val surface = holder.surface
        addr = JNI.initWgpu(surface, defaultAnimation.id)

        Choreographer.getInstance().postFrameCallback(object : Choreographer.FrameCallback {
            override fun doFrame(frameTimeNanos: Long) {
                if (addr != 0L) {
                    JNI.frame(addr)

                    if (lastFrameTimeNanos != 0L) {
                        val diffNanos = frameTimeNanos - lastFrameTimeNanos
                        // 1s = 1,000,000,000ns
                        val fps = 1_000_000_000.0 / diffNanos
                        tvFps.text = String.format("FPS: %.1f", fps)
                    }
                    lastFrameTimeNanos = frameTimeNanos
                }
                Choreographer.getInstance().postFrameCallback(this)
            }
        })
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        JNI.resize(addr, width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        JNI.cleanup(addr)
        addr = 0
    }
}
