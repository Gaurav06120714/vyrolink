// The web chat's host (ADR-040): the same Rust core as the phones, compiled to
// WebAssembly (app/web/vlcore, built by tools/build-core-web.sh), with browser
// storage instead of Keychain/Keystore and no Bluetooth. Messages travel only
// through the internet mailbox here.
import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:math';
import 'dart:ui_web' as ui_web;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:vl_host/vl_host.dart';
import 'package:web/web.dart' as web;

@JS('vlcoreReady')
external JSPromise<JSAny?> get _coreReady;

@JS('VlWebNode')
extension type _Node._(JSObject _) implements JSObject {
  external factory _Node(JSUint8Array relay, JSUint8Array conversation, JSUint8Array relayKey, JSUint8Array convKey);
  external String call(String method, double now, String args);
  external JSUint8Array export(bool relay);
  external void wipe();
}

@JS('vlStore')
extension type _Store._(JSObject _) implements JSObject {
  external JSPromise<JSAny?> get(String key);
  external JSPromise<JSAny?> put(String key, JSAny value);
  external JSPromise<JSAny?> clear();
  external JSPromise<JSAny?> persist();
}

@JS('vlStore')
external _Store get _store;

@JS('vlPick')
external JSPromise<JSObject?> _pick(String accept);

@JS('vlShrink')
external JSPromise<JSUint8Array?> _shrink(JSUint8Array bytes, int maxBytes);

@JS('vlSave')
external void _save(String name, JSUint8Array bytes);

@JS('vlLocate')
external JSPromise<JSObject?> _locate();

@JS('vlCameraAllowed')
external JSPromise<JSBoolean> _cameraAllowed();

@JS('vlScan')
external JSPromise<JSFunction> _scan(web.HTMLVideoElement video, JSFunction onText);

const _maxImageBytes = 16 * 1024;
const _maxFileBytes = 64 * 1024;
const _scannerView = 'vl_web/qr_scanner';

double _now() => DateTime.now().millisecondsSinceEpoch / 1000;
Uint8List _b(Object? v) => Uint8List.fromList((v! as List).cast<int>());
Uint8List? _ob(Object? v) => v == null ? null : _b(v);
DeliveryState _state(Object? v) => DeliveryState.values.byName(v! as String);

Contact _contact(Map<String, dynamic> m) => Contact(
  contactId: _b(m['contactId']),
  displayName: m['displayName'] as String,
  fingerprint: _b(m['fingerprint']),
  idCode: m['idCode'] as String,
  verified: m['verified'] as bool,
  blocked: m['blocked'] as bool,
  safetyNumber: m['safetyNumber'] as String,
  lastSeenSecs: m['lastSeenSecs'] as int?,
  introducedBy: m['introducedBy'] as String?,
);

PollInfo _poll(Map<String, dynamic> m) => PollInfo(
  question: m['question'] as String,
  options: (m['options'] as List).cast<String>(),
  multi: m['multi'] as bool,
  counts: (m['counts'] as List).cast<int>(),
  mine: m['mine'] as int,
  voters: m['voters'] as int,
);

