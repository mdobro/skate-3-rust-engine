package com.skate3.engine

import android.app.AlertDialog
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.StatFs
import android.provider.OpenableColumns
import android.view.View
import android.view.WindowManager
import android.widget.Button
import android.widget.ProgressBar
import android.widget.TextView
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.util.zip.ZipInputStream
import kotlin.concurrent.thread

class LauncherActivity : AppCompatActivity() {
    private lateinit var status: TextView
    private lateinit var error: TextView
    private lateinit var progress: ProgressBar
    private lateinit var importBtn: Button
    private lateinit var playBtn: Button
    private lateinit var deleteBtn: Button
    @Volatile private var busy = false

    private val filesDir0: File get() = getExternalFilesDir(null) ?: filesDir
    private val install: File get() = File(filesDir0, "installation")
    private val staging: File get() = File(filesDir0, "installation.new")
    private val old: File get() = File(filesDir0, "installation.old")

    private val picker = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) startImport(uri)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_launcher)
        status = findViewById(R.id.status)
        error = findViewById(R.id.error)
        progress = findViewById(R.id.progress)
        importBtn = findViewById(R.id.importButton)
        playBtn = findViewById(R.id.playButton)
        deleteBtn = findViewById(R.id.deleteButton)
        importBtn.setOnClickListener { picker.launch(arrayOf("application/zip", "application/x-zip-compressed")) }
        playBtn.setOnClickListener { startActivity(Intent(this, SkateActivity::class.java)) }
        deleteBtn.setOnClickListener {
            AlertDialog.Builder(this)
                .setMessage("Delete the imported game data?")
                .setPositiveButton("Delete") { _, _ -> deleteData() }
                .setNegativeButton("Cancel", null)
                .show()
        }
        refresh()
    }

    override fun onResume() {
        super.onResume()
        if (!busy) refresh()
    }

    private fun hasData() =
        File(install, "android-manifest.json").isFile || File(install, "assets/private/game.json").isFile

    private fun refresh() {
        val has = hasData()
        status.text = if (has) "Game data found:\n$install" else "No game data. Import the zip exported from the PC setup.\n\nFolder: $install"
        playBtn.isEnabled = has && !busy
        deleteBtn.isEnabled = (install.exists() || staging.exists()) && !busy
        importBtn.isEnabled = !busy
    }

    private fun showError(msg: String) {
        error.text = msg
        error.visibility = if (msg.isEmpty()) View.GONE else View.VISIBLE
    }

    private fun setBusy(b: Boolean) {
        busy = b
        if (b) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        progress.visibility = if (b) View.VISIBLE else View.GONE
        refresh()
    }

    private fun deleteData() {
        thread {
            install.deleteRecursively()
            staging.deleteRecursively()
            old.deleteRecursively()
            runOnUiThread { showError(""); refresh() }
        }
    }

    private fun startImport(uri: Uri) {
        showError("")
        setBusy(true)
        progress.progress = 0
        thread {
            val result = try {
                doImport(uri)
            } catch (e: Throwable) {
                "Import failed: ${e.message ?: e.javaClass.simpleName}"
            }
            if (result.isNotEmpty()) staging.deleteRecursively()
            runOnUiThread {
                setBusy(false)
                showError(result)
            }
        }
    }

    private fun post(text: String, frac: Float) = runOnUiThread {
        status.text = text
        progress.progress = (frac * 1000).toInt()
    }

    private fun zipSize(uri: Uri): Long =
        contentResolver.query(uri, arrayOf(OpenableColumns.SIZE), null, null, null)?.use {
            if (it.moveToFirst() && !it.isNull(0)) it.getLong(0) else -1L
        } ?: -1L

    /** Returns "" on success, else an error message. */
    private fun doImport(uri: Uri): String {
        val root = filesDir0
        val size = zipSize(uri)
        if (size > 0) {
            val free = StatFs(root.path).availableBytes
            val need = (size * 1.05).toLong()
            if (free < need) return "Not enough free space: need ${mb(need)} MB, have ${mb(free)} MB."
        }
        staging.deleteRecursively()
        staging.mkdirs()
        val canon = staging.canonicalPath + File.separator
        var count = 0
        var read = 0L
        val buf = ByteArray(1 shl 16)
        val stream = contentResolver.openInputStream(uri) ?: return "Cannot open the selected file."
        val counting = object : java.io.FilterInputStream(stream.buffered(1 shl 20)) {
            override fun read(b: ByteArray, off: Int, len: Int): Int =
                super.read(b, off, len).also { if (it > 0) read += it }
        }
        ZipInputStream(counting).use { zin ->
            while (true) {
                val e = zin.nextEntry ?: break
                val name = e.name.replace('\\', '/')
                if (!name.startsWith("installation/")) continue
                val rel = name.removePrefix("installation/")
                if (rel.isEmpty()) continue
                val out = File(staging, rel)
                if (!out.canonicalPath.startsWith(canon)) return "Rejected unsafe path in zip: ${e.name}"
                if (e.isDirectory) {
                    out.mkdirs()
                    continue
                }
                out.parentFile?.mkdirs()
                out.outputStream().buffered(1 shl 16).use { o ->
                    while (true) {
                        val n = zin.read(buf)
                        if (n < 0) break
                        o.write(buf, 0, n)
                    }
                }
                count++
                if (count % 8 == 0 || size <= 0) {
                    val f = if (size > 0) (read.toFloat() / size).coerceIn(0f, 1f) else 0f
                    post("Importing: $count files (${mb(read)} MB)", f * 0.9f)
                }
            }
        }
        if (count == 0) return "The zip has no installation/ folder. Export it with the PC setup."
        post("Verifying files...", 0.9f)
        val sr = staging.absolutePath
        NativeBridge.verifyImport(sr).let { if (it.isNotEmpty()) return it }
        post("Checking assets...", 0.97f)
        NativeBridge.checkAssets(sr).let { if (it.isNotEmpty()) return it }
        post("Finalizing...", 0.99f)
        old.deleteRecursively()
        if (install.exists() && !install.renameTo(old)) return "Could not replace the existing game data."
        if (!staging.renameTo(install)) {
            old.renameTo(install)
            return "Could not move the imported data into place."
        }
        old.deleteRecursively()
        return ""
    }

    private fun mb(b: Long) = b / (1024 * 1024)
}
