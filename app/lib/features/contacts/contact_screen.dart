import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/live.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';
import '../../design/theme.dart';
import '../../design/widgets/nearby.dart';
import '../pairing/pairing_screen.dart';
import '../settings/my_id_screen.dart';

/// Contact profile: message, verify, disappearing messages, block.
class ContactScreen extends StatelessWidget {
  const ContactScreen({super.key, required this.contactId});

  final Uint8List contactId;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(),
      body: Live<(Contact?, int)>(
        load: (api) async {
          final c = (await api.contacts()).where((x) => idKey(x.contactId) == idKey(contactId)).firstOrNull;
          return (c, c == null ? 0 : await api.disappearingTimer(contactId));
        },
        builder: (context, data) {
          final (contact, timer) = data;
          if (contact == null) {
            return const EmptyState(
              icon: Icons.person_off_outlined,
              title: 'Contact not found',
              body: 'This contact is no longer on this phone.',
            );
          }
          return _Body(contact: contact, timer: timer);
        },
      ),
    );
  }
}

class _Body extends StatelessWidget {
  const _Body({required this.contact, required this.timer});
  final Contact contact;
  final int timer;

  Future<void> _run(BuildContext context, Future<void> Function(VlHostApi) op, String done) async {
    try {
      await context.services.mutate(op);
      if (context.mounted) showMessage(context, done);
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
  }

  /// ADR-037: pick another contact; both are asked before anything is shared.
  Future<void> _introduce(BuildContext context) async {
    final s = context.services;
    final others = (await s.call((api) => api.contacts()))
        .where((x) => !x.blocked && idKey(x.contactId) != idKey(contact.contactId))
        .toList();
    if (!context.mounted) return;
    if (others.isEmpty) {
      showMessage(context, 'You need at least one other contact to introduce them to.');
      return;
    }
    final other = await showModalBottomSheet<Contact>(
      context: context,
      useSafeArea: true,
      builder: (sheet) => ListView(
        shrinkWrap: true,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
            child: Text('Introduce ${contact.displayName} to…', style: Theme.of(sheet).textTheme.headlineSmall),
          ),
          for (final o in others)
            ListTile(
              leading: Avatar(name: o.displayName, seed: o.contactId, size: 40),
              title: Text(o.displayName),
              onTap: () => Navigator.pop(sheet, o),
            ),
        ],
      ),
    );
    if (other == null || !context.mounted) return;
    await _run(
      context,
      (api) => api.introduce(contact.contactId, other.contactId),
      'Asked ${contact.displayName} and ${other.displayName}',
    );
  }

  Future<void> _rename(BuildContext context) async {
    final name = await askName(context, title: 'Rename contact', initial: contact.displayName);
    if (name == null || name.isEmpty || !context.mounted) return;
    await _run(context, (api) => api.renameContact(contact.contactId, name), 'Renamed');
  }

  Future<void> _delete(BuildContext context) async {
    final c = VlColors.of(context);
    final ok = await showAdaptiveDialog<bool>(
      context: context,
      builder: (d) => AlertDialog.adaptive(
        title: Text('Delete ${contact.displayName}?'),
        content: const Text(
          'The contact and your whole conversation are erased from this phone. You can only talk again after pairing in person.',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancel')),
          TextButton(
            onPressed: () => Navigator.pop(d, true),
            style: TextButton.styleFrom(foregroundColor: c.err),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;
    try {
      await context.services.mutate((api) => api.deleteContact(contact.contactId));
      if (context.mounted) Navigator.of(context).popUntil((r) => r.isFirst);
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
  }

  Future<void> _toggleBlock(BuildContext context) async {
    final c = VlColors.of(context);
    if (!contact.blocked) {
      final ok = await showAdaptiveDialog<bool>(
        context: context,
        builder: (d) => AlertDialog.adaptive(
          title: Text('Block ${contact.displayName}?'),
          content: const Text(
            'Their new messages are discarded on this phone and you cannot message them. They are not told. You can unblock at any time.',
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancel')),
            TextButton(
              onPressed: () => Navigator.pop(d, true),
              style: TextButton.styleFrom(foregroundColor: c.err),
              child: const Text('Block'),
            ),
          ],
        ),
      );
      if (ok != true || !context.mounted) return;
    }
    await _run(
      context,
      (api) => api.setBlocked(contact.contactId, !contact.blocked),
      contact.blocked ? 'Unblocked' : 'Blocked',
    );
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.only(bottom: Space.xxl),
      children: [
        const SizedBox(height: Space.s),
        Center(
          child: Avatar(name: contact.displayName, seed: contact.contactId, size: 88),
        ),
        const SizedBox(height: Space.l),
        Text(contact.displayName, style: t.headlineSmall, textAlign: TextAlign.center),
        const SizedBox(height: Space.xs),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            VerifiedBadge(verified: contact.verified),
            const SizedBox(width: Space.xs),
            Text(
              contact.verified ? 'Verified in person' : 'Not verified',
              style: t.bodySmall!.copyWith(color: contact.verified ? c.ok : c.text2),
            ),
          ],
        ),
        const SizedBox(height: Space.xs),
        if (contact.introducedBy != null)
          Padding(
            padding: const EdgeInsets.only(bottom: Space.xs),
            child: Text(
              'Introduced by ${contact.introducedBy}, not met in person',
              style: t.bodySmall!.copyWith(color: c.text2),
              textAlign: TextAlign.center,
            ),
          ),
        Center(
          child: NearbyTag(contactId: contact.contactId, detail: true, lastSeenSecs: contact.lastSeenSecs),
        ),
        const SizedBox(height: Space.xl),
        Padding(
          padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
          child: FilledButton.icon(
            onPressed: contact.blocked ? null : () => Routes.conversation(context, contact),
            icon: const Icon(Icons.chat_bubble_outline_rounded),
            label: const Text('Message'),
          ),
        ),
        const SectionHeader('Security'),
        Section(
          rows: [
            ListTile(
              leading: const Icon(Icons.pin_outlined),
              title: const Text('Safety number'),
              subtitle: Text(contact.verified ? 'Compared and marked verified' : 'Compare in person to verify'),
              trailing: const Icon(Icons.chevron_right_rounded),
              onTap: () => showModalBottomSheet<void>(
                context: context,
                isScrollControlled: true,
                useSafeArea: true,
                builder: (_) => VerifySheet(contact: contact),
              ),
            ),
          ],
          footer: 'If the numbers on both phones match, no one replaced your codes when you paired.',
        ),
        const SectionHeader('Their ID'),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: Space.l),
          child: Semantics(
            label: '${contact.displayName}\'s ID: ${contact.idCode.split('').join(' ')}',
            excludeSemantics: true,
            child: Container(
              padding: const EdgeInsets.all(Space.l),
              decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
              child: SelectableText(
                groupedIdCode(contact.idCode),
                textAlign: TextAlign.center,
                style: monoStyle(context).copyWith(letterSpacing: 1),
              ),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(Space.xl, Space.s, Space.xl, 0),
          child: Text(
            'It should match the ID on their My ID screen. Only their identity key can produce it.',
            style: t.bodySmall,
          ),
        ),
        const SectionHeader('Conversation'),
        Section(
          rows: [
            ListTile(
              leading: const Icon(Icons.timer_outlined),
              title: const Text('Disappearing messages'),
              trailing: Text(timerLabel(timer), style: t.bodyMedium!.copyWith(color: c.text2)),
              onTap: () async {
                final v = await showModalBottomSheet<int>(
                  context: context,
                  useSafeArea: true,
                  builder: (sheet) => RadioGroup<int>(
                    groupValue: timer,
                    onChanged: (v) => Navigator.pop(sheet, v),
                    child: ListView(
                      shrinkWrap: true,
                      children: [
                        for (final e in timerChoices.entries) RadioListTile<int>(value: e.key, title: Text(e.value)),
                      ],
                    ),
                  ),
                );
                if (v != null && context.mounted) {
                  await _run(
                    context,
                    (api) => api.setDisappearingTimer(contact.contactId, v),
                    'Disappearing messages: ${timerLabel(v)}',
                  );
                }
              },
            ),
            ListTile(
              leading: const Icon(Icons.search_rounded),
              title: const Text('Search in chat'),
              onTap: () => Routes.search(context, contactId: contact.contactId),
            ),
          ],
        ),
        const SectionHeader('Contact'),
        Section(
          rows: [
            ListTile(
              leading: const Icon(Icons.edit_outlined),
              title: const Text('Rename'),
              subtitle: const Text('Only on this phone'),
              onTap: () => _rename(context),
            ),
            ListTile(
              leading: const Icon(Icons.qr_code_scanner_rounded),
              title: const Text('Pair again'),
              subtitle: const Text('Refreshes the keys in person; messages are kept'),
              onTap: () => Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => const PairingScreen())),
            ),
            if (!contact.blocked)
              ListTile(
                leading: const Icon(Icons.handshake_outlined),
                title: const Text('Introduce to someone'),
                subtitle: const Text('Both are asked first; they start as not verified'),
                onTap: () => _introduce(context),
              ),
          ],
        ),
        const SectionHeader('Privacy'),
        Section(
          rows: [
            ListTile(
              leading: Icon(
                contact.blocked ? Icons.check_circle_outline_rounded : Icons.block_rounded,
                color: contact.blocked ? null : c.err,
              ),
              title: Text(
                contact.blocked ? 'Unblock ${contact.displayName}' : 'Block ${contact.displayName}',
                style: contact.blocked ? null : TextStyle(color: c.err),
              ),
              onTap: () => _toggleBlock(context),
            ),
            ListTile(
              leading: Icon(Icons.person_remove_outlined, color: c.err),
              title: Text('Delete contact', style: TextStyle(color: c.err)),
              onTap: () => _delete(context),
            ),
          ],
          footer: 'Contacts are stored only on this phone, encrypted. Nothing is uploaded.',
        ),
      ],
    );
  }
}