Message _message(Map<String, dynamic> m) {
  final a = m['attachment'] as Map<String, dynamic>?;
  final l = m['location'] as Map<String, dynamic>?;
  final i = m['intro'] as Map<String, dynamic>?;
  return Message(
    messageId: _b(m['messageId']),
    contactId: _b(m['contactId']),
    outgoing: m['outgoing'] as bool,
    body: m['body'] as String,
    createdAtSecs: m['createdAtSecs'] as int,
    state: _state(m['state']),
    replyTo: _ob(m['replyTo']),
    forwarded: m['forwarded'] as bool,
    deleted: m['deleted'] as bool,
    edited: m['edited'] as bool,
    disappearsAtSecs: m['disappearsAtSecs'] as int?,
    pinned: m['pinned'] as bool,
    starred: m['starred'] as bool,
    attachment: a == null
        ? null
        : AttachmentInfo(
            kind: a['kind'] as int,
            name: a['name'] as String,
            totalLen: a['totalLen'] as int,
            chunkCount: a['chunkCount'] as int,
            receivedChunks: a['receivedChunks'] as int,
            complete: a['complete'] as bool,
            deliveredChunks: a['deliveredChunks'] as int,
          ),
    location: l == null
        ? null
        : LocationInfo(
            latE7: l['latE7'] as int,
            lonE7: l['lonE7'] as int,
            accuracyM: l['accuracyM'] as int,
            capturedMs: l['capturedMs'] as int,
          ),
    poll: m['poll'] == null ? null : _poll(m['poll'] as Map<String, dynamic>),
    intro: i == null
        ? null
        : IntroInfo(introId: _b(i['introId']), otherName: i['otherName'] as String, state: i['state'] as int),
  );
}

List<Message> _messages(Object? v) => [for (final m in v! as List) _message(m as Map<String, dynamic>)];

Group _group(Map<String, dynamic> m) => Group(
  groupId: _b(m['groupId']),
  name: m['name'] as String,
  epoch: m['epoch'] as int,
  creatorFingerprint: _b(m['creatorFingerprint']),
  left: m['left'] as bool,
  members: [
    for (final x in m['members'] as List)
      GroupMember(fingerprint: _b((x as Map)['fingerprint']), name: x['name'] as String),
  ],
);

GroupMessage _groupMessage(Map<String, dynamic> m) => GroupMessage(
  groupMsgId: _b(m['groupMsgId']),
  authorFingerprint: _b(m['authorFingerprint']),
  outgoing: m['outgoing'] as bool,
  body: m['body'] as String,
  replyTo: _ob(m['replyTo']),
  createdAtSecs: m['createdAtSecs'] as int,
  poll: m['poll'] == null ? null : _poll(m['poll'] as Map<String, dynamic>),
);

/// Implements the host API in the browser.
class WebHost extends VlHostApi {
  WebHost();

  _Node? _node;
  Timer? _saveTimer;
  bool _camera = false;

  /// Receives scanned QR codes (the app's [VlEventsApi] implementation).
  VlEventsApi? events;

  static const _relayDb = 'relay.db';
  static const _conversationDb = 'conversation.db';
  static const _relayKey = 'key.relay';
  static const _conversationKey = 'key.conversation';

  _Node get _live => _node ?? (throw PlatformException(code: 'NotStarted', message: 'call start() first'));

  /// Calls the core and saves the databases shortly after (coalesced).
  Object? _call(String method, [List<Object?> args = const []]) {
    final out = jsonDecode(_live.call(method, _now(), jsonEncode(args))) as Map<String, dynamic>;
    _saveTimer ??= Timer(const Duration(milliseconds: 400), _flush);
    if (out['err'] case final String err) {
      // The app's error code, optionally "Code: reason", as on the phones.
      final colon = err.indexOf(': ');
      throw PlatformException(
        code: colon < 0 ? err : err.substring(0, colon),
        message: colon < 0 ? null : err.substring(colon + 2),
      );
    }
    return out['ok'];
  }

  Future<void> _flush() async {
    _saveTimer = null;
    final n = _node;
    if (n == null) return;
    await _store.put(_relayDb, n.export(true)).toDart;
    await _store.put(_conversationDb, n.export(false)).toDart;
  }

  Future<Uint8List> _bytes(String key) async =>
      ((await _store.get(key).toDart) as JSUint8Array?)?.toDart ?? Uint8List(0);

  Future<Uint8List> _key(String name) async {
    final have = await _bytes(name);
    if (have.length == 32) return have;
    final r = Random.secure();
    final k = Uint8List.fromList(List.generate(32, (_) => r.nextInt(256)));
    await _store.put(name, k.toJS).toDart;
    return k;
  }

