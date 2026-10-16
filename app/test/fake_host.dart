// Test double for the native host. Used only by widget tests; the real app
// always talks to the Rust core through VlHostApi.

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/services.dart';
import 'package:vyro_link/design/theme.dart';

Uint8List id(int n) => Uint8List.fromList(List.filled(16, n));

Contact contact(int n, String name, {bool verified = false, bool blocked = false}) => Contact(
  contactId: id(n),
  displayName: name,
  fingerprint: Uint8List.fromList(List.filled(32, n)),
  idCode: List.filled(32, 'A').join(),
  verified: verified,
  blocked: blocked,
  safetyNumber: List.filled(12, '12345').join(' '),
);

Message message(
  int n,
  int contactN,
  String body, {
  bool outgoing = true,
  DeliveryState state = DeliveryState.queued,
  int at = 1790000000,
  bool deleted = false,
}) => Message(
  messageId: id(n),
  contactId: id(contactN),
  outgoing: outgoing,
  body: body,
  createdAtSecs: at,
  state: outgoing ? state : DeliveryState.received,
  forwarded: false,
  deleted: deleted,
  edited: false,
  pinned: false,
  starred: false,
);

/// Alias usable inside [FakeHost], where `message` is the host method.
Message _make(int n, int contactN, String body) => message(n, contactN, body);

class FakeHost extends Fake implements VlHostApi {
  bool hasIdentity = false;
  final settings = <String, String>{};
  final contactsList = <Contact>[];
  final messages = <Message>[];
  final calls = <String>[];
  PlatformException? failNext;

  void _maybeFail() {
    final f = failNext;
    if (f != null) {
      failNext = null;
      throw f;
    }
  }

  @override
  Future<StartupInfo> start() async => StartupInfo(
    hasIdentity: hasIdentity,
    maxTextBytes: 1700,
    reactions: const ['👍', '👎', '😀', '😂', '😢', '❤️', '🚀', '✅'],
  );

  @override
  Future<void> createIdentity() async {
    calls.add('createIdentity');
    _maybeFail();
    hasIdentity = true;
  }

  @override
  Future<String?> setting(String key) async => settings[key];

  @override
  Future<void> setSetting(String key, String? value) async {
    calls.add('setSetting $key=$value');
    value == null ? settings.remove(key) : settings[key] = value;
  }

  @override
  Future<void> setRelayEnabled(bool enabled) async => calls.add('relay $enabled');

  @override
  Future<NetworkStatus> networkStatus() async => NetworkStatus(
    radio: RadioState.poweredOn,
    relayEnabled: false,
    relayMode: 2,
    charging: false,
    devicesSeenRecently: 0,
    scanning: false,
    advertising: false,
    openLinks: 0,
    encountersSinceLaunch: 0,
    ownEnvelopes: 0,
    carriedEnvelopes: 0,
    carriedBytes: 0,
  );

  @override
  Future<List<Contact>> contacts() async => contactsList;

  @override
  Future<List<ConversationSummary>> conversationSummaries() async => [
    for (final c in contactsList)
      ConversationSummary(
        contact: c,
        last: messages.where((m) => _same(m.contactId, c.contactId)).lastOrNull,
        unread: messages.where((m) => _same(m.contactId, c.contactId) && !m.outgoing).length,
      ),
  ];

  @override
  Future<List<Group>> groups() async => const [];

  @override
  Future<List<Message>> conversation(Uint8List contactId, int limit) async =>
      messages.where((m) => _same(m.contactId, contactId)).toList().reversed.take(limit).toList();

  @override
  Future<List<Reaction>> reactions(Uint8List messageId) async => const [];

  @override
  Future<int> disappearingTimer(Uint8List contactId) async => 0;

  @override
  Future<int> markRead(Uint8List contactId) async => 0;

  @override
  Future<String?> draft(Uint8List contactId) async => null;

  @override
  Future<void> saveDraft(Uint8List contactId, String text) async {}

  @override
  Future<Uint8List> sendMessage(Uint8List contactId, String text, Uint8List? replyTo, bool forwarded) async {
    calls.add('send $text');
    _maybeFail();
    final m = _make(100 + messages.length, contactId.first, text);
    messages.add(m);
    return m.messageId;
  }

  @override
  Future<void> panicWipe() async {
    calls.add('panicWipe');
    _maybeFail();
    hasIdentity = false;
  }

  @override
  Future<PermissionState> cameraPermission() async => PermissionState.notDetermined;

  static const myCode = '68QSTVZSN2PTTKR6XA2V77GH6JK0NPWF';
  bool canAuth = true;

  @override
  Future<String> myIdCode() async => myCode;

  @override
  Future<QrCode> qrCode(String text) async => QrCode(size: 21, modules: Uint8List(21 * 21));

  @override
  Future<bool> canAuthenticate() async => canAuth;

  @override
  Future<Message?> message(Uint8List messageId) async =>
      messages.where((m) => _same(m.messageId, messageId)).firstOrNull;

  @override
  Future<List<MessageEvent>> messageEvents(Uint8List messageId) async => const [];

  @override
  Future<Uint8List?> attachmentData(Uint8List messageId) async => null;

  @override
  Future<Uint8List> myFingerprint() async => Uint8List(32);

  static bool _same(Uint8List a, Uint8List b) => idKey(a) == idKey(b);
}

/// Wraps [child] like the real app does, with a fake host.
Widget harness(FakeHost host, Widget child, {Brightness brightness = Brightness.light}) {
  final services = AppServices(host);
  return AppScope(
    services: services,
    child: MaterialApp(theme: buildTheme(brightness, TargetPlatform.iOS), home: child),
  );
}
