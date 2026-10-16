package com.vyrolink.vl_host

import android.content.Context
import android.view.View
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.lifecycle.LifecycleOwner
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import io.flutter.plugin.common.StandardMessageCodec
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory
import java.util.concurrent.Executors

/**
 * Camera preview that reports QR text (pairing codes). Decoding uses ZXing
 * on the device; frames are never stored or sent anywhere. The camera runs
 * only while this view exists.
 */
internal class QrScanner(
  context: Context,
  private val owner: LifecycleOwner?,
  private val onCode: (String) -> Unit,
) : PlatformView {
  private val preview = PreviewView(context).apply { implementationMode = PreviewView.ImplementationMode.COMPATIBLE }
  private val executor = Executors.newSingleThreadExecutor()
  private val reader = QRCodeReader()
  private val hints = mapOf(DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE))
  private var provider: ProcessCameraProvider? = null
  private var lastCode: String? = null
  private var lastAt = 0L

  init {
    val future = ProcessCameraProvider.getInstance(context)
    future.addListener({
      val p = runCatching { future.get() }.getOrNull() ?: return@addListener
      val o = owner ?: return@addListener
      provider = p
      val previewUse = Preview.Builder().build().also { it.setSurfaceProvider(preview.surfaceProvider) }
      val analysis = ImageAnalysis.Builder()
        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
        .build()
        .also { it.setAnalyzer(executor, ::analyze) }
      runCatching {
        p.unbindAll()
        p.bindToLifecycle(o, CameraSelector.DEFAULT_BACK_CAMERA, previewUse, analysis)
      }
    }, context.mainExecutor)
  }

  private fun analyze(image: ImageProxy) {
    image.use {
      val plane = it.planes[0]
      val buffer = plane.buffer
      val bytes = ByteArray(buffer.remaining()).also(buffer::get)
      val source = PlanarYUVLuminanceSource(bytes, plane.rowStride, it.height, 0, 0, it.width, it.height, false)
      val text = runCatching { reader.decode(BinaryBitmap(HybridBinarizer(source)), hints).text }.getOrNull()
      reader.reset()
      if (text != null) {
        val t = System.currentTimeMillis()
        // The same code stays in view for a while; report it once per 2 s.
        if (text == lastCode && t - lastAt < 2_000) return
        lastCode = text
        lastAt = t
        preview.post { onCode(text) }
      }
    }
  }

  override fun getView(): View = preview

  override fun dispose() {
    provider?.unbindAll()
    executor.shutdown()
  }
}

internal class QrScannerFactory(
  private val owner: () -> LifecycleOwner?,
  private val onCode: (String) -> Unit,
) : PlatformViewFactory(StandardMessageCodec.INSTANCE) {
  override fun create(context: Context, viewId: Int, args: Any?): PlatformView = QrScanner(context, owner(), onCode)
}
