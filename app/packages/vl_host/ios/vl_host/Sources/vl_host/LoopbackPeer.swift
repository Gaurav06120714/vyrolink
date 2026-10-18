#if DEBUG
  import Foundation
  import VlCore

  /// Debug builds only. A second, complete node in a temporary directory that
  /// pairs with the app's node and exchanges over an in-memory link: the same
  /// Rust code path as Bluetooth minus the radio. Lets the simulator (which has
  /// no Bluetooth) exercise sending, receipts and incoming messages for real.
  final class LoopbackPeer {
    private let node: VlNode
    private let dir: URL
    private let contactOfMe: Data

    private init(node: VlNode, dir: URL, contactOfMe: Data) {
      self.node = node
      self.dir = dir
      self.contactOfMe = contactOfMe
    }

    private static var now: UInt64 { UInt64(Date().timeIntervalSince1970) }

    static func create(name: String, pairWith me: VlNode, myName: String) throws -> (LoopbackPeer, VlCore.Contact) {
      let dir = FileManager.default.temporaryDirectory.appendingPathComponent("vl-loopback-\(UUID().uuidString)")
      try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
      let random = { Data((0..<32).map { _ in UInt8.random(in: 0...255) }) }
      let node = try VlNode.open(
        relayDbPath: dir.appendingPathComponent("relay.db").path, relayKey: random(),
        conversationDbPath: dir.appendingPathComponent("conversation.db").path, conversationKey: random())
      try node.createIdentity(nowSecs: now)
      let mine = try me.pairingCode(displayName: myName, nowSecs: now)
      let theirs = try node.pairingCode(displayName: name, nowSecs: now)
      let contact = try me.completePairingCode(code: theirs, nowSecs: now)
      let meInPeer = try node.completePairingCode(code: mine, nowSecs: now)
      return (LoopbackPeer(node: node, dir: dir, contactOfMe: meInPeer.contactId), contact)
    }

    func send(_ text: String) throws {
      _ = try node.sendText(contactId: contactOfMe, text: text, nowSecs: Self.now)
    }

    func sendFile(_ name: String, _ data: Data) throws {
      _ = try node.sendAttachment(
        contactId: contactOfMe, kind: 2, name: name, data: data, caption: "", replyTo: nil, nowSecs: Self.now)
    }

    /// One encounter, 185-byte writes, until both sides go quiet.
    func exchange(with me: VlNode) throws {
      let a = try me.linkOpen(initiator: true)
      let b = try node.linkOpen(initiator: false)
      defer {
        me.linkClose(linkId: a.linkId)
        node.linkClose(linkId: b.linkId)
      }
      var toPeer = a.bytesToSend
      var toMe = Data()
      for _ in 0..<200 where !(toPeer.isEmpty && toMe.isEmpty) {
        var nextToMe = Data()
        for chunk in toPeer.chunked(185) {
          nextToMe.append(try node.linkReceive(linkId: b.linkId, bytes: chunk, nowSecs: Self.now))
        }
        var nextToPeer = Data()
        for chunk in toMe.chunked(185) {
          nextToPeer.append(try me.linkReceive(linkId: a.linkId, bytes: chunk, nowSecs: Self.now))
        }
        toMe = nextToMe
        toPeer = nextToPeer
      }
      // The peer reads what it received, as a person would.
      _ = try node.markRead(contactId: contactOfMe, nowSecs: Self.now)
    }

    func destroy() {
      try? node.panicWipe()
      try? FileManager.default.removeItem(at: dir)
    }
  }

  extension Data {
    fileprivate func chunked(_ size: Int) -> [Data] {
      stride(from: 0, to: count, by: size).map { Data(self[($0 + startIndex)..<Swift.min($0 + size, count) + startIndex]) }
    }
  }
#endif
