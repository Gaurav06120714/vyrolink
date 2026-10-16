package com.vyrolink.vl_host

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.net.Uri
import android.os.Build
import android.os.CancellationSignal
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.view.WindowManager
import android.webkit.MimeTypeMap
import androidx.core.content.FileProvider
import io.flutter.plugin.common.PluginRegistry
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.UUID
import kotlin.coroutines.resume
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext

/**
 * Pickers, the share sheet, the app-lock prompt and screenshot blocking.
 * Uses only platform APIs plus androidx FileProvider; no photo or storage
 * permission is needed because the system pickers hand over single items.
 */
internal class SystemUi(private val context: Context) : PluginRegistry.ActivityResultListener {
  companion object {
    const val MAX_IMAGE_BYTES = 16 * 1024
    const val MAX_FILE_BYTES = 64 * 1024
    private const val PICK_IMAGE = 0x5701
    private const val PICK_FILE = 0x5702
  }

  var activity: Activity? = null
    set(value) {
      field = value
      applyScreenCapture()
    }

  /** Release builds block screenshots unless the user turned it off. */
  var blockScreenCapture: Boolean = true
    set(value) {
      field = value
      applyScreenCapture()
    }

  private val pending = mutableMapOf<Int, (Uri?) -> Unit>()

  private fun applyScreenCapture() {
    val a = activity ?: return
    a.runOnUiThread {
      if (blockScreenCapture) {
        a.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
      } else {
        a.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
      }
    }
  }

  override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
    val done = pending.remove(requestCode) ?: return false
    done(if (resultCode == Activity.RESULT_OK) data?.data else null)
    return true
  }

  private suspend fun pick(intent: Intent, code: Int): Uri? = suspendCancellableCoroutine { cont ->
    val a = activity
    if (a == null) {
      cont.resume(null)
      return@suspendCancellableCoroutine
    }
    pending[code] = { cont.resume(it) }
    a.runOnUiThread { a.startActivityForResult(intent, code) }
  }

  // Images

  suspend fun pickImage(): PickedFile? {
    val intent = if (Build.VERSION.SDK_INT >= 33) {
      Intent(MediaStore.ACTION_PICK_IMAGES)
    } else {
      Intent(Intent.ACTION_GET_CONTENT).setType("image/*").addCategory(Intent.CATEGORY_OPENABLE)
    }
    val uri = pick(intent, PICK_IMAGE) ?: return null
    return withContext(Dispatchers.Default) {
      val jpeg = runCatching { compress(uri) }.getOrNull() ?: return@withContext null
      PickedFile("photo.jpg", jpeg.size.toLong(), jpeg)
    }
  }

  /**
   * Decodes at most 640 px on the long side (ImageDecoder applies EXIF
   * orientation), then recompresses until the JPEG fits 16 KiB. The output is
   * re-encoded from pixels, so no metadata such as location survives.
   */
  private fun compress(uri: Uri): ByteArray? {
    var side = 640
    while (side >= 120) {
      val source = ImageDecoder.createSource(context.contentResolver, uri)
      val target = side
      val bitmap = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
        val w = info.size.width
        val h = info.size.height
        val scale = minOf(1f, target.toFloat() / maxOf(w, h))
        decoder.setTargetSize(maxOf(1, (w * scale).toInt()), maxOf(1, (h * scale).toInt()))
        decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
      }
      for (quality in 70 downTo 20 step 10) {
        val out = ByteArrayOutputStream()
        bitmap.compress(Bitmap.CompressFormat.JPEG, quality, out)
        if (out.size() <= MAX_IMAGE_BYTES) {
          bitmap.recycle()
          return out.toByteArray()
        }
      }
      bitmap.recycle()
      side = side * 3 / 4
    }
    return null
  }

  // Files

  suspend fun pickFile(): PickedFile? {
    val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE)
    val uri = pick(intent, PICK_FILE) ?: return null
    return withContext(Dispatchers.IO) {
      var name = "file"
      var size = -1L
      context.contentResolver.query(uri, null, null, null, null)?.use { c ->
        if (c.moveToFirst()) {
          c.getColumnIndex(OpenableColumns.DISPLAY_NAME).takeIf { it >= 0 }?.let { name = c.getString(it) ?: name }
          c.getColumnIndex(OpenableColumns.SIZE).takeIf { it >= 0 }?.let { if (!c.isNull(it)) size = c.getLong(it) }
        }
      }
      name = name.take(64)
      if (size > MAX_FILE_BYTES) return@withContext PickedFile(name, size, null)
      // Read at most one byte past the limit; never trust the reported size.
      val bytes = context.contentResolver.openInputStream(uri)?.use { input ->
        val buf = ByteArray(MAX_FILE_BYTES + 1)
        var total = 0
        while (total < buf.size) {
          val n = input.read(buf, total, buf.size - total)
          if (n < 0) break
          total += n
        }
        buf.copyOf(total)
      } ?: return@withContext null
      if (bytes.isEmpty() || bytes.size > MAX_FILE_BYTES) {
        PickedFile(name, maxOf(size, bytes.size.toLong()), null)
      } else {
        PickedFile(name, bytes.size.toLong(), bytes)
      }
    }
  }

  // Share

  suspend fun share(name: String, data: ByteArray) {
    val a = activity ?: return
    val file = withContext(Dispatchers.IO) {
      val root = File(context.cacheDir, "shared")
      root.listFiles()?.forEach { it.deleteRecursively() }
      val dir = File(root, UUID.randomUUID().toString()).apply { mkdirs() }
      val safe = name.replace('/', '_').replace(':', '_').ifBlank { "attachment" }
      File(dir, safe).apply { writeBytes(data) }
    }
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.vlfiles", file)
    val ext = file.extension.lowercase()
    val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext) ?: "application/octet-stream"
    val send = Intent(Intent.ACTION_SEND)
      .setType(mime)
      .putExtra(Intent.EXTRA_STREAM, uri)
      .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    a.runOnUiThread { a.startActivity(Intent.createChooser(send, null)) }
  }

  // App lock

  private val authenticators =
    BiometricManager.Authenticators.BIOMETRIC_WEAK or BiometricManager.Authenticators.DEVICE_CREDENTIAL

  fun canAuthenticate(): Boolean =
    context.getSystemService(BiometricManager::class.java)?.canAuthenticate(authenticators) ==
      BiometricManager.BIOMETRIC_SUCCESS

  /** Fingerprint/face with the device PIN, pattern or password as fallback. */
  suspend fun authenticate(reason: String): Boolean = suspendCancellableCoroutine { cont ->
    val a = activity
    if (a == null) {
      cont.resume(false)
      return@suspendCancellableCoroutine
    }
    a.runOnUiThread {
      val prompt = BiometricPrompt.Builder(a)
        .setTitle(reason)
        .setAllowedAuthenticators(authenticators)
        .build()
      prompt.authenticate(CancellationSignal(), a.mainExecutor, object : BiometricPrompt.AuthenticationCallback() {
        override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
          if (cont.isActive) cont.resume(true)
        }

        override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
          if (cont.isActive) cont.resume(false)
        }
      })
    }
  }
}
