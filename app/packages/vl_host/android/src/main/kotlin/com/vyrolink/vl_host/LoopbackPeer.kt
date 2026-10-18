package com.vyrolink.vl_host

import android.content.Context
import java.io.ByteArrayOutputStream
import java.io.File
import java.security.SecureRandom
import java.util.UUID
import uniffi.vl_ffi.Contact
import uniffi.vl_ffi.VlNode

/**
 * Debug builds only (NodeHost checks FLAG_DEBUGGABLE). A second, complete
 * node in the cache directory that pairs with the app's node and exchanges
 * over an in-memory link: the Bluetooth code path minus the radio.
 */
internal class LoopbackPeer private constructor(
  private val node: VlNode,
  private val dir: File,
  private val contactOfMe: ByteArray,
) {
  companion object {
    private fun now() = (System.currentTimeMillis() / 1000).toULong()

    fun create(context: Context, name: String, me: VlNode, myName: String): Pair<LoopbackPeer, Contact> {
      val dir = File(context.cacheDir, "vl-loopback-${UUID.randomUUID()}").apply { mkdirs() }
      val rnd = SecureRandom()
      fun key() = ByteArray(32).also(rnd::nextBytes)
      val node = VlNode.open(File(dir, "relay.db").path, key(), File(dir, "conversation.db").path, key())
      node.createIdentity(now())
      val mine = me.pairingCode(myName, now())
      val theirs = node.pairingCode(name, now())
      val contact = me.completePairingCode(theirs, now())
      val meInPeer = node.completePairingCode(mine, now())
      return LoopbackPeer(node, dir, meInPeer.contactId) to contact
    }
  }

  fun send(text: String) {
    node.sendText(contactOfMe, text, now())
  }

  fun sendFile(name: String, data: ByteArray) {
    node.sendAttachment(contactOfMe, 2u, name, data, "", null, now())
  }

  /** One encounter, 185-byte writes, until both sides go quiet. */
  fun exchange(me: VlNode) {
    val a = me.linkOpen(true)
    val b = node.linkOpen(false)
    try {
      var toPeer = a.bytesToSend
      var toMe = ByteArray(0)
      repeat(200) {
        if (toPeer.isEmpty() && toMe.isEmpty()) return@repeat
        val nextToMe = ByteArrayOutputStream()
        toPeer.asList().chunked(185).forEach { nextToMe.write(node.linkReceive(b.linkId, it.toByteArray(), now())) }
        val nextToPeer = ByteArrayOutputStream()
        toMe.asList().chunked(185).forEach { nextToPeer.write(me.linkReceive(a.linkId, it.toByteArray(), now())) }
        toMe = nextToMe.toByteArray()
        toPeer = nextToPeer.toByteArray()
      }
      node.markRead(contactOfMe, now())
    } finally {
      me.linkClose(a.linkId)
      node.linkClose(b.linkId)
    }
  }

  fun destroy() {
    runCatching { node.panicWipe() }
    node.destroy()
    dir.deleteRecursively()
  }
}