  @override
  Future<StartupInfo> start() async {
    await _coreReady.toDart;
    if (_node == null) {
      unawaited(_store.persist().toDart);
      _node = _Node(
        (await _bytes(_relayDb)).toJS,
        (await _bytes(_conversationDb)).toJS,
        (await _key(_relayKey)).toJS,
        (await _key(_conversationKey)).toJS,
      );
      ui_web.platformViewRegistry.registerViewFactory(_scannerView, _scannerFactory);
      QrScannerView.webBuilder = () => const HtmlElementView(viewType: _scannerView);
      _call('maintain');
      if (const bool.fromEnvironment('VL_WEB_TEST')) _testHooks();
    }
    return StartupInfo(
      hasIdentity: _call('hasIdentity')! as bool,
      maxTextBytes: _call('maxTextBytes')! as int,
      reactions: (_call('reactionSet')! as List).cast<String>(),
    );
  }

  /// Two-tab tests only (built with VL_WEB_TEST): pairing without a camera.
  void _testHooks() {
    globalContext['vlTestPairingCode'] = ((JSString name) => pairingCode(name.toDart).then((c) => c.toJS).toJS).toJS;
    globalContext['vlTestCompletePairing'] = ((JSString code) => completePairing(
      code.toDart,
    ).then((c) => c.displayName.toJS).toJS).toJS;
  }

  web.HTMLElement _scannerFactory(int _) {
    final video = web.HTMLVideoElement()
      ..style.width = '100%'
      ..style.height = '100%'
      ..style.objectFit = 'cover';
    JSFunction? stop;
    _scan(video, ((JSString t) => events?.qrScanned(t.toDart)).toJS).toDart.then((s) {
      stop = s;
      // Stop the camera as soon as the scan screen goes away.
      Timer.periodic(const Duration(seconds: 1), (timer) {
        if (!video.isConnected) {
          stop?.callAsFunction();
          timer.cancel();
        }
      });
    }).ignore();
    return video;
  }

  // Lifecycle and identity. There is no Keychain-style store in a browser
  // that outlives clearing site data, so the ID is not kept across that.

  @override
  Future<bool> createIdentity() async {
    _call('createIdentity');
    await _flush();
    return false;
  }

  @override
  Future<Uint8List> myFingerprint() async => _b(_call('myFingerprint'));
  @override
  Future<String?> setting(String key) async => _call('setting', [key]) as String?;
  @override
  Future<void> setSetting(String key, String? value) async => _call('setSetting', [key, value]);
  @override
  Future<void> maintain() async => _call('maintain');
  @override
  Future<String> myIdCode() async => _call('myIdCode')! as String;

  @override
  Future<void> panicWipe() async {
    _saveTimer?.cancel();
    _saveTimer = null;
    _node?.wipe();
    _node = null;
    await _store.clear().toDart;
  }

  // Internet mailbox

  @override
  Future<List<Uint8List>> internetOutbox() async => [for (final e in _call('internetOutbox')! as List) _b(e)];
  @override
  Future<void> internetUploaded(List<Uint8List> envelopes) async => _call('internetUploaded', [envelopes]);
  @override
  Future<List<Uint8List>> internetTags() async => [for (final e in _call('internetTags')! as List) _b(e)];
  @override
  Future<bool> receiveInternet(Uint8List envelope) async => _call('receiveInternet', [envelope])! as bool;

  // Pairing and contacts

  @override
  Future<String> pairingCode(String displayName) async => _call('pairingCode', [displayName])! as String;
  @override
  Future<QrCode> qrCode(String text) async {
    final m = _call('qrCode', [text])! as Map<String, dynamic>;
    return QrCode(size: m['size'] as int, modules: _b(m['modules']));
  }

