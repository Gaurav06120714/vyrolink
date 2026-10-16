package com.vyrolink.vl_host

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder

/**
 * "Always relay" mode (Doc 16 screen 12): a `connectedDevice` foreground
 * service that keeps the process, and so the Bluetooth relay in [BleRelay],
 * alive in the background. Android requires the visible notification. The
 * service does no work itself.
 */
class RelayService : Service() {
  companion object {
    private const val CHANNEL = "relay"
    private const val ID = 2

    fun start(context: Context) {
      runCatching { context.startForegroundService(Intent(context, RelayService::class.java)) }
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, RelayService::class.java))
    }
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val nm = getSystemService(NotificationManager::class.java)
    nm?.createNotificationChannel(NotificationChannel(CHANNEL, "Relaying", NotificationManager.IMPORTANCE_LOW))
    val open = packageManager.getLaunchIntentForPackage(packageName)?.let {
      PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_IMMUTABLE)
    }
    val notification = Notification.Builder(this, CHANNEL)
      .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
      .setContentTitle("VYRO LINK is relaying")
      .setContentText("Carrying encrypted messages for nearby phones. Change this in Network.")
      .setOngoing(true)
      .apply { open?.let { setContentIntent(it) } }
      .build()
    // Must be called promptly after startForegroundService(); callers only start
    // this service when Bluetooth permission is granted, which the type needs.
    val started = runCatching {
      startForeground(ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
    }.isSuccess
    if (!started) {
      stopSelf()
      return START_NOT_STICKY
    }
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? = null
}
