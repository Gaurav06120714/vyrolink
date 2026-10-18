package com.vyrolink.vl_host

import android.Manifest
import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import io.flutter.plugin.common.PluginRegistry
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * Runtime permissions, requested only when a feature needs them (Doc 16
 * screen 13): Bluetooth when the relay is turned on, camera on the pairing
 * screen, notifications from Settings, and location only when the user taps
 * "Share location" (ADR-034). Bluetooth never needs location
 * (BLUETOOTH_SCAN is declared `neverForLocation`).
 */
internal class Permissions(private val context: Context) : PluginRegistry.RequestPermissionsResultListener {
  var activity: Activity? = null
  private val pending = mutableMapOf<Int, (Boolean) -> Unit>()
  private var nextCode = 0x5600

  private fun granted(p: String) = context.checkSelfPermission(p) == PackageManager.PERMISSION_GRANTED

  val bluetooth = arrayOf(
    Manifest.permission.BLUETOOTH_SCAN,
    Manifest.permission.BLUETOOTH_CONNECT,
    Manifest.permission.BLUETOOTH_ADVERTISE,
  )

  fun hasBluetooth() = bluetooth.all(::granted)

  private fun request(perms: Array<String>, done: (Boolean) -> Unit) {
    if (perms.all(::granted)) return done(true)
    val a = activity ?: return done(false)
    val code = nextCode++
    pending[code] = done
    a.runOnUiThread { a.requestPermissions(perms, code) }
  }

  fun requestBluetooth(done: () -> Unit) = request(bluetooth) { done() }

  fun camera(): PermissionState = when {
    granted(Manifest.permission.CAMERA) -> PermissionState.GRANTED
    activity?.shouldShowRequestPermissionRationale(Manifest.permission.CAMERA) == true -> PermissionState.DENIED
    else -> PermissionState.NOT_DETERMINED
  }

  suspend fun requestCamera(): PermissionState = suspendCancellableCoroutine { cont ->
    request(arrayOf(Manifest.permission.CAMERA)) { ok ->
      cont.resume(if (ok) PermissionState.GRANTED else PermissionState.DENIED)
    }
  }

  suspend fun requestNotifications(): PermissionState {
    if (Build.VERSION.SDK_INT < 33) return PermissionState.GRANTED
    return suspendCancellableCoroutine { cont ->
      request(arrayOf(Manifest.permission.POST_NOTIFICATIONS)) { ok ->
        cont.resume(if (ok) PermissionState.GRANTED else PermissionState.DENIED)
      }
    }
  }

  private val location = arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)

  fun hasLocation() = location.any(::granted)

  /** True if precise or approximate location is allowed afterwards. */
  suspend fun requestLocation(): Boolean = suspendCancellableCoroutine { cont ->
    request(location) { cont.resume(hasLocation()) }
  }

  fun openAppSettings() {
    val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
      .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    context.startActivity(intent)
  }

  override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray): Boolean {
    val done = pending.remove(requestCode) ?: return false
    done(grantResults.isNotEmpty() && grantResults.all { it == PackageManager.PERMISSION_GRANTED })
    return true
  }
}

/** A generic notification; never the sender or the text (Doc 16 §11 default). */
internal object Notifier {
  private const val CHANNEL = "messages"
  private const val ID = 1
  private const val NEARBY_ID = 3

  fun newMessage(context: Context) = post(context, ID, "You have a new message.")

  /** ADR-029: never names the contact. */
  fun contactNearby(context: Context) = post(context, NEARBY_ID, "A contact is nearby.")

  private fun post(context: Context, id: Int, text: String) {
    if (Build.VERSION.SDK_INT >= 33 &&
      context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) return
    val nm = context.getSystemService(NotificationManager::class.java) ?: return
    nm.createNotificationChannel(NotificationChannel(CHANNEL, "Messages", NotificationManager.IMPORTANCE_DEFAULT))
    val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
    val tap = launch?.let {
      android.app.PendingIntent.getActivity(context, 0, it, android.app.PendingIntent.FLAG_IMMUTABLE)
    }
    val n = Notification.Builder(context, CHANNEL)
      .setSmallIcon(android.R.drawable.stat_notify_chat)
      .setContentTitle("VYRO LINK")
      .setContentText(text)
      .setVisibility(Notification.VISIBILITY_PRIVATE)
      .setAutoCancel(true)
      .apply { tap?.let { setContentIntent(it) } }
      .build()
    nm.notify(id, n)
  }

  fun cancelAll(context: Context) {
    context.getSystemService(NotificationManager::class.java)?.cancelAll()
  }
}
