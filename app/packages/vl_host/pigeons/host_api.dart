// Contract between the Flutter UI and the native hosts (Doc 18 §6.3, ADR-013).
// Regenerate after editing: `dart run pigeon --input pigeons/host_api.dart`
// (from app/packages/vl_host). The UI never holds keys, never parses
// envelopes and never talks to Bluetooth; the hosts own the Rust node.
import 'package:pigeon/pigeon.dart';

@ConfigurePigeon(
  PigeonOptions(
    dartOut: 'lib/src/host_api.g.dart',
    swiftOut: 'ios/vl_host/Sources/vl_host/HostApi.g.swift',
    kotlinOut: 'android/src/main/kotlin/com/vyrolink/vl_host/HostApi.g.kt',
    kotlinOptions: KotlinOptions(package: 'com.vyrolink.vl_host'),
    dartPackageName: 'vl_host',
  ),
)
/// Doc 18 §7.4. `delivered` is shown only after an authenticated receipt or
/// vaccine proof (ADR-015/020); relay custody is `acceptedByRelay`.
enum DeliveryState {
  queued,
  sentToNearby,
  acceptedByRelay,
  delivered,
  read,
  expired,
  deliveryUnknown,
  failed,
  received,
}

enum RadioState { unknown, unsupported, unauthorized, poweredOff, poweredOn }

/// A paired contact proven to be in Bluetooth range (ADR-029). The hosts keep
/// it in memory only; it is never stored.
class NearbyContact {
  NearbyContact({required this.contactId, required this.seenSecs});
  Uint8List contactId;

  /// Unix seconds of the last link that showed this contact's presence tag.
  int seenSecs;
}

enum PermissionState { granted, denied, notDetermined, restricted }

class StartupInfo {
  StartupInfo({required this.hasIdentity, required this.maxTextBytes, required this.reactions});
  bool hasIdentity;
  int maxTextBytes;
  List<String> reactions;
}

class Contact {
  Contact({
    required this.contactId,
    required this.displayName,
    required this.fingerprint,
    required this.idCode,
    required this.verified,
    required this.blocked,
    required this.safetyNumber,
    this.lastSeenSecs,
    this.introducedBy,
  });
  Uint8List contactId;
  String displayName;
  Uint8List fingerprint;

  /// 32-character ID code (A–Z, 0–9) derived from the contact's identity key.
  String idCode;
  bool verified;
  bool blocked;
  String safetyNumber;

  /// Unix seconds this contact was last proven nearby over Bluetooth (ADR-033).
  int? lastSeenSecs;

  /// Who introduced this contact (ADR-037); null if met in person.
  String? introducedBy;
}

class Message {
  Message({
    required this.messageId,
    required this.contactId,
    required this.outgoing,
    required this.body,
    required this.createdAtSecs,
    required this.state,
    this.replyTo,
    required this.forwarded,
    required this.deleted,
    required this.edited,
    this.disappearsAtSecs,
    required this.pinned,
    required this.starred,
    this.attachment,
    this.location,
    this.poll,
    this.intro,
  });
  Uint8List messageId;
  Uint8List contactId;
  bool outgoing;
  String body;
  int createdAtSecs;
  DeliveryState state;
  Uint8List? replyTo;
  bool forwarded;
  bool deleted;
  bool edited;
  int? disappearsAtSecs;
  bool pinned;
  bool starred;
  AttachmentInfo? attachment;
  LocationInfo? location;
  PollInfo? poll;
  IntroInfo? intro;
}

/// An introduction under its message (ADR-037). `state`: 0 asked, 1 accepted,
/// 2 declined, 3 connected, 4 failed (the card did not match).
class IntroInfo {
  IntroInfo({required this.introId, required this.otherName, required this.state});
  Uint8List introId;
  String otherName;
  int state;
}

/// A shared position (ADR-034): degrees × 10⁷ (WGS 84), accuracy in metres,
/// and when the GPS fix was taken (Unix ms).
class LocationInfo {
  LocationInfo({required this.latE7, required this.lonE7, required this.accuracyM, required this.capturedMs});
  int latE7;
  int lonE7;
  int accuracyM;
  int capturedMs;
}

/// A poll with live counts (ADR-035). Bit i of `mine` = I chose option i.
class PollInfo {
  PollInfo({
    required this.question,
    required this.options,
    required this.multi,
    required this.counts,
    required this.mine,
    required this.voters,
  });
  String question;
  List<String> options;
  bool multi;
  List<int> counts;
  int mine;
  int voters;
}

/// ADR-026. `kind` 1 = image (JPEG, ≤ 16 KiB), 2 = file (≤ 64 KiB).
class AttachmentInfo {
  AttachmentInfo({
    required this.kind,
    required this.name,
    required this.totalLen,
    required this.chunkCount,
    required this.receivedChunks,
    required this.complete,
    required this.deliveredChunks,
  });
  int kind;
  String name;
  int totalLen;
  int chunkCount;
  int receivedChunks;
  bool complete;

