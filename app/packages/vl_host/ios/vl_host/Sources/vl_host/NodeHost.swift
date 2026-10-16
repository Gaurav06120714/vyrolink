import Flutter
import Foundation
import UIKit
import UserNotifications
import VlCore

private func now() -> UInt64 { UInt64(Date().timeIntervalSince1970) }
private func bytes(_ d: Data) -> FlutterStandardTypedData { FlutterStandardTypedData(bytes: d) }

/// Owns the Rust node for this process and implements the typed host API.
/// Pigeon runs these handlers on one background serial queue; Bluetooth runs on
/// its own queue. `VlNode` is internally locked, so both may call it.
final class NodeHost: VlHostApi {
  static let relaySetting = "relay.enabled"
  static let carryCapSetting = "relay.carry_cap_mb"

  private let lock = NSLock()
  private var current: VlNode?
  private let relayDb: URL
  private let conversationDb: URL
  var relay: BleRelay?
  #if DEBUG
    private var peer: LoopbackPeer?
  #endif

  init() {
    let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    var dir = base.appendingPathComponent("VyroLink", isDirectory: true)
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    var values = URLResourceValues()
    values.isExcludedFromBackup = true
    try? dir.setResourceValues(values)
    relayDb = dir.appendingPathComponent("relay.db")
    conversationDb = dir.appendingPathComponent("conversation.db")
  }

  var node: VlNode? { lock.withLock { current } }

  /// Opens the node if both keys already exist (used when iOS relaunches the
  /// app in the background for Bluetooth, before the UI asks for anything).
  func openIfProvisioned() {
    guard KeyStore.exists(.relay), KeyStore.exists(.conversation) else { return }
    _ = try? open()
  }

  @discardableResult
  private func open() throws -> VlNode {
    try lock.withLock {
      if let n = current { return n }
      let n = try VlNode.open(
        relayDbPath: relayDb.path, relayKey: KeyStore.key(.relay),
        conversationDbPath: conversationDb.path, conversationKey: KeyStore.key(.conversation))
      try? n.maintain(nowSecs: now())
      if let mb = (try? n.setting(key: Self.carryCapSetting)).flatMap({ $0 }).flatMap(UInt64.init) {
        try? n.setCarryCap(bytes: mb << 20)
      }
      current = n
      return n
    }
  }

  private func live() throws -> VlNode {
    guard let n = node else { throw PigeonError(code: "NotStarted", message: "call start() first", details: nil) }
    return n
  }

  private func run<T>(_ body: (VlNode) throws -> T) throws -> T {
    do { return try body(try live()) } catch { throw Self.map(error) }
  }

  static func map(_ error: Error) -> Error {
    switch error {
    case let e as PigeonError: return e
    case let e as VlError:
      switch e {
      case .InvalidInput(let r): return PigeonError(code: "InvalidInput", message: r, details: nil)
      case .Storage(let r): return PigeonError(code: "Storage", message: r, details: nil)
      case .Crypto(let r): return PigeonError(code: "Crypto", message: r, details: nil)
      default: return PigeonError(code: "\(e)", message: nil, details: nil)
      }
    case let e as KeyStore.Failure:
      return PigeonError(code: "KeyStore", message: "Keychain \(e.action) failed (\(e.status))", details: nil)
    default: return PigeonError(code: "Host", message: "\(error)", details: nil)
    }
  }

  // MARK: Lifecycle and identity

  func start() throws -> StartupInfo {
    do {
      let n = try open()
      if n.hasIdentity(), (try? n.setting(key: Self.relaySetting)).flatMap({ $0 }) == "1" {
        relay?.setEnabled(true)
      }
      return StartupInfo(
        hasIdentity: n.hasIdentity(), maxTextBytes: Int64(maxTextBytes()), reactions: reactionSet())
    } catch { throw Self.map(error) }
  }

