package com.vyrolink.vl_host

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothGattServer
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothGattService
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.ParcelUuid
import android.os.SystemClock
import java.io.ByteArrayOutputStream
import java.util.UUID
import uniffi.vl_ffi.NodeStats
import uniffi.vl_ffi.VlNode

/** Doc 21 L0: the only thing VYRO LINK advertises. No name, no identifiers. */
internal val SERVICE_UUID: UUID = UUID.fromString("3B29F840-0F6B-4221-9AEB-88FF70CCF5CD")

/** The central writes (without response), the peripheral notifies. */
internal val LINK_UUID: UUID = UUID.fromString("60EAE795-7B2A-49EE-B8C2-7430FF5FF590")
private val CCCD_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

/**
 * Dual-role Bluetooth transport. Rust decides what to send (ADR-022); this
 * class only moves bytes, slices them to the negotiated MTU, waits for the
 * platform's flow control and ends sessions once both sides go quiet
 * (research/algorithms/06). All state is touched on one handler thread.
 * Android may stop this work in the background; no foreground service is used
 * yet, so relaying is reliable only while the app is in use.
 */
@SuppressLint("MissingPermission")
internal class BleRelay(
  private val context: Context,
  private val permissions: Permissions,
  private val node: () -> VlNode?,
  private val onDataChanged: () -> Unit,
  private val onStatus: (NetworkStatus) -> Unit,
) {
  private companion object {
    const val IDLE_MS = 4_000L
    const val SESSION_LIMIT_MS = 60_000L
    const val MAX_CENTRAL_LINKS = 2
    const val MAINTAIN_EVERY_MS = 15 * 60_000L
    const val ATT_OVERHEAD = 3
  }

  private val thread = HandlerThread("vl-ble").apply { start() }
  private val handler = Handler(thread.looper)
  private val manager = context.getSystemService(BluetoothManager::class.java)
  private val adapter: BluetoothAdapter? get() = manager?.adapter

  private class Session(val linkId: ULong) {
    val started = SystemClock.elapsedRealtime()
    var lastActivity = started
    val outbox = ByteArrayOutputStream()
    var pending = ByteArray(0)
    var busy = false
    var exchanged = false
    var mtu = 23
    var gatt: BluetoothGatt? = null
    var characteristic: BluetoothGattCharacteristic? = null
    var device: BluetoothDevice? = null
  }

  private var enabled = false

  /** 1 = relay only while charging, 2 = always. */
  private var mode = 2
  private var charging = false

  /** Last time each (rotating) peer address was seen; only the count leaves this class. */
  private val seenAt = mutableMapOf<String, Long>()
  private var scanning = false
  private var advertising = false
  private var server: BluetoothGattServer? = null
  private var serverCharacteristic: BluetoothGattCharacteristic? = null
  private val asCentral = mutableMapOf<String, Session>()
  private val connecting = mutableMapOf<String, BluetoothGatt>()
  private val asPeripheral = mutableMapOf<String, Session>()
  private val serverMtu = mutableMapOf<String, Int>()
  private val cooldownUntil = mutableMapOf<String, Long>()
  private var encounters = 0
  private var lastExchangeSecs: Long? = null
  private var lastMaintain = 0L
  private var lastUnread: Int? = null
  private var changePending = false

  private val stateReceiver = object : BroadcastReceiver() {
    override fun onReceive(c: Context, i: Intent) {
      handler.post {
        when (i.action) {
          Intent.ACTION_POWER_CONNECTED -> charging = true
          Intent.ACTION_POWER_DISCONNECTED -> charging = false
        }
        applyRadio()
        publish()
      }
    }
  }

  init {
    val filter = IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED).apply {
      addAction(Intent.ACTION_POWER_CONNECTED)
      addAction(Intent.ACTION_POWER_DISCONNECTED)
    }
    context.registerReceiver(stateReceiver, filter)
    charging = context.getSystemService(android.os.BatteryManager::class.java)?.isCharging == true
    handler.post(::tick)
  }

  /** Radio on when enabled, Bluetooth on, and either "always" or charging. */
  private fun applyRadio() {
    if (enabled && adapter?.isEnabled == true && (mode == 2 || charging)) startRadio() else stopRadio(keepEnabled = true)
  }

  fun setMode(m: Int) = handler.post {
    mode = if (m == 1) 1 else 2
    applyRadio()
    publish()
  }

  private fun now() = (System.currentTimeMillis() / 1000).toULong()

  // Control

  fun setEnabled(on: Boolean) = handler.post {
    enabled = on
    if (on) applyRadio() else stopRadio(keepEnabled = false)
    publish()
  }

  /** Stops everything and waits (panic wipe). */
  fun shutdown() {
    val done = java.util.concurrent.CountDownLatch(1)
    handler.post {
      enabled = false
      stopRadio(keepEnabled = false)
      done.countDown()
    }
    done.await(5, java.util.concurrent.TimeUnit.SECONDS)
  }

  fun outboxChanged() = handler.post { cooldownUntil.clear() }

  fun notifyDataChanged() = onDataChanged()

  fun status(): NetworkStatus {
    val stats = runCatching { node()?.stats() }.getOrNull()
    val lock = java.util.concurrent.FutureTask { snapshot(stats) }
    handler.post(lock)
    return lock.get(2, java.util.concurrent.TimeUnit.SECONDS)
  }

  private fun radio(): RadioState = when {
    adapter == null -> RadioState.UNSUPPORTED
    !permissions.hasBluetooth() -> if (enabled) RadioState.UNAUTHORIZED else RadioState.UNKNOWN
    adapter?.isEnabled != true -> RadioState.POWERED_OFF
    else -> RadioState.POWERED_ON
  }

  private fun snapshot(stats: NodeStats?) = NetworkStatus(
    radio = radio(),
    relayEnabled = enabled,
    relayMode = mode.toLong(),
    charging = charging,
    devicesSeenRecently = seenAt.values.count { it > SystemClock.elapsedRealtime() - 86_400_000L }.toLong(),
    scanning = scanning,
    advertising = advertising,
    openLinks = (asCentral.size + asPeripheral.size).toLong(),
    encountersSinceLaunch = encounters.toLong(),
    lastExchangeSecs = lastExchangeSecs,
    ownEnvelopes = stats?.ownEnvelopes?.toLong() ?: 0,
    carriedEnvelopes = stats?.carriedEnvelopes?.toLong() ?: 0,
    carriedBytes = stats?.carriedBytes?.toLong() ?: 0,
  )

  private fun publish() {
    val s = snapshot(runCatching { node()?.stats() }.getOrNull())
    onStatus(s)
  }

  private fun startRadio() {
    val a = adapter ?: return
    if (!enabled || !a.isEnabled || !permissions.hasBluetooth() || node() == null) return
    if (server == null) openServer()
    if (!advertising) {
      a.bluetoothLeAdvertiser?.startAdvertising(
        AdvertiseSettings.Builder()
          // Fast beacons (ADR-032): about 10 adverts a second so phones find each other quickly.
          .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
          .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM)
          .setConnectable(true)
          .setTimeout(0)
          .build(),
        AdvertiseData.Builder().addServiceUuid(ParcelUuid(SERVICE_UUID)).setIncludeDeviceName(false).build(),
        advertiseCallback,
      )
    }
    if (!scanning) {
      // One long filtered scan: survives screen-off and avoids the scan-start throttle.
      a.bluetoothLeScanner?.startScan(
        listOf(ScanFilter.Builder().setServiceUuid(ParcelUuid(SERVICE_UUID)).build()),
        ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build(),
        scanCallback,
      )
      scanning = true
    }
  }

  private fun stopRadio(keepEnabled: Boolean) {
    if (!keepEnabled) enabled = false
    runCatching { if (scanning) adapter?.bluetoothLeScanner?.stopScan(scanCallback) }
    runCatching { if (advertising) adapter?.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback) }
    scanning = false
    advertising = false
    connecting.values.forEach { runCatching { it.close() } }
    connecting.clear()
    asCentral.toMap().forEach { (k, s) -> s.gatt?.let { runCatching { it.disconnect(); it.close() } }; finish(k, s, true) }
    asPeripheral.toMap().forEach { (k, s) -> finish(k, s, false) }
    runCatching { server?.close() }
    server = null
    serverCharacteristic = null
  }

  /** Ends quiet or overlong sessions and runs periodic maintenance. */
  private fun tick() {
    val t = SystemClock.elapsedRealtime()
    val n = node()
    asCentral.toMap().forEach { (k, s) ->
      val closed = n?.linkIsOpen(s.linkId) != true
      val quiet = s.outbox.size() == 0 && s.pending.isEmpty() && t - s.lastActivity > IDLE_MS
      if ((closed && s.outbox.size() == 0) || quiet || t - s.started > SESSION_LIMIT_MS) {
        s.gatt?.disconnect()
      }
    }
    asPeripheral.toMap().forEach { (k, s) -> if (t - s.lastActivity > SESSION_LIMIT_MS) finish(k, s, false) }
    if (n != null && t - lastMaintain > MAINTAIN_EVERY_MS) {
      lastMaintain = t
      runCatching { n.maintain(now()) }
    }
    handler.postDelayed(::tick, 1_000)
  }

  // Byte pump

  private fun receive(s: Session, bytes: ByteArray) {
    val n = node() ?: return
    s.lastActivity = SystemClock.elapsedRealtime()
    try {
      val reply = n.linkReceive(s.linkId, bytes, now())
      s.exchanged = true
      s.outbox.write(reply)
      pump(s)
      scheduleChange()
    } catch (e: Exception) {
      // Protocol or crypto failure: Rust closed the link; drop the connection.
      s.outbox.reset()
      s.gatt?.disconnect()
    }
  }

  private fun nextChunk(s: Session): ByteArray? {
    if (s.pending.isEmpty() && s.outbox.size() > 0) {
      s.pending = s.outbox.toByteArray()
      s.outbox.reset()
    }
    if (s.pending.isEmpty()) return null
    val size = minOf(s.pending.size, maxOf(20, s.mtu - ATT_OVERHEAD))
    val chunk = s.pending.copyOfRange(0, size)
    s.pending = s.pending.copyOfRange(size, s.pending.size)
    return chunk
  }

  /** One write or notification in flight per link; the callback sends the next. */
  @Suppress("DEPRECATION")
  private fun pump(s: Session) {
    if (s.busy) return
    val chunk = nextChunk(s) ?: return
    s.busy = true
    s.lastActivity = SystemClock.elapsedRealtime()
    val gatt = s.gatt
    val c = s.characteristic
    val ok = if (gatt != null && c != null) {
      if (Build.VERSION.SDK_INT >= 33) {
        gatt.writeCharacteristic(c, chunk, BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE) == BluetoothGatt.GATT_SUCCESS
      } else {
        c.writeType = BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
        c.value = chunk
        gatt.writeCharacteristic(c)
      }
    } else {
      val d = s.device
      val sc = serverCharacteristic
      val srv = server
      if (d == null || sc == null || srv == null) {
        false
      } else if (Build.VERSION.SDK_INT >= 33) {
        srv.notifyCharacteristicChanged(d, sc, false, chunk) == BluetoothGatt.GATT_SUCCESS
      } else {
        sc.value = chunk
        srv.notifyCharacteristicChanged(d, sc, false)
      }
    }
    if (!ok) {
      // Busy stack: put the chunk back and retry shortly.
      s.pending = chunk + s.pending
      s.busy = false
      handler.postDelayed({ pump(s) }, 20)
    }
  }

  private fun sent(s: Session) {
    s.busy = false
    pump(s)
  }

  private fun finish(key: String, s: Session, asCentral: Boolean) {
    node()?.linkClose(s.linkId)
    if (asCentral) {
      this.asCentral.remove(key)
      // ADR-032: the user's re-sync interval (1–45 s, default 5).
      val resyncMs = (node()?.resyncSecs()?.toLong() ?: 5L) * 1_000L
      cooldownUntil[key] = SystemClock.elapsedRealtime() + resyncMs
    } else {
      asPeripheral.remove(key)
    }
    if (s.exchanged) {
      encounters++
      lastExchangeSecs = System.currentTimeMillis() / 1000
      notifyIfNew()
    }
    scheduleChange()
  }

  private fun scheduleChange() {
    if (changePending) return
    changePending = true
    handler.postDelayed({
      changePending = false
      onDataChanged()
      publish()
    }, 500)
  }

  private fun notifyIfNew() {
    // ADR-029: a paired contact came into range. Says nobody's name.
    val arrived = runCatching { node()?.takeNearbyArrivals() ?: 0u }.getOrDefault(0u)
    if (arrived > 0u && !VlHostPlugin.inForeground) Notifier.contactNearby(context)
    val unread = runCatching { node()?.conversationSummaries()?.sumOf { it.unread.toInt() } }.getOrNull() ?: return
    val before = lastUnread
    lastUnread = unread
    if (before != null && unread > before && !VlHostPlugin.inForeground) Notifier.newMessage(context)
  }

  // Central role

  private val scanCallback = object : ScanCallback() {
    override fun onScanResult(callbackType: Int, result: ScanResult) {
      handler.post { discovered(result.device) }
    }

    override fun onScanFailed(errorCode: Int) {
      handler.post {
        scanning = false
        publish()
      }
    }
  }

  private fun discovered(device: BluetoothDevice) {
    val key = device.address
    val t = SystemClock.elapsedRealtime()
    seenAt[key] = t
    if (!enabled || node() == null || key in asCentral || key in connecting || key in asPeripheral) return
    if (asCentral.size + connecting.size >= MAX_CENTRAL_LINKS || (cooldownUntil[key] ?: 0) > t) return
    connecting[key] = device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE) ?: return
  }

  private val gattCallback = object : BluetoothGattCallback() {
    override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
      handler.post {
        val key = gatt.device.address
        if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
          gatt.requestMtu(517)
        } else {
          connecting.remove(key)
          asCentral[key]?.let { finish(key, it, true) } ?: run {
            cooldownUntil[key] = SystemClock.elapsedRealtime() + 10_000
          }
          gatt.close()
        }
      }
    }

    override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) {
      handler.post {
        serverMtu[gatt.device.address] = if (status == BluetoothGatt.GATT_SUCCESS) mtu else 23
        gatt.discoverServices()
      }
    }

    override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
      handler.post {
        val c = gatt.getService(SERVICE_UUID)?.getCharacteristic(LINK_UUID)
        val cccd = c?.getDescriptor(CCCD_UUID)
        if (status != BluetoothGatt.GATT_SUCCESS || c == null || cccd == null) return@post gatt.disconnect()
        gatt.setCharacteristicNotification(c, true)
        @Suppress("DEPRECATION")
        val ok = if (Build.VERSION.SDK_INT >= 33) {
          gatt.writeDescriptor(cccd, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE) == BluetoothGatt.GATT_SUCCESS
        } else {
          cccd.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
          gatt.writeDescriptor(cccd)
        }
        if (!ok) gatt.disconnect()
      }
    }

    override fun onDescriptorWrite(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor, status: Int) {
      handler.post {
        val key = gatt.device.address
        val n = node()
        val start = if (status == BluetoothGatt.GATT_SUCCESS && n != null) runCatching { n.linkOpen(true) }.getOrNull() else null
        if (start == null) return@post gatt.disconnect()
        connecting.remove(key)
        val s = Session(start.linkId).apply {
          this.gatt = gatt
          characteristic = descriptor.characteristic
          mtu = serverMtu[key] ?: 23
          outbox.write(start.bytesToSend)
        }
        asCentral[key] = s
        pump(s)
        publish()
      }
    }

    override fun onCharacteristicWrite(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) {
      handler.post { asCentral[gatt.device.address]?.let(::sent) }
    }

    override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, value: ByteArray) {
      handler.post { asCentral[gatt.device.address]?.let { receive(it, value) } }
    }

    @Deprecated("Android 12 delivers notifications here")
    override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
      if (Build.VERSION.SDK_INT >= 33) return
      @Suppress("DEPRECATION")
      val value = characteristic.value?.copyOf() ?: return
      handler.post { asCentral[gatt.device.address]?.let { receive(it, value) } }
    }
  }

  // Peripheral role

  private val advertiseCallback = object : AdvertiseCallback() {
    override fun onStartSuccess(settingsInEffect: AdvertiseSettings) {
      handler.post {
        advertising = true
        publish()
      }
    }

    override fun onStartFailure(errorCode: Int) {
      // Some phones cannot advertise; this phone then works as a client only.
      handler.post {
        advertising = false
        publish()
      }
    }
  }

  private fun openServer() {
    val srv = manager?.openGattServer(context, serverCallback) ?: return
    val c = BluetoothGattCharacteristic(
      LINK_UUID,
      BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE or BluetoothGattCharacteristic.PROPERTY_NOTIFY,
      BluetoothGattCharacteristic.PERMISSION_WRITE,
    )
    c.addDescriptor(
      BluetoothGattDescriptor(CCCD_UUID, BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE),
    )
    val service = BluetoothGattService(SERVICE_UUID, BluetoothGattService.SERVICE_TYPE_PRIMARY)
    service.addCharacteristic(c)
    srv.addService(service)
    server = srv
    serverCharacteristic = c
  }

  private fun responder(device: BluetoothDevice): Session? {
    asPeripheral[device.address]?.let { return it }
    seenAt[device.address] = SystemClock.elapsedRealtime()
    if (!enabled) return null
    val start = runCatching { node()?.linkOpen(false) }.getOrNull() ?: return null
    val s = Session(start.linkId).apply {
      this.device = device
      mtu = serverMtu[device.address] ?: 23
      outbox.write(start.bytesToSend)
    }
    asPeripheral[device.address] = s
    publish()
    return s
  }

  private val serverCallback = object : BluetoothGattServerCallback() {
    override fun onConnectionStateChange(device: BluetoothDevice, status: Int, newState: Int) {
      if (newState == BluetoothProfile.STATE_DISCONNECTED) {
        handler.post {
          serverMtu.remove(device.address)
          asPeripheral[device.address]?.let { finish(device.address, it, false) }
        }
      }
    }

    override fun onMtuChanged(device: BluetoothDevice, mtu: Int) {
      handler.post {
        serverMtu[device.address] = mtu
        asPeripheral[device.address]?.mtu = mtu
      }
    }

    override fun onDescriptorWriteRequest(
      device: BluetoothDevice, requestId: Int, descriptor: BluetoothGattDescriptor,
      preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray,
    ) {
      handler.post {
        if (responseNeeded) server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, 0, null)
        if (descriptor.uuid == CCCD_UUID && value.contentEquals(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)) {
          responder(device)?.let(::pump)
        }
      }
    }

    override fun onCharacteristicWriteRequest(
      device: BluetoothDevice, requestId: Int, characteristic: BluetoothGattCharacteristic,
      preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray,
    ) {
      handler.post {
        if (responseNeeded) server?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, 0, null)
        if (characteristic.uuid == LINK_UUID) responder(device)?.let { receive(it, value) }
      }
    }

    override fun onNotificationSent(device: BluetoothDevice, status: Int) {
      handler.post { asPeripheral[device.address]?.let(::sent) }
    }
  }
}
