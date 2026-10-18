import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/live.dart';
import '../../design/tokens.dart';
import '../../design/widgets/identity.dart';

/// Shows one of my undelivered messages as a QR code (ADR-036). The code is
/// the sealed envelope Bluetooth would carry; only the recipient can open it.
class PaperScreen extends StatelessWidget {
  const PaperScreen({super.key, required this.messageId, required this.recipient});
  final Uint8List messageId;
  final String recipient;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    return Scaffold(
      appBar: AppBar(title: const Text('Message as QR code')),
      body: Live<QrCode>(
        load: (api) async => api.qrCode(await api.paperMessage(messageId)),
        builder: (context, qr) => ListView(
          padding: EdgeInsets.all(Space.gutter(context)),
          children: [
            Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 360),
                child: QrPanel(code: qr, semanticLabel: 'Encrypted message code for $recipient'),
              ),
            ),
            const SizedBox(height: Space.l),
            Text(
              'Let $recipient scan this with VYRO LINK: Contacts → Add contact → Scan code.',
              style: t.bodyLarge,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: Space.s),
            Text(
              'It is the same end-to-end encrypted message that Bluetooth would carry. Only $recipient\'s phone can '
              'open it; anyone else who scans it sees nothing. "Delivered" appears once their phone confirms it over '
              'Bluetooth.',
              style: t.bodySmall!.copyWith(color: c.text2),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}