  func createIdentity() throws { try run { try $0.createIdentity(nowSecs: now()) } }
  func myFingerprint() throws -> FlutterStandardTypedData { try run { bytes(try $0.myFingerprint()) } }
  func setting(key: String) throws -> String? { try run { try $0.setting(key: key) } }
  func setSetting(key: String, value: String?) throws { try run { try $0.setSetting(key: key, value: value) } }
  func maintain() throws { try run { try $0.maintain(nowSecs: now()) } }
  func myIdCode() throws -> String { try run { try $0.myIdCode() } }

  func renameContact(contactId: FlutterStandardTypedData, name: String) throws {
    try run { try $0.renameContact(contactId: contactId.data, name: name) }
  }

  func deleteContact(contactId: FlutterStandardTypedData) throws {
    try run { try $0.deleteContact(contactId: contactId.data) }
  }

  func resend(messageId: FlutterStandardTypedData) throws {
    try run { try $0.resend(messageId: messageId.data, nowSecs: now()) }
    relay?.outboxChanged()
  }

  func sendAttachment(
    contactId: FlutterStandardTypedData, kind: Int64, name: String, data: FlutterStandardTypedData,
    caption: String, replyTo: FlutterStandardTypedData?
  ) throws -> FlutterStandardTypedData {
    let id = try run {
      try $0.sendAttachment(
        contactId: contactId.data, kind: UInt8(clamping: kind), name: name, data: data.data, caption: caption,
        replyTo: replyTo?.data, nowSecs: now())
    }
    relay?.outboxChanged()
    return bytes(id)
  }

  func attachmentData(messageId: FlutterStandardTypedData) throws -> FlutterStandardTypedData? {
    try run { try $0.attachmentData(messageId: messageId.data).map(bytes) }
  }

  func pickImage() async throws -> PickedFile? { await Pickers.pickImage() }
  func pickFile() async throws -> PickedFile? { await Pickers.pickFile() }
  func shareFile(name: String, data: FlutterStandardTypedData) async throws {
    await Pickers.share(name: name, data: data.data)
  }

  func setCarryCapMb(mb: Int64) throws {
    let clamped = min(max(mb, 1), 100)
    try run {
      try $0.setSetting(key: Self.carryCapSetting, value: String(clamped))
      try $0.setCarryCap(bytes: UInt64(clamped) << 20)
    }
  }

  func carryCapMb() throws -> Int64 { try run { Int64(try $0.carryCap() >> 20) } }

  /// iOS has a single "on" mode: the system decides how much runs in the background.
  func setRelayMode(mode: Int64) throws {}

  func canAuthenticate() throws -> Bool { Pickers.canAuthenticate() }
  func authenticate(reason: String) async throws -> Bool { await Pickers.authenticate(reason: reason) }

  /// iOS does not let apps block screenshots; the setting exists only on Android.
  func setScreenCaptureBlocked(blocked: Bool) throws {}

  func panicWipe() throws {
    relay?.shutdown()
    #if DEBUG
      peer?.destroy()
      peer = nil
    #endif
    UNUserNotificationCenter.current().removeAllDeliveredNotifications()
    do {
      // 1. Destroy the keys (cryptographic erasure); stop here if that fails.
      try KeyStore.destroyAll()
      // 2. Then remove the files and refuse further use of the old node.
      let old = lock.withLock { () -> VlNode? in
        defer { current = nil }
        return current
      }
      if let old {
        try old.panicWipe()
      } else {
        try removeDatabaseFiles(relayDbPath: relayDb.path, conversationDbPath: conversationDb.path)
      }
    } catch { throw Self.map(error) }
  }

  // MARK: Pairing and contacts

  func pairingCode(displayName: String) throws -> String {
    try run { try $0.pairingCode(displayName: displayName, nowSecs: now()) }
  }

  func qrCode(text: String) throws -> QrCode {
    do {
      let m = try qrMatrix(text: text)
      return QrCode(size: Int64(m.size), modules: bytes(m.modules))
    } catch { throw Self.map(error) }
  }

