// Swift ⇄ Rust interop check for the UniFFI surface (run by tools/swift-interop-test.sh).
import Foundation

func check(_ cond: Bool, _ what: String) {
    if !cond { print("FAIL: \(what)"); exit(1) }
    print("ok: \(what)")
}

let base = FileManager.default.temporaryDirectory.appendingPathComponent("vl-swift-\(ProcessInfo.processInfo.processIdentifier)")
atexit { try? FileManager.default.removeItem(at: FileManager.default.temporaryDirectory.appendingPathComponent("vl-swift-\(ProcessInfo.processInfo.processIdentifier)")) }
try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
func key(_ b: UInt8) -> Data { Data(repeating: b, count: 32) }
func node(_ name: String) throws -> VlNode {
    try VlNode.open(relayDbPath: base.appendingPathComponent("\(name)-r.db").path, relayKey: key(1),
                    conversationDbPath: base.appendingPathComponent("\(name)-c.db").path, conversationKey: key(2))
}

let t0: UInt64 = 1_790_000_000
do {
    let alice = try node("alice"), charlie = try node("charlie")
    check(!alice.hasIdentity(), "fresh node has no identity")
    try alice.createIdentity(nowSecs: t0); try charlie.createIdentity(nowSecs: t0)
    let ca = try alice.pairingCard(displayName: "Alice", nowSecs: t0)
    let cc = try charlie.pairingCard(displayName: "Charlie", nowSecs: t0)
    let charlieForAlice = try alice.completePairing(theirCard: cc, nowSecs: t0)
    let aliceForCharlie = try charlie.completePairing(theirCard: ca, nowSecs: t0)
    check(charlieForAlice.safetyNumber == aliceForCharlie.safetyNumber, "safety numbers match across the boundary")
    check(charlieForAlice.displayName == "Charlie", "strings round-trip")

    let mid = try alice.sendText(contactId: charlieForAlice.contactId, text: "héllo from Swift ✓", nowSecs: t0)
    check(mid.count == 16, "message id is 16 bytes")

    func exchange(_ now: UInt64) throws {
        let a = try alice.linkOpen(initiator: true), c = try charlie.linkOpen(initiator: false)
        var toC = a.bytesToSend, toA = Data()
        for _ in 0..<100 where !(toC.isEmpty && toA.isEmpty) {
            var nextA = Data(), nextC = Data()
            if !toC.isEmpty { nextA.append(try charlie.linkReceive(linkId: c.linkId, bytes: toC, nowSecs: now)) }
            if !toA.isEmpty { nextC.append(try alice.linkReceive(linkId: a.linkId, bytes: toA, nowSecs: now)) }
            toA = nextA; toC = nextC
        }
        alice.linkClose(linkId: a.linkId); charlie.linkClose(linkId: c.linkId)
    }
    try exchange(t0 + 5)
    let inbox = try charlie.conversation(contactId: aliceForCharlie.contactId, limit: 10)
    check(inbox.first?.body == "héllo from Swift ✓", "UTF-8 text delivered through Rust")
    check(inbox.first?.state == .received, "recipient state is Received")
    let sent = try alice.conversation(contactId: charlieForAlice.contactId, limit: 10)
    check(sent.first?.state == .delivered, "sender sees Delivered only after authenticated proof")

    let reply = try charlie.sendMessage(contactId: aliceForCharlie.contactId, text: "reply", replyTo: mid,
                                        forwarded: false, nowSecs: t0 + 10)
    try charlie.react(contactId: aliceForCharlie.contactId, messageId: mid, reaction: 6, nowSecs: t0 + 11)
    try alice.editMessage(contactId: charlieForAlice.contactId, messageId: mid, text: "héllo, edited", nowSecs: t0 + 12)
    try exchange(t0 + 20)
    check(try alice.message(messageId: reply)?.replyTo == mid, "reply target crosses the boundary")
    let r = try alice.reactions(messageId: mid)
    check(r.count == 1 && reactionSet()[Int(r[0].reaction) - 1] == "❤️", "reaction index maps to the fixed set")
    let edited = try charlie.message(messageId: mid)
    check(edited?.body == "héllo, edited" && edited?.edited == true, "edit applied on the recipient")
    check(try charlie.search(query: "edited", contactId: nil, limit: 5).count == 1, "search callable from Swift")
    do {
        try alice.react(contactId: charlieForAlice.contactId, messageId: mid, reaction: 9, nowSecs: t0)
        check(false, "out-of-range reaction must throw")
    } catch VlError.InvalidReaction { check(true, "reaction validation crosses the boundary") }

    do {
        _ = try alice.sendText(contactId: Data(repeating: 0, count: 16), text: "x", nowSecs: t0)
        check(false, "unknown contact must throw")
    } catch VlError.UnknownContact { check(true, "typed error crosses the boundary") }
    do {
        _ = try alice.sendText(contactId: Data([1, 2, 3]), text: "x", nowSecs: t0)
        check(false, "bad id must throw")
    } catch VlError.InvalidInput { check(true, "input validation error crosses the boundary") }
    do {
        _ = try VlNode.open(relayDbPath: base.appendingPathComponent("alice-r.db").path, relayKey: key(9),
                            conversationDbPath: base.appendingPathComponent("alice-c.db").path, conversationKey: key(2))
        check(false, "wrong key must throw")
    } catch VlError.Storage { check(true, "wrong database key rejected") }
    let gid = try alice.createGroup(name: "Trip", myName: "Alice", contactIds: [charlieForAlice.contactId], nowSecs: t0 + 30)
    try exchange(t0 + 31)
    check(try charlie.groups().first?.name == "Trip", "group roster crosses the boundary")
    let gm = try charlie.sendGroupMessage(groupId: gid, text: "group hello", replyTo: nil, nowSecs: t0 + 32)
    try exchange(t0 + 33)
    check(try alice.groupMessages(groupId: gid, limit: 5).first?.body == "group hello", "group message delivered")
    let d = try charlie.groupDelivery(groupMsgId: gm)
    check(d.delivered == 1 && d.sent == 1, "group delivery counted with proof")
    check(maxTextBytes() == 1700, "free function callable")

    let doomed = try node("doomed")
    try doomed.createIdentity(nowSecs: t0)
    try doomed.panicWipe()
    let leftovers = try FileManager.default.contentsOfDirectory(atPath: base.path).filter { $0.hasPrefix("doomed") }
    check(leftovers.isEmpty, "panic wipe deletes the database files")
    do {
        try doomed.createIdentity(nowSecs: t0)
        check(false, "wiped node must refuse calls")
    } catch VlError.Wiped { check(true, "wiped node refuses every call") }
    do { try doomed.panicWipe(); check(false, "second wipe must throw") } catch VlError.Wiped { check(true, "second wipe reports Wiped") }
    print("ALL SWIFT INTEROP CHECKS PASSED")
} catch {
    print("FAIL: unexpected error \(error)"); exit(1)
}