  /// For my attachments: chunks whose delivery was proven by receipts.
  int deliveredChunks;
}

/// A file chosen by the user. `data` is null when it is over the size limit.
class PickedFile {
  PickedFile({required this.name, required this.size, this.data});
  String name;
  int size;
  Uint8List? data;
}

class ConversationSummary {
  ConversationSummary({required this.contact, this.last, required this.unread});
  Contact contact;
  Message? last;
  int unread;
}

class Reaction {
  Reaction({required this.fromMe, required this.reaction});
  bool fromMe;

  /// 1..=8, index + 1 into [StartupInfo.reactions].
  int reaction;
}

class MessageEvent {
  MessageEvent({required this.atSecs, required this.state});
  int atSecs;
  DeliveryState state;
}

class GroupMember {
  GroupMember({required this.fingerprint, required this.name});
  Uint8List fingerprint;
  String name;
}

class Group {
  Group({
    required this.groupId,
    required this.name,
    required this.epoch,
    required this.creatorFingerprint,
    required this.left,
    required this.members,
  });
  Uint8List groupId;
  String name;
  int epoch;
  Uint8List creatorFingerprint;
  bool left;
  List<GroupMember> members;
}

class GroupMessage {
  GroupMessage({
    required this.groupMsgId,
    required this.authorFingerprint,
    required this.outgoing,
    required this.body,
    this.replyTo,
    required this.createdAtSecs,
    this.poll,
  });
  Uint8List groupMsgId;
  Uint8List authorFingerprint;
  bool outgoing;
  String body;
  Uint8List? replyTo;
  int createdAtSecs;
  PollInfo? poll;
}

class GroupDelivery {
  GroupDelivery({required this.delivered, required this.sent});
  int delivered;
  int sent;
}

/// A QR symbol, `size` × `size`, row-major, 1 = dark, no quiet zone.
class QrCode {
  QrCode({required this.size, required this.modules});
  int size;
  Uint8List modules;
}

/// Counters only. Encounter figures are kept in memory by the host and reset
/// when the app process ends.
class NetworkStatus {
  NetworkStatus({
    required this.radio,
    required this.relayEnabled,
    required this.relayMode,
    required this.charging,
    required this.devicesSeenRecently,
    required this.scanning,
    required this.advertising,
    required this.openLinks,
    required this.encountersSinceLaunch,
    this.lastExchangeSecs,
    required this.ownEnvelopes,
    required this.carriedEnvelopes,
    required this.carriedBytes,
  });
  RadioState radio;
  bool relayEnabled;

  /// Android: 1 = only while charging, 2 = always (foreground service). iOS: 2.
  int relayMode;
  bool charging;

  /// Distinct nearby VYRO LINK devices seen in the last 24 h (a count only).
  int devicesSeenRecently;
  bool scanning;
  bool advertising;
  int openLinks;
  int encountersSinceLaunch;
  int? lastExchangeSecs;
  int ownEnvelopes;
  int carriedEnvelopes;
  int carriedBytes;
}

/// Errors arrive as `PlatformException` whose `code` is a `VlError` case name
/// (for example `TextTooLong`, `Wiped`, `OwnPairingCode`) or `KeyStore` /
/// `Bluetooth` for host-side failures.
@HostApi()
abstract class VlHostApi {
  // Lifecycle and identity.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  StartupInfo start();
  /// Internet mailbox (ADR-039): sealed envelopes to upload (mine first),
  /// confirmation that the mailbox holds them, the tags to fetch, and one
  /// fetched envelope (true if it was new). The app does the HTTP itself.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Uint8List> internetOutbox();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void internetUploaded(List<Uint8List> envelopes);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Uint8List> internetTags();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  bool receiveInternet(Uint8List envelope);

  /// Creates this phone's identity, or brings back the one this phone had
  /// before a reinstall (ADR-038). True when it was restored.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  bool createIdentity();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List myFingerprint();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  String? setting(String key);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setSetting(String key, String? value);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void maintain();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  String myIdCode();

  /// Deletes both database keys from the Keychain/Keystore first, then the
  /// databases. Throws if any step fails; the UI must not report success then.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void panicWipe();

  // Pairing and contacts.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  String pairingCode(String displayName);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  QrCode qrCode(String text);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Contact completePairing(String code);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Contact> contacts();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setVerified(Uint8List contactId, bool verified);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setBlocked(Uint8List contactId, bool blocked);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void renameContact(Uint8List contactId, String name);

  /// Deletes the contact and the whole conversation with them.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void deleteContact(Uint8List contactId);

