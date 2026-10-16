package com.vyrolink.vl_host

import android.content.pm.ApplicationInfo
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.embedding.engine.plugins.activity.ActivityAware
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding
import java.util.concurrent.Executors
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * Registers the host API, the QR scanner view and the Bluetooth relay. The
 * node lives as long as the process.
 */
class VlHostPlugin : FlutterPlugin, ActivityAware {
  companion object {
    @Volatile
    var inForeground = false
  }

  private lateinit var host: NodeHost
  private lateinit var permissions: Permissions
  private lateinit var ui: SystemUi
  private var binding: ActivityPluginBinding? = null
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
  private val background = Executors.newSingleThreadExecutor()

  override fun onAttachedToEngine(b: FlutterPlugin.FlutterPluginBinding) {
    val context = b.applicationContext
    val events = VlEventsApi(b.binaryMessenger)
    permissions = Permissions(context)
    ui = SystemUi(context)
    host = NodeHost(context, permissions, ui)
    host.relay = BleRelay(
      context, permissions, { host.node },
      onDataChanged = { scope.launch { runCatching { events.dataChanged() } } },
      onStatus = { s -> scope.launch { runCatching { events.networkChanged(s) } } },
    )
    host.openIfProvisioned()
    host.node?.let { n ->
      if (runCatching { n.hasIdentity() && n.setting(NodeHost.RELAY_SETTING) == "1" }.getOrDefault(false)) {
        host.relay?.setMode(host.savedMode())
        host.relay?.setEnabled(true)
      }
    }
    VlHostApi.setUp(b.binaryMessenger, host)
    b.platformViewRegistry.registerViewFactory(
      "vl_host/qr_scanner",
      QrScannerFactory({ binding?.activity as? LifecycleOwner }) { text -> scope.launch { runCatching { events.qrScanned(text) } } },
    )
    ProcessLifecycleOwner.get().lifecycle.addObserver(object : DefaultLifecycleObserver {
      override fun onStart(owner: LifecycleOwner) {
        inForeground = true
        background.execute {
          runCatching { host.maintain() }
          val on = runCatching { host.node?.setting(NodeHost.RELAY_SETTING) == "1" }.getOrDefault(false)
          if (on) host.applyRelayService(true)
        }
      }

      override fun onStop(owner: LifecycleOwner) {
        inForeground = false
      }
    })
  }

  override fun onDetachedFromEngine(b: FlutterPlugin.FlutterPluginBinding) {
    VlHostApi.setUp(b.binaryMessenger, null)
  }

  override fun onAttachedToActivity(b: ActivityPluginBinding) {
    binding = b
    permissions.activity = b.activity
    b.addRequestPermissionsResultListener(permissions)
    b.addActivityResultListener(ui)
    // Screen-capture protection (Doc 16 screen 11) is on by default in release
    // builds and off in debug builds (test screenshots); the Privacy screen
    // overrides it.
    val debuggable = (b.activity.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0
    ui.blockScreenCapture = !debuggable
    ui.activity = b.activity
  }

  override fun onDetachedFromActivity() {
    binding?.removeRequestPermissionsResultListener(permissions)
    binding?.removeActivityResultListener(ui)
    binding = null
    permissions.activity = null
    ui.activity = null
  }

  override fun onReattachedToActivityForConfigChanges(b: ActivityPluginBinding) = onAttachedToActivity(b)

  override fun onDetachedFromActivityForConfigChanges() = onDetachedFromActivity()
}