  func completePairing(code: String) throws -> Contact {
    try run { Contact(try $0.completePairingCode(code: code, nowSecs: now())) }
  }

  func contacts() throws -> [Contact] { try run { try $0.contacts().map(Contact.init) } }

  func setVerified(contactId: FlutterStandardTypedData, verified: Bool) throws {
    try run { try $0.setVerified(contactId: contactId.data, verified: verified) }
  }

  func setBlocked(contactId: FlutterStandardTypedData, blocked: Bool) throws {
    try run { try $0.setBlocked(contactId: contactId.data, blocked: blocked, nowSecs: now()) }
  }

  // MARK: Conversations

  func conversationSummaries() throws -> [ConversationSummary] {
    try run {
      try $0.conversationSummaries().map {
        ConversationSummary(contact: Contact($0.contact), last: $0.last.map(Message.init), unread: Int64($0.unread))
      }
    }
  }

  func conversation(contactId: FlutterStandardTypedData, limit: Int64) throws -> [Message] {
    try run { try $0.conversation(contactId: contactId.data, limit: UInt32(clamping: limit)).map(Message.init) }
  }

  func message(messageId: FlutterStandardTypedData) throws -> Message? {
    try run { try $0.message(messageId: messageId.data).map(Message.init) }
  }

  func sendMessage(
    contactId: FlutterStandardTypedData, text: String, replyTo: FlutterStandardTypedData?, forwarded: Bool
  ) throws -> FlutterStandardTypedData {
    let id = try run {
      try $0.sendMessage(
        contactId: contactId.data, text: text, replyTo: replyTo?.data, forwarded: forwarded, nowSecs: now())
    }
    relay?.outboxChanged()
    return bytes(id)
  }

  func react(contactId: FlutterStandardTypedData, messageId: FlutterStandardTypedData, reaction: Int64) throws {
    try run {
      try $0.react(contactId: contactId.data, messageId: messageId.data, reaction: UInt8(clamping: reaction), nowSecs: now())
    }
    relay?.outboxChanged()
  }

  func reactions(messageId: FlutterStandardTypedData) throws -> [Reaction] {
    try run { try $0.reactions(messageId: messageId.data).map { Reaction(fromMe: $0.fromMe, reaction: Int64($0.reaction)) } }
  }

  func editMessage(contactId: FlutterStandardTypedData, messageId: FlutterStandardTypedData, text: String) throws {
    try run { try $0.editMessage(contactId: contactId.data, messageId: messageId.data, text: text, nowSecs: now()) }
    relay?.outboxChanged()
  }

  func deleteForEveryone(contactId: FlutterStandardTypedData, messageId: FlutterStandardTypedData) throws {
    try run { try $0.deleteForEveryone(contactId: contactId.data, messageId: messageId.data, nowSecs: now()) }
    relay?.outboxChanged()
  }

  func deleteForMe(messageId: FlutterStandardTypedData) throws {
    try run { try $0.deleteForMe(messageId: messageId.data, nowSecs: now()) }
  }

  func setDisappearingTimer(contactId: FlutterStandardTypedData, timerSecs: Int64) throws {
    try run {
      try $0.setDisappearingTimer(contactId: contactId.data, timerSecs: UInt32(clamping: timerSecs), nowSecs: now())
    }
    relay?.outboxChanged()
  }

  func disappearingTimer(contactId: FlutterStandardTypedData) throws -> Int64 {
    try run { Int64(try $0.disappearingTimer(contactId: contactId.data)) }
  }

  func markRead(contactId: FlutterStandardTypedData) throws -> Int64 {
    try run { Int64(try $0.markRead(contactId: contactId.data, nowSecs: now())) }
  }

  func search(query: String, contactId: FlutterStandardTypedData?, limit: Int64) throws -> [Message] {
    try run { try $0.search(query: query, contactId: contactId?.data, limit: UInt32(clamping: limit)).map(Message.init) }
  }

