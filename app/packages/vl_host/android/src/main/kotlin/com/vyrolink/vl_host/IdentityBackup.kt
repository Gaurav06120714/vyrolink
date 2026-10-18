package com.vyrolink.vl_host

import android.content.Context
import com.google.android.gms.auth.blockstore.Blockstore
import com.google.android.gms.auth.blockstore.DeleteBytesRequest
import com.google.android.gms.auth.blockstore.RetrieveBytesRequest
import com.google.android.gms.auth.blockstore.StoreBytesData
import com.google.android.gms.tasks.Task
import com.google.android.gms.tasks.Tasks
import java.util.concurrent.TimeUnit

/**
 * The long-term identity keys in Google Block Store, so a reinstall on this
 * phone keeps the same ID (ADR-038). Block Store keeps the bytes across an
 * uninstall when Settings > Google > Backup is on. Cloud backup is never
 * requested, so they stay on this phone. Calls block, so they must run on the
 * Pigeon background queue. Without Play services, nothing is kept.
 */
internal class IdentityBackup(context: Context) {
  private val client = Blockstore.getClient(context)

  private fun <T> await(task: Task<T>): T = Tasks.await(task, 10, TimeUnit.SECONDS)

  fun load(): ByteArray? = runCatching {
    await(client.retrieveBytes(RetrieveBytesRequest.Builder().setKeys(listOf(KEY)).build()))
      .blockstoreDataMap[KEY]?.bytes
  }.getOrNull()

  fun save(bytes: ByteArray) {
    if (load()?.contentEquals(bytes) == true) return
    runCatching {
      await(client.storeBytes(StoreBytesData.Builder().setKey(KEY).setBytes(bytes).setShouldBackupToCloud(false).build()))
    }
  }

  /**
   * Deletes the backup; throws if it can still be read afterwards, so panic
   * wipe can say so. ponytail: "unreadable" counts as gone (no Play services
   * means nothing was kept); a Play services outage during a wipe could hide
   * a copy, which is reported only if it is readable again.
   */
  fun destroy() {
    runCatching { await(client.deleteBytes(DeleteBytesRequest.Builder().setKeys(listOf(KEY)).build())) }
    check(load() == null) { "the reinstall copy of this phone's ID could not be erased" }
  }

  private companion object {
    const val KEY = "com.vyrolink.identity"
  }
}