  @override
  Future<Contact> completePairing(String code) async =>
      _contact(_call('completePairing', [code])! as Map<String, dynamic>);
  @override
  Future<List<Contact>> contacts() async => [
    for (final c in _call('contacts')! as List) _contact(c as Map<String, dynamic>),
  ];
  @override
  Future<void> setVerified(Uint8List contactId, bool verified) async => _call('setVerified', [contactId, verified]);
  @override
  Future<void> setBlocked(Uint8List contactId, bool blocked) async => _call('setBlocked', [contactId, blocked]);
  @override
  Future<void> renameContact(Uint8List contactId, String name) async => _call('renameContact', [contactId, name]);
  @override
  Future<void> deleteContact(Uint8List contactId) async => _call('deleteContact', [contactId]);

  // Conversations

  @override
  Future<List<ConversationSummary>> conversationSummaries() async => [
    for (final s in _call('conversationSummaries')! as List)
      ConversationSummary(
        contact: _contact((s as Map<String, dynamic>)['contact'] as Map<String, dynamic>),
        last: s['last'] == null ? null : _message(s['last'] as Map<String, dynamic>),
        unread: s['unread'] as int,
      ),
  ];
  @override
  Future<List<Message>> conversation(Uint8List contactId, int limit) async =>
      _messages(_call('conversation', [contactId, limit]));
  @override
  Future<Message?> message(Uint8List messageId) async {
    final m = _call('message', [messageId]);
    return m == null ? null : _message(m as Map<String, dynamic>);
  }

  @override
  Future<Uint8List> sendMessage(Uint8List contactId, String text, Uint8List? replyTo, bool forwarded) async =>
      _b(_call('sendMessage', [contactId, text, replyTo, forwarded]));
  @override
  Future<void> react(Uint8List contactId, Uint8List messageId, int reaction) async =>
      _call('react', [contactId, messageId, reaction]);
  @override
  Future<List<Reaction>> reactions(Uint8List messageId) async => [
    for (final r in _call('reactions', [messageId])! as List)
      Reaction(fromMe: (r as Map)['fromMe'] as bool, reaction: r['reaction'] as int),
  ];
  @override
  Future<void> editMessage(Uint8List contactId, Uint8List messageId, String text) async =>
      _call('editMessage', [contactId, messageId, text]);
  @override
  Future<void> deleteForEveryone(Uint8List contactId, Uint8List messageId) async =>
      _call('deleteForEveryone', [contactId, messageId]);
  @override
  Future<void> deleteForMe(Uint8List messageId) async => _call('deleteForMe', [messageId]);
  @override
  Future<void> setDisappearingTimer(Uint8List contactId, int timerSecs) async =>
      _call('setDisappearingTimer', [contactId, timerSecs]);
  @override
  Future<int> disappearingTimer(Uint8List contactId) async => _call('disappearingTimer', [contactId])! as int;
  @override
  Future<int> markRead(Uint8List contactId) async => _call('markRead', [contactId])! as int;
  @override
  Future<List<Message>> search(String query, Uint8List? contactId, int limit) async =>
      _messages(_call('search', [query, contactId, limit]));
  @override
  Future<void> saveDraft(Uint8List contactId, String text) async => _call('saveDraft', [contactId, text]);
  @override
  Future<String?> draft(Uint8List contactId) async => _call('draft', [contactId]) as String?;
  @override
  Future<void> setPinned(Uint8List messageId, bool pinned) async => _call('setPinned', [messageId, pinned]);
  @override
  Future<void> setStarred(Uint8List messageId, bool starred) async => _call('setStarred', [messageId, starred]);
  @override
  Future<List<Message>> flagged(Uint8List? contactId, bool pinned) async =>
      _messages(_call('flagged', [contactId, pinned]));
  @override
  Future<List<Message>> pendingOutgoing() async => _messages(_call('pendingOutgoing'));
  @override
  Future<List<MessageEvent>> messageEvents(Uint8List messageId) async => [
    for (final e in _call('messageEvents', [messageId])! as List)
      MessageEvent(atSecs: (e as Map)['atSecs'] as int, state: _state(e['state'])),
  ];
  @override
  Future<void> resend(Uint8List messageId) async => _call('resend', [messageId]);

