package com.vyrolink.vl_host

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The two 32-byte SQLCipher keys (Class R relay, Class C conversation). Android
 * Keystore keys cannot be exported, so each database key is stored wrapped
 * (AES-256-GCM) by a non-exportable Keystore key in private preferences that
 * are excluded from backup. Deleting the Keystore key makes both database keys
 * unrecoverable (cryptographic erasure).
 */
internal class KeyStore(context: Context) {
  enum class Slot(val pref: String) { RELAY("relay"), CONVERSATION("conversation") }

  class Failure(val action: String, cause: Throwable? = null) : Exception("Keystore $action failed", cause)

  private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

  private fun keystore() = java.security.KeyStore.getInstance(PROVIDER).apply { load(null) }

  private fun wrapKey(create: Boolean): SecretKey? {
    (keystore().getKey(ALIAS, null) as? SecretKey)?.let { return it }
    if (!create) return null
    val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
    gen.init(
      KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setKeySize(256)
        .build(),
    )
    return gen.generateKey()
  }

  fun exists(slot: Slot): Boolean = prefs.contains(slot.pref) && runCatching { wrapKey(false) != null }.getOrDefault(false)

  /** Returns the key, creating it on first use. */
  @Synchronized
  fun key(slot: Slot): ByteArray {
    val wrap = try {
      wrapKey(create = true)!!
    } catch (e: Exception) {
      throw Failure("open", e)
    }
    val stored = prefs.getString(slot.pref, null)
    if (stored != null) {
      try {
        val blob = Base64.decode(stored, Base64.NO_WRAP)
        val cipher = Cipher.getInstance(TRANSFORM)
        cipher.init(Cipher.DECRYPT_MODE, wrap, GCMParameterSpec(128, blob, 0, IV_LEN))
        return cipher.doFinal(blob, IV_LEN, blob.size - IV_LEN).also { check(it.size == 32) }
      } catch (e: Exception) {
        throw Failure("read", e)
      }
    }
    val key = ByteArray(32).also { SecureRandom().nextBytes(it) }
    try {
      val cipher = Cipher.getInstance(TRANSFORM)
      cipher.init(Cipher.ENCRYPT_MODE, wrap)
      val blob = cipher.iv + cipher.doFinal(key)
      check(prefs.edit().putString(slot.pref, Base64.encodeToString(blob, Base64.NO_WRAP)).commit())
    } catch (e: Exception) {
      throw Failure("store", e)
    }
    return key
  }

  /** Deletes the wrapping key and the wrapped keys, and confirms they are gone. */
  @Synchronized
  fun destroyAll() {
    try {
      val ks = keystore()
      if (ks.containsAlias(ALIAS)) ks.deleteEntry(ALIAS)
      check(!keystore().containsAlias(ALIAS))
    } catch (e: Exception) {
      throw Failure("delete", e)
    }
    if (!prefs.edit().clear().commit()) throw Failure("delete wrapped keys")
  }

  private companion object {
    const val PROVIDER = "AndroidKeyStore"
    const val ALIAS = "com.vyrolink.db-wrap"
    const val PREFS = "com.vyrolink.db-keys"
    const val TRANSFORM = "AES/GCM/NoPadding"
    const val IV_LEN = 12
  }
}
