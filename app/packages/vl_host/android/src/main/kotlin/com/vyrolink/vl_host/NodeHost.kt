package com.vyrolink.vl_host

import android.content.Context
import android.content.pm.ApplicationInfo
import java.io.File
import uniffi.vl_ffi.VlException
import uniffi.vl_ffi.VlNode
import uniffi.vl_ffi.maxTextBytes
import uniffi.vl_ffi.qrMatrix
import uniffi.vl_ffi.reactionSet
import uniffi.vl_ffi.removeDatabaseFiles
import uniffi.vl_ffi.Contact as CoreContact
import uniffi.vl_ffi.DeliveryState as CoreDeliveryState
import uniffi.vl_ffi.Group as CoreGroup
import uniffi.vl_ffi.GroupMessage as CoreGroupMessage
import uniffi.vl_ffi.Message as CoreMessage

private fun now(): ULong = (System.currentTimeMillis() / 1000).toULong()

private fun Long.toU32(): UInt = coerceIn(0L, UInt.MAX_VALUE.toLong()).toUInt()

/**
 * Owns the Rust node for this process and implements the typed host API.
 * Pigeon runs handlers on one background task queue; Bluetooth runs on its own
 * thread. VlNode is internally locked, so both may call it.
 */
internal class NodeHost(
  private val context: Context,
  private val permissions: Permissions,
  private val ui: SystemUi,
) : VlHostApi {
  companion object {
    const val RELAY_SETTING = "relay.enabled"
    const val MODE_SETTING = "relay.mode"
    const val CARRY_CAP_SETTING = "relay.carry_cap_mb"

    fun map(e: Throwable): FlutterError = when (e) {
      is FlutterError -> e
      is VlException.InvalidInput -> FlutterError("InvalidInput", e.reason, null)
      is VlException.Storage -> FlutterError("Storage", e.reason, null)
      is VlException.Crypto -> FlutterError("Crypto", e.reason, null)
      is VlException -> FlutterError(e::class.simpleName ?: "Host", null, null)
      is KeyStore.Failure -> FlutterError("KeyStore", e.message, null)
      else -> FlutterError("Host", e.toString(), null)
    }
  }

  private val keys = KeyStore(context)
  private val backup = IdentityBackup(context)
  private val dir = File(context.noBackupFilesDir, "vyrolink").apply { mkdirs() }
  private val relayDb = File(dir, "relay.db")
  private val conversationDb = File(dir, "conversation.db")
  private val debuggable = (context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0

  @Volatile private var current: VlNode? = null
  var relay: BleRelay? = null
  private var peer: LoopbackPeer? = null

  val node: VlNode? get() = current

  /** Opens the node when both keys exist (process started for Bluetooth work). */
  fun openIfProvisioned() {
    if (keys.exists(KeyStore.Slot.RELAY) && keys.exists(KeyStore.Slot.CONVERSATION)) runCatching { open() }
  }

  @Synchronized
  private fun open(): VlNode {
    current?.let { return it }
    val n = VlNode.open(
      relayDb.path, keys.key(KeyStore.Slot.RELAY),
      conversationDb.path, keys.key(KeyStore.Slot.CONVERSATION),
    )
    runCatching { n.maintain(now()) }
    n.setting(CARRY_CAP_SETTING)?.toULongOrNull()?.let { runCatching { n.setCarryCap(it shl 20) } }
    current = n
    return n
  }

  private fun live(): VlNode = current ?: throw FlutterError("NotStarted", "call start() first", null)

  private inline fun <T> run(body: (VlNode) -> T): T = try {
    body(live())
  } catch (e: Throwable) {
    throw map(e)
  }

  // Lifecycle and identity

  override fun start(): StartupInfo = try {
    val n = open()
    if (n.hasIdentity()) runCatching { backup.save(n.identityBackup()) }
    if (n.hasIdentity() && n.setting(RELAY_SETTING) == "1") {
      relay?.setMode(savedMode())
      relay?.setEnabled(true)
      applyRelayService(true)
    }
    StartupInfo(n.hasIdentity(), maxTextBytes().toLong(), reactionSet())
  } catch (e: Throwable) {
    throw map(e)
  }

  override fun createIdentity(): Boolean = run { n ->
    val saved = backup.load()
    if (saved != null && runCatching { n.restoreIdentity(saved, now()) }.isSuccess) return@run true
    n.createIdentity(now())
    backup.save(n.identityBackup())
    false
  }
  override fun myFingerprint(): ByteArray = run { it.myFingerprint() }
  override fun internetOutbox(): List<ByteArray> = run { it.internetOutbox(now()) }
  override fun internetUploaded(envelopes: List<ByteArray>) = run { it.internetUploaded(envelopes, now()) }
  override fun internetTags(): List<ByteArray> = run { it.internetTags() }
  override fun receiveInternet(envelope: ByteArray): Boolean = run { it.receiveInternet(envelope, now()) }
  override fun setting(key: String): String? = run { it.setting(key) }
  override fun setSetting(key: String, value: String?) = run { it.setSetting(key, value) }
  override fun maintain() = run { it.maintain(now()) }
  override fun myIdCode(): String = run { it.myIdCode() }

  /** Relay mode saved by the user (1 = while charging, 2 = always). */
  fun savedMode(): Int = runCatching { current?.setting(MODE_SETTING)?.toInt() }.getOrNull() ?: 2

  /**
   * Starts the foreground service only for "always" mode while relaying is on,
   * Bluetooth permission is granted (Android requires it for the
   * `connectedDevice` type) and the app is in front (Android 12+ forbids
   * starting one from the background).
   */
  fun applyRelayService(enabled: Boolean) {
    val allowed = enabled && savedMode() == 2 && permissions.hasBluetooth() && VlHostPlugin.inForeground
    if (allowed) RelayService.start(context) else if (!enabled || savedMode() != 2) RelayService.stop(context)
  }

  override fun renameContact(contactId: ByteArray, name: String) = run { it.renameContact(contactId, name) }
  override fun deleteContact(contactId: ByteArray) = run { it.deleteContact(contactId) }

  override fun resend(messageId: ByteArray) {
    run { it.resend(messageId, now()) }
    relay?.outboxChanged()
  }

  override fun sendAttachment(
    contactId: ByteArray, kind: Long, name: String, data: ByteArray, caption: String, replyTo: ByteArray?,
  ): ByteArray = run {
    it.sendAttachment(contactId, kind.coerceIn(0, 255).toUByte(), name, data, caption, replyTo, now())
  }.also { relay?.outboxChanged() }

  override fun attachmentData(messageId: ByteArray): ByteArray? = run { it.attachmentData(messageId) }
  override suspend fun pickImage(): PickedFile? = ui.pickImage()
  override suspend fun pickFile(): PickedFile? = ui.pickFile()
  override suspend fun shareFile(name: String, data: ByteArray) = ui.share(name, data)

  override fun setCarryCapMb(mb: Long) {
    val clamped = mb.coerceIn(1, 100)
    run {
      it.setSetting(CARRY_CAP_SETTING, clamped.toString())
      it.setCarryCap(clamped.toULong() shl 20)
    }
  }

  override fun carryCapMb(): Long = run { (it.carryCap() shr 20).toLong() }

  override fun setRelayMode(mode: Long) {
    val m = if (mode == 1L) 1 else 2
    run { it.setSetting(MODE_SETTING, m.toString()) }
    relay?.setMode(m)
    val on = runCatching { current?.setting(RELAY_SETTING) == "1" }.getOrDefault(false)
    applyRelayService(on)
  }

  override fun canAuthenticate(): Boolean = ui.canAuthenticate()
  override suspend fun authenticate(reason: String): Boolean = ui.authenticate(reason)

  override fun setScreenCaptureBlocked(blocked: Boolean) {
    ui.blockScreenCapture = blocked
  }

  override fun panicWipe() {
    relay?.shutdown()
    RelayService.stop(context)
    peer?.destroy()
    peer = null
    Notifier.cancelAll(context)
    try {
      // 0. The reinstall copy of the identity goes first (ADR-038).
      val backupError = runCatching { backup.destroy() }.exceptionOrNull()
      // 1. Destroy the keys (cryptographic erasure); stop here if that fails.
      keys.destroyAll()
      // 2. Then remove the files and refuse further use of the old node.
      val old = synchronized(this) { current.also { current = null } }
      if (old != null) {
        old.panicWipe()
        old.destroy()
      } else {
        removeDatabaseFiles(relayDb.path, conversationDb.path)
      }
      backupError?.let { throw FlutterError("KeyStore", it.message, null) }
    } catch (e: FlutterError) {
      throw e
    } catch (e: Throwable) {
      throw map(e)
    }
  }

  // Pairing and contacts

  override fun pairingCode(displayName: String): String = run { it.pairingCode(displayName, now()) }

  override fun qrCode(text: String): QrCode = try {
    val m = qrMatrix(text)
    QrCode(m.size.toLong(), m.modules)
  } catch (e: Throwable) {
    throw map(e)
  }

  override fun completePairing(code: String): Contact = run { it.completePairingCode(code, now()).toPigeon() }
  override fun contacts(): List<Contact> = run { n -> n.contacts().map { it.toPigeon() } }
  override fun setVerified(contactId: ByteArray, verified: Boolean) = run { it.setVerified(contactId, verified) }
  override fun setBlocked(contactId: ByteArray, blocked: Boolean) = run { it.setBlocked(contactId, blocked, now()) }

  // Conversations

  override fun conversationSummaries(): List<ConversationSummary> = run { n ->
    n.conversationSummaries().map { ConversationSummary(it.contact.toPigeon(), it.last?.toPigeon(), it.unread.toLong()) }
  }

  override fun conversation(contactId: ByteArray, limit: Long): List<Message> =
    run { n -> n.conversation(contactId, limit.toU32()).map { it.toPigeon() } }

  override fun message(messageId: ByteArray): Message? = run { it.message(messageId)?.toPigeon() }

  override fun sendMessage(contactId: ByteArray, text: String, replyTo: ByteArray?, forwarded: Boolean): ByteArray =
    run { it.sendMessage(contactId, text, replyTo, forwarded, now()) }.also { relay?.outboxChanged() }

  override fun react(contactId: ByteArray, messageId: ByteArray, reaction: Long) {
    run { it.react(contactId, messageId, reaction.coerceIn(0, 255).toUByte(), now()) }
    relay?.outboxChanged()
  }

  override fun reactions(messageId: ByteArray): List<Reaction> =
    run { n -> n.reactions(messageId).map { Reaction(it.fromMe, it.reaction.toLong()) } }

  override fun editMessage(contactId: ByteArray, messageId: ByteArray, text: String) {
    run { it.editMessage(contactId, messageId, text, now()) }
    relay?.outboxChanged()
  }

  override fun deleteForEveryone(contactId: ByteArray, messageId: ByteArray) {
    run { it.deleteForEveryone(contactId, messageId, now()) }
    relay?.outboxChanged()
  }

  override fun deleteForMe(messageId: ByteArray) = run { it.deleteForMe(messageId, now()) }

  override fun setDisappearingTimer(contactId: ByteArray, timerSecs: Long) {
    run { it.setDisappearingTimer(contactId, timerSecs.toU32(), now()) }
    relay?.outboxChanged()
  }

  override fun disappearingTimer(contactId: ByteArray): Long = run { it.disappearingTimer(contactId).toLong() }
  override fun markRead(contactId: ByteArray): Long = run { it.markRead(contactId, now()).toLong() }

  override fun search(query: String, contactId: ByteArray?, limit: Long): List<Message> =
    run { n -> n.search(query, contactId, limit.toU32()).map { it.toPigeon() } }

  override fun saveDraft(contactId: ByteArray, text: String) = run { it.saveDraft(contactId, text, now()) }
  override fun draft(contactId: ByteArray): String? = run { it.draft(contactId) }
  override fun setPinned(messageId: ByteArray, pinned: Boolean) { run { it.setPinned(messageId, pinned) } }
  override fun setStarred(messageId: ByteArray, starred: Boolean) { run { it.setStarred(messageId, starred) } }

  override fun flagged(contactId: ByteArray?, pinned: Boolean): List<Message> =
    run { n -> n.flagged(contactId, pinned).map { it.toPigeon() } }

  override fun pendingOutgoing(): List<Message> = run { n -> n.pendingOutgoing().map { it.toPigeon() } }

  override fun messageEvents(messageId: ByteArray): List<MessageEvent> =
    run { n -> n.messageEvents(messageId).map { MessageEvent(it.atSecs, it.state.toPigeon()) } }

  // Groups

  override fun createGroup(name: String, myName: String, contactIds: List<ByteArray>): ByteArray =
    run { it.createGroup(name, myName, contactIds, now()) }.also { relay?.outboxChanged() }

  override fun setGroupMembers(groupId: ByteArray, myName: String, contactIds: List<ByteArray>) {
    run { it.setGroupMembers(groupId, myName, contactIds, now()) }
    relay?.outboxChanged()
  }

  override fun leaveGroup(groupId: ByteArray) {
    run { it.leaveGroup(groupId, now()) }
    relay?.outboxChanged()
  }

  override fun groups(): List<Group> = run { n -> n.groups().map { it.toPigeon() } }

  override fun groupMessages(groupId: ByteArray, limit: Long): List<GroupMessage> =
    run { n -> n.groupMessages(groupId, limit.toU32()).map { it.toPigeon() } }

  override fun sendGroupMessage(groupId: ByteArray, text: String, replyTo: ByteArray?): ByteArray =
    run { it.sendGroupMessage(groupId, text, replyTo, now()) }.also { relay?.outboxChanged() }

  override fun groupDelivery(groupMsgId: ByteArray): GroupDelivery =
    run { n -> n.groupDelivery(groupMsgId).let { GroupDelivery(it.delivered.toLong(), it.sent.toLong()) } }

  // Bluetooth and permissions

  override suspend fun currentLocation(): LocationInfo? {
    if (!permissions.hasLocation() && !permissions.requestLocation()) return null
    return Location.currentFix(context)
  }

  override fun sendLocation(contactId: ByteArray, fix: LocationInfo): ByteArray = run {
    it.sendLocation(
      contactId, fix.latE7.toInt(), fix.lonE7.toInt(), fix.accuracyM.coerceIn(0, 65_535).toInt().toUShort(),
      fix.capturedMs.coerceAtLeast(0).toULong(), now(),
    )
  }

  override fun openMap(latE7: Long, lonE7: Long, label: String) = Location.openMap(context, latE7, lonE7, label)

  override fun sendPoll(contactId: ByteArray, question: String, options: List<String>, multi: Boolean): ByteArray =
    run { it.sendPoll(contactId, question, options, multi, now()) }

  override fun sendGroupPoll(groupId: ByteArray, question: String, options: List<String>, multi: Boolean): ByteArray =
    run { it.sendGroupPoll(groupId, question, options, multi, now()) }

  override fun vote(pollId: ByteArray, choices: Long) =
    run { it.vote(pollId, choices.coerceIn(0, 65_535).toInt().toUShort(), now()) }

  override fun introduce(contactA: ByteArray, contactB: ByteArray): ByteArray =
    run { it.introduce(contactA, contactB, now()) }

  override fun acceptIntro(introId: ByteArray, myName: String) = run { it.acceptIntro(introId, myName, now()) }

  override fun declineIntro(introId: ByteArray) = run { it.declineIntro(introId) }

  override fun paperMessage(messageId: ByteArray): String = run { it.paperMessage(messageId) }

  override fun receivePaper(code: String): ByteArray = run { it.receivePaper(code, now()) }

  override fun resyncSecs(): Long = current?.resyncSecs()?.toLong() ?: 5L

  override fun setResyncSecs(secs: Long): Long =
    run { it.setResyncSecs(secs.coerceIn(1, 45).toUInt()).toLong() }

  override fun nearbyContacts(withinSecs: Long): List<NearbyContact> = run { n ->
    n.nearbyContacts(now(), withinSecs.coerceAtLeast(0).toULong()).map {
      NearbyContact(it.contactId, it.seenSecs.toLong())
    }
  }

  override fun networkStatus(): NetworkStatus =
    relay?.status() ?: throw FlutterError("Bluetooth", "relay not set up", null)

  override fun setRelayEnabled(enabled: Boolean) {
    run { it.setSetting(RELAY_SETTING, if (enabled) "1" else "0") }
    if (enabled) {
      permissions.requestBluetooth {
        relay?.setMode(savedMode())
        relay?.setEnabled(true)
        applyRelayService(true)
      }
    } else {
      relay?.setEnabled(false)
      applyRelayService(false)
    }
  }

  override fun cameraPermission(): PermissionState = permissions.camera()
  override suspend fun requestCameraPermission(): PermissionState = permissions.requestCamera()
  override suspend fun requestNotificationPermission(): PermissionState = permissions.requestNotifications()
  override fun openAppSettings() = permissions.openAppSettings()

  // Debug loopback peer

  override fun debugCreatePeer(name: String): Contact {
    if (!debuggable) throw FlutterError("NotAvailable", "debug builds only", null)
    return try {
      val me = live()
      peer?.destroy()
      val myName = me.setting("profile.name") ?: "Me"
      val (p, contact) = LoopbackPeer.create(context, name, me, myName)
      peer = p
      contact.toPigeon()
    } catch (e: Throwable) {
      throw map(e)
    }
  }

  override fun debugPeerSend(text: String) {
    if (!debuggable) throw FlutterError("NotAvailable", "debug builds only", null)
    val p = peer ?: throw FlutterError("NotAvailable", "create a peer first", null)
    try { p.send(text) } catch (e: Throwable) { throw map(e) }
  }

  override fun debugPeerSendFile(name: String, data: ByteArray) {
    if (!debuggable) throw FlutterError("NotAvailable", "debug builds only", null)
    val p = peer ?: throw FlutterError("NotAvailable", "create a peer first", null)
    try { p.sendFile(name, data) } catch (e: Throwable) { throw map(e) }
  }

  override fun debugExchange() {
    if (!debuggable) throw FlutterError("NotAvailable", "debug builds only", null)
    val p = peer ?: throw FlutterError("NotAvailable", "create a peer first", null)
    try { p.exchange(live()) } catch (e: Throwable) { throw map(e) }
    relay?.notifyDataChanged()
  }
}

// Core → Pigeon conversions

internal fun CoreDeliveryState.toPigeon(): DeliveryState = when (this) {
  CoreDeliveryState.QUEUED -> DeliveryState.QUEUED
  CoreDeliveryState.SENT_TO_NEARBY -> DeliveryState.SENT_TO_NEARBY
  CoreDeliveryState.ACCEPTED_BY_RELAY -> DeliveryState.ACCEPTED_BY_RELAY
  CoreDeliveryState.DELIVERED -> DeliveryState.DELIVERED
  CoreDeliveryState.READ -> DeliveryState.READ
  CoreDeliveryState.EXPIRED -> DeliveryState.EXPIRED
  CoreDeliveryState.DELIVERY_UNKNOWN -> DeliveryState.DELIVERY_UNKNOWN
  CoreDeliveryState.FAILED -> DeliveryState.FAILED
  CoreDeliveryState.RECEIVED -> DeliveryState.RECEIVED
}

internal fun CoreContact.toPigeon() =
  Contact(contactId, displayName, fingerprint, idCode, verified, blocked, safetyNumber, lastSeenSecs, introducedBy)

internal fun CoreMessage.toPigeon() = Message(
  messageId, contactId, outgoing, body, createdAtSecs, state.toPigeon(), replyTo, forwarded, deleted, edited,
  disappearsAtSecs, pinned, starred,
  attachment?.let {
    AttachmentInfo(
      it.kind.toLong(), it.name, it.totalLen.toLong(), it.chunkCount.toLong(), it.receivedChunks.toLong(),
      it.complete, it.deliveredChunks.toLong(),
    )
  },
  location?.let { LocationInfo(it.latE7.toLong(), it.lonE7.toLong(), it.accuracyM.toLong(), it.capturedMs.toLong()) },
  poll?.toPigeon(),
  intro?.let { IntroInfo(it.introId, it.otherName, it.state.toLong()) },
)

internal fun CoreGroup.toPigeon() = Group(
  groupId, name, epoch.toLong(), creatorFingerprint, left, members.map { GroupMember(it.fingerprint, it.name) },
)

internal fun CoreGroupMessage.toPigeon() =
  GroupMessage(groupMsgId, authorFingerprint, outgoing, body, replyTo, createdAtSecs, poll?.toPigeon())

internal fun uniffi.vl_ffi.PollInfo.toPigeon() =
  PollInfo(question, options, multi, counts.map { it.toLong() }, mine.toLong(), voters.toLong())