  // Attachments

  @override
  Future<Uint8List> sendAttachment(
    Uint8List contactId,
    int kind,
    String name,
    Uint8List data,
    String caption,
    Uint8List? replyTo,
  ) async => _b(_call('sendAttachment', [contactId, kind, name, data, caption, replyTo]));
  @override
  Future<Uint8List?> attachmentData(Uint8List messageId) async => _ob(_call('attachmentData', [messageId]));

  Future<PickedFile?> _picked(String accept) async {
    final f = await _pick(accept).toDart;
    if (f == null) return null;
    return PickedFile(
      name: (f['name']! as JSString).toDart,
      size: (f['size']! as JSNumber).toDartInt,
      data: (f['data']! as JSUint8Array).toDart,
    );
  }

  @override
  Future<PickedFile?> pickImage() async {
    final f = await _picked('image/*');
    if (f == null || f.data == null) return null;
    final jpeg = (await _shrink(f.data!.toJS, _maxImageBytes).toDart)?.toDart;
    if (jpeg == null) return null;
    return PickedFile(name: 'photo.jpg', size: jpeg.length, data: jpeg);
  }

  @override
  Future<PickedFile?> pickFile() async {
    final f = await _picked('*/*');
    if (f == null) return null;
    // Same rule as the phones: too large comes back without data.
    return f.size <= _maxFileBytes ? f : PickedFile(name: f.name, size: f.size);
  }

  @override
  Future<void> shareFile(String name, Uint8List data) async => _save(name, data.toJS);

  @override
  Future<void> setCarryCapMb(int mb) async {
    final m = mb.clamp(1, 100);
    _call('setSetting', ['relay.carry_cap_mb', '$m']);
    _call('setCarryCap', [m << 20]);
  }

  @override
  Future<int> carryCapMb() async => (_call('carryCap')! as int) >> 20;

  // Groups

  @override
  Future<Uint8List> createGroup(String name, String myName, List<Uint8List> contactIds) async =>
      _b(_call('createGroup', [name, myName, contactIds]));
  @override
  Future<void> setGroupMembers(Uint8List groupId, String myName, List<Uint8List> contactIds) async =>
      _call('setGroupMembers', [groupId, myName, contactIds]);
  @override
  Future<void> leaveGroup(Uint8List groupId) async => _call('leaveGroup', [groupId]);
  @override
  Future<List<Group>> groups() async => [for (final g in _call('groups')! as List) _group(g as Map<String, dynamic>)];
  @override
  Future<List<GroupMessage>> groupMessages(Uint8List groupId, int limit) async => [
    for (final m in _call('groupMessages', [groupId, limit])! as List) _groupMessage(m as Map<String, dynamic>),
  ];
  @override
  Future<Uint8List> sendGroupMessage(Uint8List groupId, String text, Uint8List? replyTo) async =>
      _b(_call('sendGroupMessage', [groupId, text, replyTo]));
  @override
  Future<GroupDelivery> groupDelivery(Uint8List groupMsgId) async {
    final m = _call('groupDelivery', [groupMsgId])! as Map<String, dynamic>;
    return GroupDelivery(delivered: m['delivered'] as int, sent: m['sent'] as int);
  }

  // No Bluetooth in a browser: the network status says so honestly.

  @override
  Future<NetworkStatus> networkStatus() async {
    final s = _call('stats')! as Map<String, dynamic>;
    return NetworkStatus(
      radio: RadioState.unsupported,
      relayEnabled: false,
      relayMode: 0,
      charging: false,
      devicesSeenRecently: 0,
      scanning: false,
      advertising: false,
      openLinks: 0,
      encountersSinceLaunch: 0,
      ownEnvelopes: s['ownEnvelopes'] as int,
      carriedEnvelopes: s['carriedEnvelopes'] as int,
      carriedBytes: s['carriedBytes'] as int,
    );
  }

