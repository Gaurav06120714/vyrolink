import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/features/contacts/contact_screen.dart';
import 'package:vyro_link/features/conversation/attachment_view.dart';
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
