import Foundation
import Security

/// The two 32-byte SQLCipher keys (Class R relay, Class C conversation) live in
/// the Keychain, never on disk next to the databases. They are readable after
/// the first unlock so background Bluetooth relaying works, and never leave
/// this device (no iCloud Keychain, no backups).
enum KeyStore {
  enum Slot: String, CaseIterable { case relay, conversation }

  struct Failure: Error { let status: OSStatus; let action: String }

  private static let service = "com.vyrolink.db-keys"

  private static func query(_ slot: Slot) -> [String: Any] {
    [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: slot.rawValue,
    ]
  }

  /// Returns the key, creating it on first use.
  static func key(_ slot: Slot) throws -> Data {
    var q = query(slot)
    q[kSecReturnData as String] = true
    q[kSecMatchLimit as String] = kSecMatchLimitOne
    var item: CFTypeRef?
    let status = SecItemCopyMatching(q as CFDictionary, &item)
    if status == errSecSuccess, let data = item as? Data, data.count == 32 { return data }
    guard status == errSecItemNotFound else { throw Failure(status: status, action: "read") }

    var bytes = Data(count: 32)
    let rng = bytes.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, 32, $0.baseAddress!) }
    guard rng == errSecSuccess else { throw Failure(status: rng, action: "generate") }
    var add = query(slot)
    add[kSecValueData as String] = bytes
    add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
    let added = SecItemAdd(add as CFDictionary, nil)
    guard added == errSecSuccess else { throw Failure(status: added, action: "store") }
    return bytes
  }

  static func exists(_ slot: Slot) -> Bool {
    SecItemCopyMatching(query(slot) as CFDictionary, nil) == errSecSuccess
  }

  /// Deletes both keys and confirms they are gone. This is the step that makes
  /// the encrypted databases unrecoverable (cryptographic erasure).
  static func destroyAll() throws {
    for slot in Slot.allCases {
      let status = SecItemDelete(query(slot) as CFDictionary)
      guard status == errSecSuccess || status == errSecItemNotFound else {
        throw Failure(status: status, action: "delete")
      }
      guard !exists(slot) else { throw Failure(status: errSecDuplicateItem, action: "verify deletion") }
    }
  }
}
