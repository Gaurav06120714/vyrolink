// Real-phone check for ADR-038, run right after uninstalling and reinstalling
// the app: set-up must bring back the same ID. The test then pairs in person
// with the other phone (a person holds the phones together and taps the iOS
// permission prompts) and exchanges one message each way over Bluetooth.
//
// Run: flutter drive --profile --driver test_driver/integration_test.dart \
//        --target integration_test/device_reinstall_test.dart -d <iphone> \
//        --dart-define=VL_NAME=<name> --dart-define=VL_EXPECT_ID=<id before>
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/app.dart';
import 'package:vyro_link/app/services.dart';
import 'package:vyro_link/features/pairing/pairing_screen.dart';

import 'app_flow_test.dart' as flow show binding;
import 'app_flow_test.dart' show tapText, waitFor;

const name = String.fromEnvironment('VL_NAME');
const expectId = String.fromEnvironment('VL_EXPECT_ID');

void main() {
  final binding = flow.binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('a reinstall keeps the ID, then pairs and talks over Bluetooth', (t) async {
    final api = VlHostApi();
    // Never wipe here: the point is what survived the uninstall.
    final alreadySetUp = (await api.start()).hasIdentity;
    final services = AppServices(api)..attach();
    await t.pumpWidget(VyroApp(services: services));

    if (!alreadySetUp) {
      await waitFor(t, find.text('Continue'));
      await tapText(t, 'Continue');
      await tapText(t, 'Continue');
      await tapText(t, 'Set up this phone');
      await waitFor(t, find.byType(TextField));
      await t.enterText(find.byType(TextField), name);
      await tapText(t, 'Create my identity');
      await waitFor(t, find.textContaining('kept your VYRO LINK ID'));
      debugPrint('VL_STEP tap Allow on the Bluetooth prompt');
      await tapText(t, 'Turn on Bluetooth relay');
    }
    // A new identity would have a different ID, so this is the real check.
    final id = await api.myIdCode();
    debugPrint('VL_STEP set up before this run=$alreadySetUp id=$id');
    expect(id.startsWith(expectId), isTrue, reason: 'same ID as before the uninstall');

    final existing = await api.contacts();
    debugPrint('VL_STEP contacts=${existing.map((c) => '${c.displayName} ${c.idCode}').toList()}');
    expect(existing, isEmpty, reason: 'pair from this script only on a fresh reinstall');
    await waitFor(t, find.text('Add a contact'), timeout: const Duration(minutes: 2));
    await tapText(t, 'Add a contact');
    await tapText(t, 'Scan code');
    if (find.text('Allow camera').evaluate().isNotEmpty) {
      debugPrint('VL_STEP tap Allow on the camera prompt');
      await tapText(t, 'Allow camera');
    }
    debugPrint('VL_STEP hold the iPhone camera over the Samsung code');
    await waitFor(t, find.textContaining('You scanned'), timeout: const Duration(minutes: 5));
    final contact = (await api.contacts()).single;
    debugPrint('VL_STEP scanned ${contact.displayName} id=${contact.idCode}');

    // Close the verify sheet so My code is visible for the other phone.
    if (find.byType(VerifySheet).evaluate().isNotEmpty) {
      Navigator.of(t.element(find.byType(VerifySheet))).pop();
      await t.pump(const Duration(seconds: 1));
    }
    debugPrint('VL_STEP show the iPhone code to the Samsung camera');

    await api.sendMessage(contact.contactId, 'Hello from the reinstalled iPhone', null, false);
    final end = DateTime.now().add(const Duration(minutes: 6));
    var delivered = false;
    var reply = false;
    while (DateTime.now().isBefore(end) && !(delivered && reply)) {
      await t.pump(const Duration(seconds: 2));
      final msgs = await api.conversation(contact.contactId, 20);
      delivered = msgs.any((m) => m.outgoing && m.state == DeliveryState.delivered);
      reply = msgs.any((m) => !m.outgoing && m.body.isNotEmpty);
    }
    debugPrint('VL_STEP delivered=$delivered reply=$reply');
    expect(delivered, isTrue, reason: 'the Samsung confirmed receipt');
    expect(reply, isTrue, reason: 'a message from the Samsung arrived');
    await binding.convertFlutterSurfaceToImage();
  }, timeout: const Timeout(Duration(minutes: 20)));
}
