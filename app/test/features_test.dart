import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/services.dart';
import 'package:vyro_link/design/widgets/nearby.dart';
import 'package:vyro_link/features/contacts/contact_screen.dart';
import 'package:vyro_link/features/conversation/attachment_view.dart';
import 'package:vyro_link/features/conversation/share_view.dart';
import 'package:vyro_link/features/conversation/conversation_screen.dart';
import 'package:vyro_link/features/network/network_screen.dart';
import 'package:vyro_link/features/settings/my_id_screen.dart';
import 'package:vyro_link/features/settings/privacy_screen.dart';

import 'fake_host.dart';

Future<void> settle(WidgetTester t) async {
  for (var i = 0; i < 10; i++) {
    await t.pump(const Duration(milliseconds: 50));
  }
}

/// SOI, an APP0 segment, then a SOF0 frame header declaring `w`×`h`.
Uint8List jpeg(int w, int h) => Uint8List.fromList([
  0xFF, 0xD8, //
  0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, //
  0xFF, 0xC0, 0x00, 0x11, 0x08, h >> 8, h & 0xFF, w >> 8, w & 0xFF, 0x03, 0, 0, 0, 0, 0, 0,
]);

Message withAttachment(int n, {required bool outgoing, required AttachmentInfo info, DeliveryState? state}) {
  final m = message(n, 1, '', outgoing: outgoing, state: state ?? DeliveryState.queued);
  m.attachment = info;
  return m;
}

