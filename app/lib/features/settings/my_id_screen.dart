import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/live.dart';
import '../../app/services.dart';
import '../../design/theme.dart';
import '../../design/tokens.dart';
import '../../design/widgets/identity.dart';
import '../../design/widgets/states.dart';

/// The ID code in eight groups of four, for reading aloud and comparing.
String groupedIdCode(String code) =>
    [for (var i = 0; i < code.length; i += 4) code.substring(i, (i + 4).clamp(0, code.length))].join(' ');

/// "My ID": this phone's 32-character code and its QR. The code is derived
/// from the identity key, so it is unique without any account or server.
/// It is for showing and comparing; adding a contact still uses the pairing
/// code, which carries the keys.
class MyIdScreen extends StatelessWidget {
  const MyIdScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('My ID')),
      body: Live<(String, QrCode)>(
        load: (api) async {
          final code = await api.myIdCode();
          return (code, await api.qrCode(code));
        },
        builder: (context, data) {
          final (code, qr) = data;
          final name = context.services.profileName;
          return ListView(
            padding: EdgeInsets.all(Space.gutter(context)),
            children: [
              if (name.isNotEmpty) Text(name, style: t.headlineSmall, textAlign: TextAlign.center),
              const SizedBox(height: Space.l),
              Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 260),
                  child: QrPanel(code: qr, semanticLabel: 'QR code of your VYRO LINK ID'),
                ),
              ),
              const SizedBox(height: Space.xl),
              Semantics(
                label: 'Your ID: ${code.split('').join(' ')}',
                excludeSemantics: true,
                child: Container(
                  padding: const EdgeInsets.symmetric(vertical: Space.l, horizontal: Space.m),
                  decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
                  child: SelectableText(
                    groupedIdCode(code),
                    textAlign: TextAlign.center,
                    style: monoStyle(context).copyWith(fontSize: 18, letterSpacing: 1.2, height: 1.5),
                  ),
                ),
              ),
              const SizedBox(height: Space.s),
              TextButton.icon(
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: code));
                  showMessage(context, 'ID copied');
                },
                icon: const Icon(Icons.copy_rounded),
                label: const Text('Copy ID'),
              ),
              const SizedBox(height: Space.l),
              Text(
                'Your ID is made from your phone\'s secret identity key, so no other person can have the same one '
                'and nobody can fake it. It works without an account and without the internet.',
                style: t.bodyMedium!.copyWith(color: c.text2),
              ),
              const SizedBox(height: Space.m),
              Text(
                'People you are paired with see this exact ID on your contact card. Compare it to be sure you have '
                'the right person. To add someone, use Add contact: their pairing code carries the keys this ID '
                'is made from.',
                style: t.bodySmall,
              ),
              const SizedBox(height: Space.m),
              Text(
                'Reinstalling VYRO LINK on this phone keeps this ID. Panic wipe erases it for good, '
                'and the next setup creates a new one.',
                style: t.bodySmall,
              ),
            ],
          );
        },
      ),
    );
  }
}