  // One-to-one conversations.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<ConversationSummary> conversationSummaries();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Message> conversation(Uint8List contactId, int limit);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Message? message(Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendMessage(Uint8List contactId, String text, Uint8List? replyTo, bool forwarded);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void react(Uint8List contactId, Uint8List messageId, int reaction);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Reaction> reactions(Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void editMessage(Uint8List contactId, Uint8List messageId, String text);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void deleteForEveryone(Uint8List contactId, Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void deleteForMe(Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setDisappearingTimer(Uint8List contactId, int timerSecs);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  int disappearingTimer(Uint8List contactId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  /// Returns how many messages became read.
  int markRead(Uint8List contactId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Message> search(String query, Uint8List? contactId, int limit);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void saveDraft(Uint8List contactId, String text);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  String? draft(Uint8List contactId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setPinned(Uint8List messageId, bool pinned);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setStarred(Uint8List messageId, bool starred);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Message> flagged(Uint8List? contactId, bool pinned);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Message> pendingOutgoing();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<MessageEvent> messageEvents(Uint8List messageId);

  /// Re-encrypts an expired or unconfirmed message and queues it again.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void resend(Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendAttachment(
    Uint8List contactId,
    int kind,
    String name,
    Uint8List data,
    String caption,
    Uint8List? replyTo,
  );
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List? attachmentData(Uint8List messageId);

  /// Photo picker; the host re-encodes to a JPEG of at most 16 KiB without metadata.
  @async
  PickedFile? pickImage();

  /// Document picker; files over 64 KiB come back with `data` null.
  @async
  PickedFile? pickFile();

  /// System share sheet for a received or sent attachment.
  @async
  void shareFile(String name, Uint8List data);

  /// Relay storage for others' mail, in MiB (1–100); persisted and applied at start.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setCarryCapMb(int mb);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  int carryCapMb();

  // Small groups (ADR-024).
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List createGroup(String name, String myName, List<Uint8List> contactIds);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void setGroupMembers(Uint8List groupId, String myName, List<Uint8List> contactIds);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void leaveGroup(Uint8List groupId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<Group> groups();
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<GroupMessage> groupMessages(Uint8List groupId, int limit);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendGroupMessage(Uint8List groupId, String text, Uint8List? replyTo);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  GroupDelivery groupDelivery(Uint8List groupMsgId);

  // Bluetooth relay and permissions.
  NetworkStatus networkStatus();

  /// One GPS fix for "Share location" (ADR-034). Asks for location permission
  /// the first time; null if refused or no fix within about 20 s.
  @async
  LocationInfo? currentLocation();

  /// Shares a fix with a contact; returns the message id.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendLocation(Uint8List contactId, LocationInfo fix);

  /// Opens the system Maps app at a position (it may need the internet).
  void openMap(int latE7, int lonE7, String label);

  /// Polls (ADR-035): start one in a chat or a group; returns its id.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendPoll(Uint8List contactId, String question, List<String> options, bool multi);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List sendGroupPoll(Uint8List groupId, String question, List<String> options, bool multi);

  /// Introductions (ADR-037).
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List introduce(Uint8List contactA, Uint8List contactB);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void acceptIntro(Uint8List introId, String myName);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void declineIntro(Uint8List introId);

  /// QR "paper" message (ADR-036): the sealed envelope of one of my
  /// undelivered messages as `VLM1:` text, and taking a scanned one (returns
  /// the contact id it came from).
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  String paperMessage(Uint8List messageId);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Uint8List receivePaper(String code);

  /// My whole choice for a poll (bit i = option i; 0 withdraws).
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void vote(Uint8List pollId, int choices);

  /// Paired, unblocked contacts seen over Bluetooth in the last `withinSecs`.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  List<NearbyContact> nearbyContacts(int withinSecs);

  /// Starts or stops scanning, advertising and link sessions. The choice is
  /// persisted. On iOS the first start shows the system Bluetooth prompt.
  void setRelayEnabled(bool enabled);

  /// Android: 1 = relay only while charging, 2 = always (foreground service).
  void setRelayMode(int mode);

  /// Seconds before syncing with the same nearby phone again (ADR-032): 1 ..= 45, default 5.
  int resyncSecs();

  /// Stores the interval (clamped to 1 ..= 45) and returns the stored value.
  int setResyncSecs(int secs);

  /// Device biometrics or passcode can be used for the app lock.
  bool canAuthenticate();
  @async
  bool authenticate(String reason);

  /// Android: blocks screenshots and recents thumbnails. iOS cannot do this.
  void setScreenCaptureBlocked(bool blocked);
  PermissionState cameraPermission();
  @async
  PermissionState requestCameraPermission();
  @async
  PermissionState requestNotificationPermission();
  void openAppSettings();

  /// Debug builds only (throws `NotAvailable` in release): a second node in a
  /// temporary directory, paired with this one, exchanging over an in-memory
  /// link instead of Bluetooth. Real crypto, real databases.
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  Contact debugCreatePeer(String name);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void debugPeerSend(String text);

  /// The peer sends this file as an attachment (real chunking and encryption).
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void debugPeerSendFile(String name, Uint8List data);
  @TaskQueue(type: TaskQueueType.serialBackgroundThread)
  void debugExchange();
}

/// Host → UI notifications. Only sent while the UI is attached.
@FlutterApi()
abstract class VlEventsApi {
  /// Stored data may have changed (a link session delivered or accepted mail).
  void dataChanged();
  void networkChanged(NetworkStatus status);
  void qrScanned(String text);
}
