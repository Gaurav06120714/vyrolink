import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/app.dart';
import 'package:vyro_link/app/services.dart';
import 'package:vyro_link/features/chats/chats_screen.dart';
import 'package:vyro_link/features/conversation/composer.dart';
import 'package:vyro_link/features/conversation/conversation_screen.dart';
import 'package:vyro_link/features/settings/panic_wipe_screen.dart';

import 'fake_host.dart';

Future<void> settle(WidgetTester t) async {
  for (var i = 0; i < 10; i++) {
    await t.pump(const Duration(milliseconds: 50));
  }
}

void main() {
  testWidgets('chat list: empty state offers pairing', (t) async {
    await t.pumpWidget(harness(FakeHost(), const ChatsScreen()));
    await settle(t);
    expect(find.text('No conversations yet'), findsOneWidget);
    expect(find.text('Add a contact'), findsOneWidget);
  });

  testWidgets('chat list: preview, unread badge and honest status label', (t) async {
    final host = FakeHost()
      ..contactsList.addAll([contact(1, 'Asha'), contact(2, 'Ben')])
      ..messages.addAll([
        message(1, 1, 'see you at six', state: DeliveryState.acceptedByRelay),
        message(2, 2, 'hello there', outgoing: false),
      ]);
    await t.pumpWidget(harness(host, const ChatsScreen()));
    await settle(t);
    expect(find.text('see you at six'), findsOneWidget);
    expect(find.text('1'), findsOneWidget, reason: 'unread badge for Ben');
    final asha = t.getSemantics(find.ancestor(of: find.text('Asha'), matching: find.byType(Semantics)).first);
    expect(asha.label, contains('Accepted by a relay'));
    expect(asha.label, isNot(contains('Delivered')));
  });

  testWidgets('composer: counts UTF-8 bytes and blocks sending over the limit', (t) async {
    final controller = TextEditingController();
    var sent = <String>[];
    await t.pumpWidget(
      harness(
        FakeHost(),
        Scaffold(
          body: Align(
            alignment: Alignment.bottomCenter,
            child: Composer(
              controller: controller,
              maxBytes: 20,
              onSend: (s) async {
                sent.add(s);
                return true;
              },
            ),
          ),
        ),
      ),
    );
    controller.text = 'ééééééééééé';
    await t.pump();
    expect(find.text('2 bytes over the limit'), findsOneWidget);
    await t.tap(find.byTooltip('Send'));
    expect(sent, isEmpty);
    controller.text = 'hello';
    await settle(t);
    await t.tap(find.byTooltip('Send').last);
    await settle(t);
    expect(sent, ['hello']);
    expect(controller.text, isEmpty, reason: 'composer clears after send');
  });

  testWidgets('conversation: a queued message shows waiting, never Delivered', (t) async {
    final host = FakeHost()..contactsList.add(contact(1, 'Asha'));
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    await t.enterText(find.byType(TextField), 'north gate at six?');
    await settle(t);
    await t.tap(find.byTooltip('Send').last);
    await settle(t);
    expect(host.calls, contains('send north gate at six?'));
    expect(find.text('north gate at six?'), findsOneWidget);
    expect(find.text('Waiting for nearby devices'), findsOneWidget);
    expect(find.text('Delivered'), findsNothing);
  });

  testWidgets('conversation: send failure keeps the text and explains', (t) async {
    final host = FakeHost()..contactsList.add(contact(1, 'Asha'));
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    host.failNext = PlatformException(code: 'ContactBlocked');
    await t.enterText(find.byType(TextField), 'hello');
    await settle(t);
    await t.tap(find.byTooltip('Send').last);
    await settle(t);
    expect(find.text('You blocked this contact. Unblock them to send messages.'), findsOneWidget);
    expect(find.text('hello'), findsOneWidget, reason: 'the draft text is not lost');
  });

  testWidgets('conversation: delivered state comes only from the host', (t) async {
    final host = FakeHost()
      ..contactsList.add(contact(1, 'Asha'))
      ..messages.add(message(1, 1, 'arrived', state: DeliveryState.delivered));
    await t.pumpWidget(harness(host, ConversationScreen(contactId: id(1), initialName: 'Asha')));
    await settle(t);
    expect(find.text('Delivered'), findsOneWidget);
  });

  testWidgets('onboarding creates the identity, stores the name, asks for Bluetooth last', (t) async {
    final host = FakeHost();
    final services = AppServices(host);
    await t.pumpWidget(VyroApp(services: services));
    await settle(t);
    for (final label in ['Continue', 'Continue', 'Set up this phone']) {
      await t.tap(find.text(label));
      await settle(t);
    }
    expect(host.calls, isEmpty, reason: 'nothing happens before the user chooses');
    await t.enterText(find.byType(TextField), 'Asha');
    await t.tap(find.text('Create my identity'));
    await settle(t);
    expect(host.calls, ['createIdentity', 'setSetting profile.name=Asha']);
    expect(find.text('Turn on Bluetooth relay'), findsOneWidget);
    await t.tap(find.text('Not now'));
    await settle(t);
    expect(find.text('No conversations yet'), findsOneWidget);
    expect(host.calls.where((c) => c.startsWith('relay')), isEmpty);
  });

  testWidgets('identity creation failure is shown and does not advance', (t) async {
    final host = FakeHost()..failNext = PlatformException(code: 'KeyStore');
    await t.pumpWidget(VyroApp(services: AppServices(host)));
    await settle(t);
    for (final label in ['Continue', 'Continue', 'Set up this phone']) {
      await t.tap(find.text(label));
      await settle(t);
    }
    await t.tap(find.text('Create my identity'));
    await settle(t);
    expect(find.textContaining('passcode'), findsOneWidget);
    expect(find.text('Turn on Bluetooth relay'), findsNothing);
  });

  testWidgets('panic wipe: a key-store failure says nothing was erased', (t) async {
    final host = FakeHost()..failNext = PlatformException(code: 'KeyStore');
    await t.pumpWidget(harness(host, const PanicWipeScreen()));
    await settle(t);
    await t.tap(find.text('Erase everything'));
    await settle(t);
    await t.tap(find.text('Erase'));
    await settle(t);
    expect(host.calls, ['panicWipe']);
    expect(find.textContaining('nothing was erased'), findsOneWidget);
  });

  group('accessibility guidelines', () {
    for (final brightness in Brightness.values) {
      testWidgets('chat list meets tap-target, label and contrast guidelines ($brightness)', (t) async {
        final handle = t.ensureSemantics();
        final host = FakeHost()
          ..contactsList.add(contact(1, 'Asha', verified: true))
          ..messages.add(message(1, 1, 'see you', state: DeliveryState.delivered));
        await t.pumpWidget(harness(host, const ChatsScreen(), brightness: brightness));
        await settle(t);
        await expectLater(t, meetsGuideline(androidTapTargetGuideline));
        await expectLater(t, meetsGuideline(iOSTapTargetGuideline));
        await expectLater(t, meetsGuideline(labeledTapTargetGuideline));
        await expectLater(t, meetsGuideline(textContrastGuideline));
        handle.dispose();
      });

      testWidgets('conversation meets guidelines ($brightness)', (t) async {
        final handle = t.ensureSemantics();
        final host = FakeHost()
          ..contactsList.add(contact(1, 'Asha'))
          ..messages.addAll([
            message(1, 1, 'hello', outgoing: false),
            message(2, 1, 'hi, on my way', state: DeliveryState.acceptedByRelay),
          ]);
        await t.pumpWidget(
          harness(
            host,
            ConversationScreen(contactId: id(1), initialName: 'Asha'),
            brightness: brightness,
          ),
        );
        await settle(t);
        await expectLater(t, meetsGuideline(iOSTapTargetGuideline));
        await expectLater(t, meetsGuideline(labeledTapTargetGuideline));
        await expectLater(t, meetsGuideline(textContrastGuideline));
        handle.dispose();
      });
    }
  });

  testWidgets('text scaled to 200 % does not overflow the chat list', (t) async {
    final host = FakeHost()
      ..contactsList.add(contact(1, 'A very long contact name that wraps'))
      ..messages.add(message(1, 1, 'a long message preview that needs to be cut off somewhere sensible'));
    await t.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(textScaler: TextScaler.linear(2), size: Size(390, 844)),
        child: harness(host, const ChatsScreen()),
      ),
    );
    await settle(t);
    expect(t.takeException(), isNull);
  });
}