  func saveDraft(contactId: FlutterStandardTypedData, text: String) throws {
    try run { try $0.saveDraft(contactId: contactId.data, text: text, nowSecs: now()) }
  }

  func draft(contactId: FlutterStandardTypedData) throws -> String? { try run { try $0.draft(contactId: contactId.data) } }

  func setPinned(messageId: FlutterStandardTypedData, pinned: Bool) throws {
    try run { _ = try $0.setPinned(messageId: messageId.data, pinned: pinned) }
  }

  func setStarred(messageId: FlutterStandardTypedData, starred: Bool) throws {
    try run { _ = try $0.setStarred(messageId: messageId.data, starred: starred) }
  }

  func flagged(contactId: FlutterStandardTypedData?, pinned: Bool) throws -> [Message] {
    try run { try $0.flagged(contactId: contactId?.data, pinned: pinned).map(Message.init) }
  }

  func pendingOutgoing() throws -> [Message] { try run { try $0.pendingOutgoing().map(Message.init) } }

  func messageEvents(messageId: FlutterStandardTypedData) throws -> [MessageEvent] {
    try run {
      try $0.messageEvents(messageId: messageId.data).map { MessageEvent(atSecs: $0.atSecs, state: DeliveryState($0.state)) }
    }
  }

  // MARK: Groups

  func createGroup(name: String, myName: String, contactIds: [FlutterStandardTypedData]) throws -> FlutterStandardTypedData {
    let id = try run {
      try $0.createGroup(name: name, myName: myName, contactIds: contactIds.map(\.data), nowSecs: now())
    }
    relay?.outboxChanged()
    return bytes(id)
  }

  func setGroupMembers(groupId: FlutterStandardTypedData, myName: String, contactIds: [FlutterStandardTypedData]) throws {
    try run {
      try $0.setGroupMembers(groupId: groupId.data, myName: myName, contactIds: contactIds.map(\.data), nowSecs: now())
    }
    relay?.outboxChanged()
  }

  func leaveGroup(groupId: FlutterStandardTypedData) throws {
    try run { try $0.leaveGroup(groupId: groupId.data, nowSecs: now()) }
    relay?.outboxChanged()
  }

  func groups() throws -> [Group] { try run { try $0.groups().map(Group.init) } }

  func groupMessages(groupId: FlutterStandardTypedData, limit: Int64) throws -> [GroupMessage] {
    try run { try $0.groupMessages(groupId: groupId.data, limit: UInt32(clamping: limit)).map(GroupMessage.init) }
  }

  func sendGroupMessage(groupId: FlutterStandardTypedData, text: String, replyTo: FlutterStandardTypedData?) throws
    -> FlutterStandardTypedData
  {
    let id = try run {
      try $0.sendGroupMessage(groupId: groupId.data, text: text, replyTo: replyTo?.data, nowSecs: now())
    }
    relay?.outboxChanged()
    return bytes(id)
  }

  func groupDelivery(groupMsgId: FlutterStandardTypedData) throws -> GroupDelivery {
    try run {
      let d = try $0.groupDelivery(groupMsgId: groupMsgId.data)
      return GroupDelivery(delivered: Int64(d.delivered), sent: Int64(d.sent))
    }
  }

  // MARK: Bluetooth, permissions

  func networkStatus() throws -> NetworkStatus {
    guard let relay else { throw PigeonError(code: "Bluetooth", message: "relay not set up", details: nil) }
    return relay.status()
  }

  func setRelayEnabled(enabled: Bool) throws {
    try run { try $0.setSetting(key: Self.relaySetting, value: enabled ? "1" : "0") }
    relay?.setEnabled(enabled)
  }

  func cameraPermission() throws -> PermissionState { QrScanner.permission() }

  func requestCameraPermission() async throws -> PermissionState { await QrScanner.requestPermission() }

  func requestNotificationPermission() async throws -> PermissionState {
    let center = UNUserNotificationCenter.current()
    let granted = (try? await center.requestAuthorization(options: [.alert, .sound, .badge])) ?? false
    return granted ? .granted : .denied
  }

