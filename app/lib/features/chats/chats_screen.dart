import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/live.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/delivery.dart';
import '../../design/widgets/nearby.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';
import '../conversation/conversation_screen.dart';
import '../groups/group_screen.dart';
import '../home/home_shell.dart';

/// One row of the chat list: a contact conversation or a group.
class ChatEntry {
  ChatEntry.direct(ConversationSummary this.direct) : group = null, groupLast = null;
  ChatEntry.group(Group this.group, this.groupLast) : direct = null;

  final ConversationSummary? direct;
  final Group? group;
  final GroupMessage? groupLast;

  int? get lastAt => direct?.last?.createdAtSecs ?? groupLast?.createdAtSecs;
}

Future<List<ChatEntry>> loadChats(VlHostApi api) async {
  final entries = [for (final d in await api.conversationSummaries()) ChatEntry.direct(d)];
  for (final g in await api.groups()) {
    final last = await api.groupMessages(g.groupId, 1);
    entries.add(ChatEntry.group(g, last.isEmpty ? null : last.first));
  }
  entries.sort((a, b) => (b.lastAt ?? -1).compareTo(a.lastAt ?? -1));
  return entries;
}

class ChatsScreen extends StatefulWidget {
  const ChatsScreen({super.key});

  @override
  State<ChatsScreen> createState() => _ChatsScreenState();
}

/// On wide screens (≥ 840 dp) the list and the open chat sit side by side
/// (Doc 16 §11); on phones a chat opens as its own page.
class _ChatsScreenState extends State<ChatsScreen> {
  ChatEntry? _selected;

  void _open(ChatEntry e) {
    final wide = MediaQuery.sizeOf(context).width >= 840;
    if (wide) {
      setState(() => _selected = e);
    } else if (e.direct != null) {
      Routes.conversation(context, e.direct!.contact);
    } else {
      Routes.group(context, e.group!);
    }
  }

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= 840;
    final list = _list(context);
    if (!wide) return list;
    final sel = _selected;
    return Row(
      children: [
        SizedBox(width: 380, child: list),
        const VerticalDivider(width: 1),
        Expanded(
          child: sel == null
              ? const EmptyState(
                  icon: Icons.chat_bubble_outline_rounded,
                  title: 'Choose a conversation',
                  body: 'Messages open here.',
                )
              : sel.direct != null
              ? ConversationScreen(
                  key: ValueKey(idKey(sel.direct!.contact.contactId)),
                  contactId: sel.direct!.contact.contactId,
                  initialName: sel.direct!.contact.displayName,
                )
              : GroupScreen(
                  key: ValueKey(idKey(sel.group!.groupId)),
                  groupId: sel.group!.groupId,
                  initialName: sel.group!.name,
                ),
        ),
      ],
    );
  }

  Widget _list(BuildContext context) {
    return Scaffold(
      floatingActionButton: FloatingActionButton(
        heroTag: null,
        tooltip: 'New message',
        onPressed: () => showNewChatSheet(context),
        child: const Icon(Icons.edit_rounded),
      ),
      body: CustomScrollView(
        slivers: [
          SliverAppBar.large(
            title: const Text('Chats'),
            actions: [
              const NetworkChip(),
              IconButton(
                tooltip: 'Search messages',
                icon: const Icon(Icons.search_rounded),
                onPressed: () => Routes.search(context),
              ),
              const SizedBox(width: Space.xs),
            ],
          ),
          const SliverToBoxAdapter(child: RelayBanner()),
          Live<List<ChatEntry>>(
            sliver: true,
            load: loadChats,
            builder: (context, entries) {
              if (entries.isEmpty) {
                return SliverFillRemaining(
                  hasScrollBody: false,
                  child: EmptyState(
                    icon: Icons.forum_outlined,
                    title: 'No conversations yet',
                    body: 'Add a contact in person to start. You will each scan the other\'s code.',
                    action: FilledButton.icon(
                      onPressed: () => Routes.pairing(context),
                      icon: const Icon(Icons.qr_code_scanner_rounded),
                      label: const Text('Add a contact'),
                    ),
                  ),
                );
              }
              return SliverList.builder(
                itemCount: entries.length,
                itemBuilder: (context, i) => _ChatRow(
                  entry: entries[i],
                  selected: _selected != null && _keyOf(_selected!) == _keyOf(entries[i]),
                  onTap: () => _open(entries[i]),
                ),
              );
            },
          ),
          const SliverPadding(padding: EdgeInsets.only(bottom: 96)),
        ],
      ),
    );
  }
}