  @override
  Future<List<NearbyContact>> nearbyContacts(int withinSecs) async => const [];
  @override
  Future<void> setRelayEnabled(bool enabled) async {}
  @override
  Future<void> setRelayMode(int mode) async {}
  @override
  Future<int> resyncSecs() async => _call('resyncSecs')! as int;
  @override
  Future<int> setResyncSecs(int secs) async => _call('setResyncSecs', [secs.clamp(1, 45)])! as int;

  // Sharing (ADR-034..037)

  @override
  Future<LocationInfo?> currentLocation() async {
    final p = await _locate().toDart;
    if (p == null) return null;
    double n(String k) => (p[k]! as JSNumber).toDartDouble;
    return LocationInfo(
      latE7: (n('lat') * 1e7).round(),
      lonE7: (n('lon') * 1e7).round(),
      accuracyM: n('acc').round().clamp(0, 65535),
      capturedMs: n('at').round(),
    );
  }

  @override
  Future<Uint8List> sendLocation(Uint8List contactId, LocationInfo fix) async =>
      _b(_call('sendLocation', [contactId, fix.latE7, fix.lonE7, fix.accuracyM, fix.capturedMs]));

  @override
  Future<void> openMap(int latE7, int lonE7, String label) async {
    final lat = (latE7 / 1e7).toStringAsFixed(6), lon = (lonE7 / 1e7).toStringAsFixed(6);
    web.window.open('https://www.openstreetmap.org/?mlat=$lat&mlon=$lon#map=16/$lat/$lon', '_blank', 'noopener');
  }

  @override
  Future<Uint8List> sendPoll(Uint8List contactId, String question, List<String> options, bool multi) async =>
      _b(_call('sendPoll', [contactId, question, options, multi]));
  @override
  Future<Uint8List> sendGroupPoll(Uint8List groupId, String question, List<String> options, bool multi) async =>
      _b(_call('sendGroupPoll', [groupId, question, options, multi]));
  @override
  Future<void> vote(Uint8List pollId, int choices) async => _call('vote', [pollId, choices.clamp(0, 65535)]);
  @override
  Future<Uint8List> introduce(Uint8List contactA, Uint8List contactB) async =>
      _b(_call('introduce', [contactA, contactB]));
  @override
  Future<void> acceptIntro(Uint8List introId, String myName) async => _call('acceptIntro', [introId, myName]);
  @override
  Future<void> declineIntro(Uint8List introId) async => _call('declineIntro', [introId]);
  @override
  Future<String> paperMessage(Uint8List messageId) async => _call('paperMessage', [messageId])! as String;
  @override
  Future<Uint8List> receivePaper(String code) async => _b(_call('receivePaper', [code]));

  // Device features a browser does not have.

  @override
  Future<bool> canAuthenticate() async => false;
  @override
  Future<bool> authenticate(String reason) async => false;
  @override
  Future<void> setScreenCaptureBlocked(bool blocked) async {}

  @override
  Future<PermissionState> cameraPermission() async => _camera ? PermissionState.granted : PermissionState.notDetermined;

  @override
  Future<PermissionState> requestCameraPermission() async {
    _camera = (await _cameraAllowed().toDart).toDart;
    return _camera ? PermissionState.granted : PermissionState.denied;
  }

  @override
  Future<PermissionState> requestNotificationPermission() async => PermissionState.denied;
  @override
  Future<void> openAppSettings() async {}

  @override
  Future<Contact> debugCreatePeer(String name) => throw PlatformException(code: 'NotAvailable');
  @override
  Future<void> debugPeerSend(String text) => throw PlatformException(code: 'NotAvailable');
  @override
  Future<void> debugPeerSendFile(String name, Uint8List data) => throw PlatformException(code: 'NotAvailable');
  @override
  Future<void> debugExchange() => throw PlatformException(code: 'NotAvailable');
}