  func openAppSettings() throws {
    DispatchQueue.main.async {
      if let url = URL(string: UIApplication.openSettingsURLString) { UIApplication.shared.open(url) }
    }
  }

  // MARK: Debug loopback peer

  func debugCreatePeer(name: String) throws -> Contact {
    #if DEBUG
      do {
        let me = try live()
        peer?.destroy()
        let myName = (try? me.setting(key: "profile.name")).flatMap { $0 } ?? "Me"
        let (p, contact) = try LoopbackPeer.create(name: name, pairWith: me, myName: myName)
        peer = p
        return Contact(contact)
      } catch { throw Self.map(error) }
    #else
      throw PigeonError(code: "NotAvailable", message: "debug builds only", details: nil)
    #endif
  }

  func debugPeerSend(text: String) throws {
    #if DEBUG
      guard let peer else { throw PigeonError(code: "NotAvailable", message: "create a peer first", details: nil) }
      do { try peer.send(text) } catch { throw Self.map(error) }
    #else
      throw PigeonError(code: "NotAvailable", message: "debug builds only", details: nil)
    #endif
  }

  func debugExchange() throws {
    #if DEBUG
      guard let peer else { throw PigeonError(code: "NotAvailable", message: "create a peer first", details: nil) }
      do { try peer.exchange(with: try live()) } catch { throw Self.map(error) }
      relay?.notifyDataChanged()
    #else
      throw PigeonError(code: "NotAvailable", message: "debug builds only", details: nil)
    #endif
  }
}

// MARK: Core → Pigeon conversions

extension DeliveryState {
  init(_ s: VlCore.DeliveryState) {
    switch s {
    case .queued: self = .queued
    case .sentToNearby: self = .sentToNearby
    case .acceptedByRelay: self = .acceptedByRelay
    case .delivered: self = .delivered
    case .read: self = .read
    case .expired: self = .expired
    case .deliveryUnknown: self = .deliveryUnknown
    case .failed: self = .failed
    case .received: self = .received
    }
  }
}

extension Contact {
  init(_ c: VlCore.Contact) {
    self.init(
      contactId: bytes(c.contactId), displayName: c.displayName, fingerprint: bytes(c.fingerprint),
      idCode: c.idCode, verified: c.verified, blocked: c.blocked,
      safetyNumber: c.safetyNumber)
  }
}

extension Message {
  init(_ m: VlCore.Message) {
    self.init(
      messageId: bytes(m.messageId), contactId: bytes(m.contactId), outgoing: m.outgoing, body: m.body, createdAtSecs: m.createdAtSecs,
      state: DeliveryState(m.state), replyTo: m.replyTo.map(bytes), forwarded: m.forwarded, deleted: m.deleted,
      edited: m.edited, disappearsAtSecs: m.disappearsAtSecs, pinned: m.pinned, starred: m.starred,
      attachment: m.attachment.map {
        AttachmentInfo(
          kind: Int64($0.kind), name: $0.name, totalLen: Int64($0.totalLen), chunkCount: Int64($0.chunkCount),
          receivedChunks: Int64($0.receivedChunks), complete: $0.complete, deliveredChunks: Int64($0.deliveredChunks))
      })
  }
}

extension Group {
  init(_ g: VlCore.Group) {
    self.init(
      groupId: bytes(g.groupId), name: g.name, epoch: Int64(g.epoch), creatorFingerprint: bytes(g.creatorFingerprint),
      left: g.left, members: g.members.map { GroupMember(fingerprint: bytes($0.fingerprint), name: $0.name) })
  }
}

extension GroupMessage {
  init(_ m: VlCore.GroupMessage) {
    self.init(
      groupMsgId: bytes(m.groupMsgId), authorFingerprint: bytes(m.authorFingerprint), outgoing: m.outgoing,
      body: m.body, replyTo: m.replyTo.map(bytes), createdAtSecs: m.createdAtSecs)
  }
}