String _keyOf(ChatEntry e) => idKey(e.direct?.contact.contactId ?? e.group!.groupId);

class _ChatRow extends StatelessWidget {
  const _ChatRow({required this.entry, required this.onTap, this.selected = false});
  final ChatEntry entry;
  final VoidCallback onTap;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<Map<String, int>>(
      valueListenable: context.services.nearby,
      builder: (context, nearby, _) =>
          _row(context, entry.direct != null && nearby.containsKey(idKey(entry.direct!.contact.contactId))),
    );
  }

  Widget _row(BuildContext context, bool near) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final d = entry.direct;
    final g = entry.group;
    final name = d?.contact.displayName ?? g!.name;
    final seed = d?.contact.contactId ?? g!.groupId;
    final unread = d?.unread ?? 0;

    String preview;
    DeliveryState? glyph;
    var muted = false;
    if (d != null) {
      final m = d.last;
      if (m == null) {
        preview = 'Paired. Say hello.';
        muted = true;
      } else if (m.deleted) {
        preview = 'Message deleted';
        muted = true;
      } else {
        preview = m.body;
      }
      if (m != null && m.outgoing && !m.deleted) glyph = m.state;
    } else {
      final m = entry.groupLast;
      if (m == null) {
        preview = g!.left ? 'You left this group' : 'Group created';
        muted = true;
      } else {
        final author = m.outgoing
            ? 'You'
            : g!.members.where((x) => idKey(x.fingerprint) == idKey(m.authorFingerprint)).firstOrNull?.name ??
                  'Someone';
        preview = '$author: ${m.body}';
      }
    }

    final label = [
      name,
      if (d?.contact.blocked ?? false) 'blocked',
      if (near) 'nearby',
      if (unread > 0) '$unread unread',
      preview,
      if (glyph != null) DeliveryInfo.of(glyph).label,
      if (entry.lastAt != null) shortStamp(context, entry.lastAt!),
    ].join(', ');

    return Semantics(
      button: true,
      label: label,
      excludeSemantics: true,
      selected: selected,
      child: InkWell(
        onTap: onTap,
        child: Container(
          color: selected ? VlColors.of(context).accent.withValues(alpha: 0.08) : null,
          padding: EdgeInsets.symmetric(horizontal: Space.gutter(context), vertical: Space.m),
          child: Row(
            children: [
              Avatar(name: name, seed: seed, size: 52, group: g != null),
              const SizedBox(width: Space.m),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(name, style: t.titleMedium, maxLines: 1, overflow: TextOverflow.ellipsis),
                        ),
                        if (d?.contact.verified ?? false) ...[
                          const SizedBox(width: Space.xs),
                          const VerifiedBadge(verified: true, size: 15),
                        ],
                        if (d?.contact.blocked ?? false) ...[
                          const SizedBox(width: Space.xs),
                          Icon(Icons.block_rounded, size: 15, color: c.text2),
                        ],
                        if (near) ...[const SizedBox(width: Space.s), NearbyTag(contactId: d!.contact.contactId)],
                        const Spacer(),
                        if (entry.lastAt != null)
                          Text(
                            shortStamp(context, entry.lastAt!),
                            style: t.bodySmall!.copyWith(color: unread > 0 ? c.accent : c.text2),
                          ),
                      ],
                    ),
                    const SizedBox(height: 2),
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        if (glyph != null) ...[
                          Padding(
                            padding: const EdgeInsets.only(top: 3),
                            child: DeliveryGlyph(state: glyph, size: 15),
                          ),
                          const SizedBox(width: Space.xs),
                        ],
                        Expanded(
                          child: Text(
                            preview,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: t.bodyMedium!.copyWith(color: c.text2, fontStyle: muted ? FontStyle.italic : null),
                          ),
                        ),
                        if (unread > 0) ...[
                          const SizedBox(width: Space.s),
                          Container(
                            constraints: const BoxConstraints(minWidth: 22),
                            height: 22,
                            padding: const EdgeInsets.symmetric(horizontal: 7),
                            alignment: Alignment.center,
                            decoration: BoxDecoration(color: c.accent, borderRadius: BorderRadius.circular(11)),
                            child: Text(
                              unread > 99 ? '99+' : '$unread',
                              style: t.labelSmall!.copyWith(color: c.onAccent, fontSize: 12),
                            ),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Explains why messages may not be moving (Doc 16 §9). No internet banner,
/// ever: the app is offline by design.
class RelayBanner extends StatelessWidget {
  const RelayBanner({super.key});

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    return ValueListenableBuilder<NetworkStatus?>(
      valueListenable: s.network,
      builder: (context, n, _) {
        // The web chat has no Bluetooth; it only uses the internet mailbox.
        if (n == null || kIsWeb) return const SizedBox.shrink();
        if (!n.relayEnabled) {
          return InfoBanner(
            kind: BannerKind.info,
            icon: Icons.bluetooth_disabled_rounded,
            text: 'Bluetooth relay is off. New messages wait on this phone.',
            action: 'Turn on',
            onAction: () async {
              await s.call((api) => api.setRelayEnabled(true));
              await s.refreshNetwork();
            },
          );
        }
        return switch (n.radio) {
          RadioState.poweredOff => const InfoBanner(
            kind: BannerKind.err,
            icon: Icons.bluetooth_disabled_rounded,
            text: 'Bluetooth is off. Turn it on in Control Center or Settings so messages can move.',
          ),
          RadioState.unauthorized => InfoBanner(
            kind: BannerKind.err,
            text: 'VYRO LINK needs Bluetooth permission to pass messages.',
            action: 'Fix',
            onAction: () => s.call((api) => api.openAppSettings()),
          ),
          _ => const SizedBox.shrink(),
        };
      },
    );
  }
}

/// Start a chat with a contact, add a contact, or create a group.
Future<void> showNewChatSheet(BuildContext context) => showModalBottomSheet<void>(
  context: context,
  isScrollControlled: true,
  useSafeArea: true,
  builder: (sheet) => DraggableScrollableSheet(
    expand: false,
    initialChildSize: 0.6,
    maxChildSize: 0.92,
    builder: (context, scroll) => Live<List<Contact>>(
      load: (api) => api.contacts(),
      builder: (context, contacts) {
        final usable = contacts.where((x) => !x.blocked).toList()
          ..sort((a, b) => a.displayName.toLowerCase().compareTo(b.displayName.toLowerCase()));
        return ListView(
          controller: scroll,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
              child: Text('New message', style: Theme.of(context).textTheme.headlineSmall),
            ),
            ListTile(
              leading: const Icon(Icons.qr_code_scanner_rounded),
              title: const Text('Add a contact in person'),
              onTap: () {
                Navigator.pop(sheet);
                Routes.pairing(context);
              },
            ),
            ListTile(
              leading: const Icon(Icons.group_add_outlined),
              title: const Text('New group'),
              subtitle: const Text('Up to 8 people, including you'),
              enabled: usable.isNotEmpty,
              onTap: () {
                Navigator.pop(sheet);
                Routes.newGroup(context);
              },
            ),
            if (usable.isNotEmpty) const SectionHeader('Contacts'),
            for (final contact in usable)
              ListTile(
                leading: Avatar(name: contact.displayName, seed: contact.contactId, size: 40),
                title: Text(contact.displayName),
                trailing: contact.verified ? const VerifiedBadge(verified: true) : null,
                onTap: () {
                  Navigator.pop(sheet);
                  Routes.conversation(context, contact);
                },
              ),
          ],
        );
      },
    ),
  ),
);
