import 'package:flutter/material.dart';

import '../../app/live.dart';
import '../../design/theme.dart';
import '../../design/tokens.dart';
import '../../design/widgets/identity.dart';
import '../../design/widgets/states.dart';

/// Doc 16 screen 14: the security model in plain language, including what it
/// does not protect against, and the review status (never overstated).
class AboutScreen extends StatelessWidget {
  const AboutScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    Widget point(String title, String body) => ListTile(
      title: Text(title, style: t.titleSmall),
      subtitle: Padding(
        padding: const EdgeInsets.only(top: Space.xs),
        child: Text(body, style: t.bodyMedium!.copyWith(color: c.text2)),
      ),
    );
    return Scaffold(
      appBar: AppBar(title: const Text('How VYRO LINK protects you')),
      body: ListView(
        padding: const EdgeInsets.only(bottom: Space.xxl),
        children: [
          const SectionHeader('What others can see'),
          Section(
            rows: [
              point(
                'Phones that carry your messages',
                'They hold sealed envelopes with a random-looking address that changes every 6 hours. They cannot read the text or tell who wrote it.',
              ),
              point(
                'People nearby',
                'Anyone with a Bluetooth scanner can see that a phone is running VYRO LINK. They cannot see your name, contacts or messages.',
              ),
              point(
                'Your contacts',
                'They see the name you chose and your messages. In groups, members see each other\'s names.',
              ),
            ],
          ),
          const SectionHeader('How it works'),
          Section(
            rows: [
              point(
                'End-to-end encryption',
                'Each conversation uses the Olm double ratchet (vodozemac). Keys are made on this phone and stay here.',
              ),
              point(
                'Encrypted storage',
                'Messages and contacts are stored in SQLCipher databases whose keys live in the iOS Keychain or Android Keystore.',
              ),
              point(
                'Honest delivery',
                'Delivered appears only after the recipient\'s phone sends back an authenticated receipt. A relay accepting a copy is shown separately.',
              ),
            ],
          ),
          const SectionHeader('Limits'),
          Section(
            rows: [
              point(
                'Delivery is not guaranteed',
                'Messages move only when phones meet, and expire after about a week.',
              ),
              point(
                'Deleting cannot recall',
                'Delete for everyone asks the other phone to remove a message; it cannot undo a screenshot or a copy.',
              ),
              point(
                'Not yet independently reviewed',
                'The design and code have not had an external security audit yet. Do not rely on VYRO LINK where your safety depends on it.',
              ),
            ],
          ),
          const SectionHeader('Your identity key'),
          Live(
            load: (api) => api.myFingerprint(),
            builder: (context, fp) => Padding(
              padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
              child: Container(
                padding: const EdgeInsets.all(Space.l),
                decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
                child: SelectableText(fingerprintText(fp), style: monoStyle(context), textAlign: TextAlign.center),
              ),
            ),
          ),
          const SectionHeader('About'),
          Section(
            rows: [
              const ListTile(title: Text('Version'), trailing: Text('0.1.0 (development)')),
              const ListTile(title: Text('Protocol'), trailing: Text('Wire v1 · link v1')),
              ListTile(
                title: const Text('Open-source licences'),
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: () => showLicensePage(context: context, applicationName: 'VYRO LINK'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
