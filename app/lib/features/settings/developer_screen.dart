import 'package:flutter/material.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';

/// Debug builds only. Drives a second, real node inside this app over an
/// in-memory link (no Bluetooth), so the simulator can show sending, receipts
/// and incoming messages end to end. Release builds refuse these calls.
class DeveloperScreen extends StatefulWidget {
  const DeveloperScreen({super.key});

  @override
  State<DeveloperScreen> createState() => _DeveloperScreenState();
}

class _DeveloperScreenState extends State<DeveloperScreen> {
  final _text = TextEditingController(text: 'Hello from the test peer');
  String? _peer;

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  Future<void> _do(Future<void> Function() op, String done) async {
    try {
      await op();
      if (mounted) showMessage(context, done);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, '${e.message} (${e.detail ?? e.code.name})');
    }
  }

  Future<void> _createPeer() async {
    try {
      final c = await context.services.mutate((api) => api.debugCreatePeer('Test peer'));
      if (!mounted) return;
      setState(() => _peer = c.displayName);
      await Routes.conversation(context, c);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, '${e.message} (${e.detail ?? e.code.name})');
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    final t = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Developer tools')),
      body: ListView(
        padding: const EdgeInsets.only(bottom: Space.xxl),
        children: [
          const InfoBanner(
            kind: BannerKind.warn,
            text:
                'Loopback test peer: a second VYRO LINK node inside this app, with its own keys and databases, '
                'exchanging over memory instead of Bluetooth. It is not a real device.',
          ),
          const SectionHeader('Loopback peer'),
          Section(
            rows: [
              ListTile(
                leading: const Icon(Icons.person_add_alt_outlined),
                title: Text(_peer == null ? 'Create and pair "Test peer"' : 'Paired with $_peer'),
                onTap: _createPeer,
              ),
              ListTile(
                leading: const Icon(Icons.sync_alt_rounded),
                title: const Text('Run one exchange'),
                subtitle: const Text('Like two phones meeting for a moment'),
                onTap: () => _do(() => s.mutate((api) => api.debugExchange()), 'Exchange finished'),
              ),
            ],
          ),
          const SectionHeader('Message from the peer'),
          Padding(
            padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  key: const Key('peer-text'),
                  controller: _text,
                  decoration: const InputDecoration(labelText: 'Text'),
                ),
                const SizedBox(height: Space.s),
                Text('The peer queues it; run an exchange to deliver it.', style: t.bodySmall),
                const SizedBox(height: Space.m),
                OutlinedButton(
                  onPressed: () => _do(() => s.call((api) => api.debugPeerSend(_text.text)), 'Queued on the peer'),
                  child: const Text('Peer sends'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