void main() {
  group('safe image decoding', () {
    test('reads JPEG dimensions from the header only', () {
      expect(jpegSize(jpeg(640, 480)), (640, 480));
      expect(jpegSize(Uint8List.fromList([0x89, 0x50, 0x4E, 0x47])), isNull, reason: 'PNG is not shown as an image');
      expect(jpegSize(jpeg(0, 10)), isNull);
      expect(jpegSize(Uint8List(0)), isNull);
      final bomb = jpegSize(jpeg(60000, 60000))!;
      expect(bomb.$1 > maxImageSide, isTrue, reason: 'oversized canvas is detected before decoding');
    });
  });

  test('ID code grouping and nearby buckets', () {
    expect(groupedIdCode(FakeHost.myCode), '68QS TVZS N2PT TKR6 XA2V 77GH 6JK0 NPWF');
    expect([0, 1, 2, 3, 9, 10, 99].map(nearbyBucket), ['None', '1–2', '1–2', '3–9', '3–9', '10+', '10+']);
  });

  test('diagnostics report carries counters only', () {
    final n = NetworkStatus(
      radio: RadioState.poweredOn,
      relayEnabled: true,
      relayMode: 2,
      charging: false,
      devicesSeenRecently: 4,
      scanning: true,
      advertising: true,
      openLinks: 1,
      encountersSinceLaunch: 7,
      ownEnvelopes: 3,
      carriedEnvelopes: 2,
      carriedBytes: 4096,
    );
    final text = diagnosticsReport(n, 'iOS', DateTime.utc(2026, 10, 16));
    expect(text, contains('Exchanges since start: 7'));
    expect(text, contains('Phones met in last day: 3–9'));
    expect(text, isNot(contains(FakeHost.myCode)));
    expect(utf8.encode(text).length, lessThan(1024));
  });

  test('distance and direction are computed on the phone', () {
    // About 1.1 km due north, then about 1.1 km due east (near the equator).
    expect(distanceMetres(0, 0, 100000, 0), closeTo(1112, 2));
    expect(compassPoint(bearingDegrees(0, 0, 100000, 0)), 'N');
    expect(compassPoint(bearingDegrees(0, 0, 0, 100000)), 'E');
    expect(compassPoint(bearingDegrees(0, 0, -100000, -100000)), 'SW');
    expect(formatDistance(850), '850 m');
    expect(formatDistance(2340), '2.3 km');
  });

  testWidgets('a poll shows counts and a tap sends my whole new choice', (t) async {
    final host = FakeHost();
    final poll = PollInfo(
      question: 'Start time?',
      options: ['7:00', '8:00'],
      multi: false,
      counts: [2, 1],
      mine: 2, // 8:00
      voters: 3,
    );
    await t.pumpWidget(
      harness(
        host,
        Scaffold(
          body: PollCard(pollId: id(9), poll: poll, onBubble: false),
        ),
      ),
    );
    await settle(t);
    expect(find.text('Start time?'), findsOneWidget);
    expect(find.text('Choose one · 3 votes'), findsOneWidget);
    await t.tap(find.text('7:00'));
    await settle(t);
    // Single choice: picking 7:00 replaces my 8:00 vote.
    expect(host.votes.single, (idKey(id(9)), 1));
  });

  testWidgets('a location card shows coordinates and accuracy', (t) async {
    final info = LocationInfo(latE7: 174_123_456, lonE7: 785_654_321, accuracyM: 9, capturedMs: 0);
    await t.pumpWidget(
      harness(
        FakeHost(),
        Scaffold(
          body: LocationCard(info: info, onBubble: false, label: 'Asha'),
        ),
      ),
    );
    await settle(t);
    expect(find.textContaining('17.41235, 78.56543 · ±9 m'), findsOneWidget);
    expect(find.text('Distance from me'), findsOneWidget);
    expect(find.text('Open in Maps'), findsOneWidget);
  });

  testWidgets('an introduction can be accepted and then shows its progress', (t) async {
    final host = FakeHost();
    Widget card(int state) => harness(
      host,
      Scaffold(
        body: IntroCard(
          body: 'Asha wants to introduce you to Ravi',
          intro: IntroInfo(introId: id(4), otherName: 'Ravi', state: state),
          onBubble: false,
        ),
      ),
    );
    await t.pumpWidget(card(0));
    await settle(t);
    t.element(find.byType(IntroCard)).services.profileName = 'Bala';
    await t.tap(find.text('Accept'));
    await settle(t);
    expect(host.accepted.single, (idKey(id(4)), 'Bala'));
    await t.pumpWidget(card(3));
    await settle(t);
    expect(find.textContaining('You can now message Ravi'), findsOneWidget);
    expect(find.text('Accept'), findsNothing);
  });

  testWidgets('an introduced contact says who introduced them', (t) async {
    final c = contact(1, 'Ravi')..introducedBy = 'Asha';
    final host = FakeHost()..contactsList.add(c);
    await t.pumpWidget(harness(host, ContactScreen(contactId: id(1))));
    await settle(t);
    expect(find.text('Introduced by Asha, not met in person'), findsOneWidget);
  });

  test('seen-ago wording', () {
    final now = DateTime.utc(2026, 10, 16, 12);
    final secs = now.millisecondsSinceEpoch ~/ 1000;
    expect(seenAgo(secs - 20, now: now), 'just now');
    expect(seenAgo(secs - 4 * 60, now: now), '4 min ago');
    expect(seenAgo(secs - 3 * 3600, now: now), '3 h ago');
    expect(seenAgo(secs - 3 * 86400, now: now), '3 days ago');
  });

  testWidgets('a contact not near now shows when it was last seen nearby', (t) async {
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final c = contact(1, 'Asha')..lastSeenSecs = now - 3 * 3600;
    final host = FakeHost()..contactsList.add(c);
    await t.pumpWidget(harness(host, ContactScreen(contactId: id(1))));
    await settle(t);
    expect(find.text('Last seen nearby 3 h ago'), findsOneWidget);
  });

  testWidgets('a paired contact shows Nearby only while the host reports them', (t) async {
    final host = FakeHost()..contactsList.add(contact(1, 'Asha'));
    await t.pumpWidget(harness(host, ContactScreen(contactId: id(1))));
    await settle(t);
    final services = t.element(find.byType(ContactScreen)).services;
    expect(find.textContaining('Nearby'), findsNothing);

    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    host.nearby.add(NearbyContact(contactId: id(1), seenSecs: now - 120));
    await services.refreshNetwork();
    await settle(t);
    expect(find.text('Nearby · seen 2 min ago'), findsOneWidget);

    host.nearby.clear();
    await services.refreshNetwork();
    await settle(t);
    expect(find.textContaining('Nearby'), findsNothing);
  });

  testWidgets('renaming a contact closes the dialog cleanly and shows the new name', (t) async {
    final host = FakeHost()..contactsList.add(contact(1, 'Asha'));
    await t.pumpWidget(harness(host, ContactScreen(contactId: id(1))));
    await settle(t);
    await t.scrollUntilVisible(find.text('Rename'), 200, scrollable: find.byType(Scrollable).first);
    await t.tap(find.text('Rename'));
    await t.pumpAndSettle();
    await t.enterText(find.byType(TextField).last, 'Trail buddy');
    await t.tap(find.text('Save'));
    await t.pumpAndSettle();
    expect(t.takeException(), isNull);
    expect(host.contactsList.single.displayName, 'Trail buddy');
    await t.scrollUntilVisible(find.text('Trail buddy'), -200, scrollable: find.byType(Scrollable).first);
    expect(find.text('Trail buddy'), findsWidgets);
  });

  testWidgets('an edited message still opens its actions on long-press', (t) async {
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final host = FakeHost()
      ..contactsList.add(contact(1, 'Asha'))
      ..messages.add(message(5, 1, 'Meet at six', at: now, edited: true));
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    expect(find.textContaining('Edited'), findsOneWidget);
    await t.longPress(find.text('Meet at six'));
    await settle(t);
    expect(find.text('Pin'), findsOneWidget);
    expect(find.text('Star'), findsOneWidget);
  });

  testWidgets('poll answers inside a chat bubble are reachable by screen readers', (t) async {
    // Found on the Samsung on 2026-10-18: the bubble hid its card's buttons.
    final handle = t.ensureSemantics();
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final host = FakeHost()
      ..contactsList.add(contact(1, 'Asha'))
      ..messages.add(
        message(5, 1, 'Lunch where', at: now)
          ..poll = PollInfo(
            question: 'Lunch where',
            options: ['Canteen', 'Cafe'],
            multi: false,
            counts: [0, 0],
            mine: 0,
            voters: 0,
          ),
      );
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    expect(find.bySemanticsLabel(RegExp('Canteen')), findsOneWidget);
    handle.dispose();
  });

  testWidgets('My ID shows the 32-character code in groups', (t) async {
    await t.pumpWidget(harness(FakeHost(), const MyIdScreen()));
    await settle(t);
    expect(find.text('68QS TVZS N2PT TKR6 XA2V 77GH 6JK0 NPWF'), findsOneWidget);
    expect(find.text('Copy ID'), findsOneWidget);
  });

  testWidgets('contact profile shows their ID and offers rename, re-pair and delete', (t) async {
    final host = FakeHost()..contactsList.add(contact(1, 'Asha'));
    await t.pumpWidget(harness(host, ContactScreen(contactId: id(1))));
    await settle(t);
    expect(find.text('AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA'), findsOneWidget);
    await t.scrollUntilVisible(find.text('Delete contact'), 200, scrollable: find.byType(Scrollable).first);
    expect(find.text('Rename'), findsOneWidget);
    expect(find.text('Pair again'), findsOneWidget);
  });

  testWidgets('incoming attachment shows honest progress until complete', (t) async {
    final host = FakeHost()
      ..contactsList.add(contact(1, 'Asha'))
      ..messages.add(
        withAttachment(
          5,
          outgoing: false,
          info: AttachmentInfo(
            kind: 2,
            name: 'map.pdf',
            totalLen: 20000,
            chunkCount: 15,
            receivedChunks: 6,
            complete: false,
            deliveredChunks: 0,
          ),
        ),
      );
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    expect(find.text('Receiving 6 of 15 parts'), findsOneWidget);
  });

  testWidgets('a sent file is not called delivered until every part is confirmed', (t) async {
    final host = FakeHost()
      ..contactsList.add(contact(1, 'Asha'))
      ..messages.add(
        withAttachment(
          6,
          outgoing: true,
          state: DeliveryState.delivered,
          info: AttachmentInfo(
            kind: 2,
            name: 'notes.txt',
            totalLen: 5000,
            chunkCount: 4,
            receivedChunks: 4,
            complete: true,
            deliveredChunks: 1,
          ),
        ),
      );
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    expect(find.text('Message delivered · 1 of 4 file parts confirmed'), findsOneWidget);
    expect(find.text('Delivered'), findsNothing);
  });

  testWidgets('app lock cannot be turned on without a device passcode', (t) async {
    final host = FakeHost()..canAuth = false;
    await t.pumpWidget(harness(host, const PrivacyScreen()));
    await settle(t);
    expect(find.text('Set a passcode on this phone first'), findsOneWidget);
    final toggle = t.widget<SwitchListTile>(find.byType(SwitchListTile).first);
    expect(toggle.onChanged, isNull);
  });
}
