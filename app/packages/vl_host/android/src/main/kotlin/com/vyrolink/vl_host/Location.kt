package com.vyrolink.vl_host

import android.content.Context
import android.content.Intent
import android.location.LocationManager
import android.net.Uri
import android.os.CancellationSignal
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull

/**
 * One GPS fix for "Share location" (ADR-034), from the platform location
 * service (no Google Play services). Called only after the user asked to
 * share and granted permission.
 */
internal object Location {
  suspend fun currentFix(context: Context): LocationInfo? = withTimeoutOrNull(20_000) {
    suspendCancellableCoroutine { cont ->
      val lm = context.getSystemService(LocationManager::class.java)
      val provider = listOf(LocationManager.FUSED_PROVIDER, LocationManager.GPS_PROVIDER, LocationManager.NETWORK_PROVIDER)
        .firstOrNull { p -> runCatching { lm?.isProviderEnabled(p) == true }.getOrDefault(false) }
      if (lm == null || provider == null) {
        cont.resume(null)
        return@suspendCancellableCoroutine
      }
      val signal = CancellationSignal()
      cont.invokeOnCancellation { signal.cancel() }
      try {
        lm.getCurrentLocation(provider, signal, context.mainExecutor) { loc ->
          if (!cont.isActive) return@getCurrentLocation
          cont.resume(
            loc?.let {
              LocationInfo(
                latE7 = Math.round(it.latitude * 1e7),
                lonE7 = Math.round(it.longitude * 1e7),
                accuracyM = it.accuracy.toLong().coerceIn(0, 65_535),
                capturedMs = it.time,
              )
            },
          )
        }
      } catch (e: SecurityException) {
        if (cont.isActive) cont.resume(null)
      }
    }
  }

  fun openMap(context: Context, latE7: Long, lonE7: Long, label: String) {
    val lat = latE7 / 1e7
    val lon = lonE7 / 1e7
    val uri = Uri.parse("geo:$lat,$lon?q=$lat,$lon(${Uri.encode(label)})")
    runCatching {
      context.startActivity(Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }
  }
}
