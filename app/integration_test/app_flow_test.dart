// End-to-end on a simulator or device, against the real native host and Rust
// core (Keychain keys, SQLCipher, Olm, link sessions). The second party is the
// debug-only loopback peer: a real node exchanging over memory, not Bluetooth.
//
// Run: flutter drive --driver test_driver/integration_test.dart \
//        --target integration_test/app_flow_test.dart -d <simulator>
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/app.dart';
import 'package:vyro_link/app/services.dart';

late IntegrationTestWidgetsFlutterBinding binding;

/// Pumps frames until [finder] matches (pumpAndSettle cannot be used while
/// spinners or the relay pulse animate).
Future<void> waitFor(WidgetTester t, Finder finder, {Duration timeout = const Duration(seconds: 20)}) async {
  final end = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(end)) {
    await t.pump(const Duration(milliseconds: 100));
    if (finder.evaluate().isNotEmpty) return;
  }
  throw TestFailure('Timed out waiting for $finder');
}

Future<void> tapText(WidgetTester t, String text) async {
  final f = find.text(text);
  try {
    await waitFor(t, f, timeout: const Duration(seconds: 3));
  } on TestFailure {
    // Lazily built lists only contain what is on screen; scroll to it. Tabs
    // keep their scroll position, so the target may be above or below.
    final list = find.byType(Scrollable).last;
    try {
      await t.scrollUntilVisible(f, 200, scrollable: list, maxScrolls: 20);
    } on StateError {
      await t.scrollUntilVisible(f, -200, scrollable: list);
    }
  }
  await t.ensureVisible(f.first);
  await t.pump();
  // A route that is still animating away can cover the target; wait until a
  // tap would really land on it.
  await waitFor(t, f.hitTestable());
  await t.tap(f.hitTestable().first);
  await t.pump(const Duration(milliseconds: 400));
}

Future<void> popToRoot(WidgetTester t) async {
  final nav = t.state<NavigatorState>(find.byType(Navigator).first);
  nav.popUntil((r) => r.isFirst);
  await t.pump(const Duration(milliseconds: 800));
}

Future<void> shot(WidgetTester t, String name) async {
  await t.pump(const Duration(milliseconds: 600));
  await binding.takeScreenshot(name);
}

void main() {
  binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('onboarding, messaging with receipts, reactions and panic wipe', (t) async {
    final api = VlHostApi();
    // Start from nothing: the Keychain survives reinstalls on simulators.
    if ((await api.start()).hasIdentity) await api.panicWipe();

    final services = AppServices(api)..attach();
    await t.pumpWidget(VyroApp(services: services));
    await binding.convertFlutterSurfaceToImage();

    // Onboarding.
    await waitFor(t, find.text('Continue'));
    await shot(t, '01-onboarding');
    await tapText(t, 'Continue');
    await tapText(t, 'Continue');
    await tapText(t, 'Set up this phone');
    await waitFor(t, find.text('Create my identity'));
    await t.enterText(find.byType(TextField), 'Asha');
    await shot(t, '02-identity');
    await tapText(t, 'Create my identity');
    await waitFor(t, find.text('Not now'));
    await shot(t, '03-bluetooth');
    await tapText(t, 'Not now');

    // Empty home.
    await waitFor(t, find.text('No conversations yet'));
    expect(find.text('Bluetooth relay is off. New messages wait on this phone.'), findsOneWidget);
    await shot(t, '04-chats-empty');

    // Pair with the loopback peer (developer tools, debug builds only).
    await tapText(t, 'Settings');
    await shot(t, '05-settings');
    await tapText(t, 'Developer tools');
    await tapText(t, 'Create and pair "Test peer"');

    // Conversation: send; it must wait, not claim delivery.
    await waitFor(t, find.textContaining('You will see Delivered only when'));
    await t.pump(const Duration(milliseconds: 600));
    await t.enterText(find.byType(TextField).last, 'Are we still meeting at the north gate at six?');
    await t.pump(const Duration(milliseconds: 400));
    await t.pump(const Duration(milliseconds: 100));
    await t.tap(find.byTooltip('Send').last);
    await waitFor(t, find.text('Waiting for nearby devices'));
    expect(find.text('Delivered'), findsNothing);
    await shot(t, '06-chat-waiting');

    // Peer replies; one exchange carries both directions and the receipt.
    await t.pageBack();
    await t.pump(const Duration(milliseconds: 500));
    await waitFor(t, find.byKey(const Key('peer-text')));
    await t.enterText(find.byKey(const Key('peer-text')), 'Yes. I will bring the map.');
    await tapText(t, 'Peer sends');
    await tapText(t, 'Run one exchange');
    await popToRoot(t);
    await tapText(t, 'Chats');
    await waitFor(t, find.text('Test peer'));
    await shot(t, '07-chats-list');
    await tapText(t, 'Test peer');
    await waitFor(t, find.text('Yes. I will bring the map.'));
    await waitFor(t, find.text('Delivered'));
    await shot(t, '08-chat-delivered');

    // React to the incoming message.
    await t.longPress(find.text('Yes. I will bring the map.'));
    await t.pump(const Duration(milliseconds: 600));
    await shot(t, '09-message-actions');
    await t.tap(find.bySemanticsLabel('React 👍').first);
    await t.pump(const Duration(milliseconds: 800));
    await shot(t, '10-reaction');

    // Delivery details for my message.
    await t.longPress(find.text('Are we still meeting at the north gate at six?'));
    await t.pump(const Duration(milliseconds: 600));
    await tapText(t, 'Delivery details');
    await waitFor(t, find.text('TIMELINE'));
    expect(find.text('Encrypted and queued'), findsOneWidget);
    await shot(t, '11-delivery-details');
    await popToRoot(t);

    // Network tab.
    await tapText(t, 'Network');
    await shot(t, '12-network');

    // Relay settings, My ID and Privacy.
    await tapText(t, 'Relay mode and storage');
    await waitFor(t, find.text('Relay settings'));
    await shot(t, '13a-relay-settings');
    await popToRoot(t);
    await tapText(t, 'Settings');
    await tapText(t, 'My ID');
    await waitFor(t, find.text('Copy ID'));
    expect(find.textContaining(RegExp(r'^([0-9A-Z]{4} ){7}[0-9A-Z]{4}$')), findsOneWidget);
    await shot(t, '13b-my-id');
    await popToRoot(t);
    await tapText(t, 'Privacy');
    await waitFor(t, find.text('App lock'));
    await shot(t, '13c-privacy');
    await popToRoot(t);

    // Panic wipe returns to onboarding with a fresh, empty store.
    await tapText(t, 'Settings');
    await tapText(t, 'Panic wipe');
    await shot(t, '13-panic-wipe');
    await tapText(t, 'Erase everything');
    await tapText(t, 'Erase');
    await waitFor(t, find.text('Messages without\nthe internet'));
    final after = await api.start();
    expect(after.hasIdentity, isFalse);
  });
}
