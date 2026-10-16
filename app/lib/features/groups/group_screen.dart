import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';
import '../chats/chats_screen.dart';
import '../conversation/composer.dart';
import '../conversation/message_list.dart';
import 'new_group_screen.dart';

/// A small group (ADR-024): pairwise copies to each member; delivery is shown
/// as how many members' phones confirmed with authenticated receipts.
class GroupScreen extends StatefulWidget {
  const GroupScreen({super.key, required this.groupId, required this.initialName});

  final Uint8List groupId;
  final String initialName;

  @override
  State<GroupScreen> createState() => _GroupScreenState();
}

class _GroupScreenState extends State<GroupScreen> {
  static const _page = 60;

  final _text = TextEditingController();
  final _focus = FocusNode();
  final _scroll = ScrollController();
  late final AppServices _s = context.services;

  Group? _group;
  List<GroupMessage> _messages = const [];
  Uint8List? _me;
  GroupDelivery? _delivery;
  GroupMessage? _replyTo;
  int _limit = _page;
  bool _loaded = false;
  VlFailure? _error;

  @override
  void initState() {
    super.initState();
    _s.changes.addListener(_load);
    _load();
  }

  @override
  void dispose() {
    _s.changes.removeListener(_load);
    _text.dispose();
    _focus.dispose();
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      _me ??= await _s.call((api) => api.myFingerprint());
      final group = (await _s.call((api) => api.groups()))
          .where((g) => idKey(g.groupId) == idKey(widget.groupId))
          .firstOrNull;
      final messages = group == null
          ? <GroupMessage>[]
          : await _s.call((api) => api.groupMessages(widget.groupId, _limit));
      final newestMine = messages.where((m) => m.outgoing).firstOrNull;
      final delivery = newestMine == null ? null : await _s.call((api) => api.groupDelivery(newestMine.groupMsgId));
      if (!mounted) return;
      setState(() {
        _group = group;
        _messages = messages;
        _delivery = delivery;
        _loaded = true;
        _error = null;
      });
    } on VlFailure catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  String _author(Uint8List fp) =>
      _group?.members.where((m) => idKey(m.fingerprint) == idKey(fp)).firstOrNull?.name ?? 'Former member';

  Future<bool> _send(String text) async {
    try {
      await _s.mutate((api) => api.sendGroupMessage(widget.groupId, text, _replyTo?.groupMsgId));
      setState(() => _replyTo = null);
      if (_scroll.hasClients) _scroll.jumpTo(0);
      return true;
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
      return false;
    }
  }

  void _actions(BubbleData b) {
    final m = _messages.where((x) => idKey(x.groupMsgId) == b.key).firstOrNull;
    if (m == null) return;
    HapticFeedback.selectionClick();
    showModalBottomSheet<void>(
      context: context,
      useSafeArea: true,
      builder: (sheet) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (!(_group?.left ?? true))
              ListTile(
                leading: const Icon(Icons.reply_rounded),
                title: const Text('Reply'),
                onTap: () {
                  Navigator.pop(sheet);
                  setState(() => _replyTo = m);
                  _focus.requestFocus();
                },
              ),
            ListTile(
              leading: const Icon(Icons.copy_rounded),
              title: const Text('Copy'),
              onTap: () {
                Navigator.pop(sheet);
                Clipboard.setData(ClipboardData(text: m.body));
                showMessage(context, 'Copied');
              },
            ),
          ],
        ),
      ),
    );
  }

  void _info() => showModalBottomSheet<void>(
    context: context,
    useSafeArea: true,
    isScrollControlled: true,
    builder: (sheet) => _GroupInfo(group: _group!, me: _me!),
  );

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final group = _group;
    final name = group?.name ?? widget.initialName;
    final d = _delivery;
    final status = d == null
        ? null
        : d.sent == 0
        ? null
        : d.delivered == d.sent
        ? 'Delivered to everyone'
        : 'Delivered to ${d.delivered} of ${d.sent}';
    final items = [
      for (final m in _messages)
        BubbleData(
          id: m.groupMsgId,
          outgoing: m.outgoing,
          body: m.body,
          createdAtSecs: m.createdAtSecs,
          author: m.outgoing ? null : _author(m.authorFingerprint),
          status: m.outgoing ? status : null,
          replyTo: m.replyTo,
        ),
    ];
    return Scaffold(
      appBar: AppBar(
        titleSpacing: 0,
        centerTitle: false,
        title: InkWell(
          borderRadius: BorderRadius.circular(Radii.card),
          onTap: group == null ? null : _info,
          child: Padding(
            padding: const EdgeInsets.all(Space.xs),
            child: Row(
              children: [
                Avatar(name: name, seed: widget.groupId, size: 36, group: true),
                const SizedBox(width: Space.m),
                Flexible(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(name, style: t.titleMedium, maxLines: 1, overflow: TextOverflow.ellipsis),
                      if (group != null)
                        Text(
                          group.members
                              .map((m) => idKey(m.fingerprint) == idKey(_me ?? Uint8List(0)) ? 'You' : m.name)
                              .join(', '),
                          style: t.bodySmall,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
        actions: [
          if (group != null)
            IconButton(tooltip: 'Group details', onPressed: _info, icon: const Icon(Icons.info_outline_rounded)),
        ],
      ),
      body: Column(
        children: [
          const RelayBanner(),
          Expanded(
            child: _error != null && !_loaded
                ? ErrorState(failure: _error!, onRetry: _load)
                : !_loaded
                ? const SizedBox.shrink()
                : group == null
                ? const EmptyState(
                    icon: Icons.group_off_outlined,
                    title: 'Group not found',
                    body: 'This group is no longer on this phone.',
                  )
                : MessageList(
                    items: items,
                    controller: _scroll,
                    reactionSet: const [],
                    onLongPress: _actions,
                    onNearTop: () {
                      if (_messages.length >= _limit) {
                        _limit += _page;
                        _load();
                      }
                    },
                    header: _messages.length < _limit
                        ? Padding(
                            padding: const EdgeInsets.all(Space.l),
                            child: Text(
                              'Each message is encrypted separately for every member and travels phone to phone.',
                              style: t.bodySmall,
                              textAlign: TextAlign.center,
                            ),
                          )
                        : null,
                  ),
          ),
          if (group != null && group.left)
            Material(
              color: c.surface,
              child: SafeArea(
                top: false,
                child: Padding(
                  padding: const EdgeInsets.all(Space.l),
                  child: Text(
                    'You are no longer a member of this group.',
                    style: t.bodyMedium,
                    textAlign: TextAlign.center,
                  ),
                ),
              ),
            )
          else if (group != null)
            Composer(
              controller: _text,
              focusNode: _focus,
              maxBytes: _s.startup.maxTextBytes,
              onSend: _send,
              hint: 'Message $name',
              context: _replyTo == null
                  ? null
                  : ComposerContext(
                      icon: Icons.reply_rounded,
                      title: 'Replying to ${_replyTo!.outgoing ? 'yourself' : _author(_replyTo!.authorFingerprint)}',
                      text: _replyTo!.body,
                    ),
              onCancelContext: () => setState(() => _replyTo = null),
            ),
        ],
      ),
    );
  }
}

class _GroupInfo extends StatelessWidget {
  const _GroupInfo({required this.group, required this.me});
  final Group group;
  final Uint8List me;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final creator = idKey(group.creatorFingerprint) == idKey(me);
    return ListView(
      shrinkWrap: true,
      padding: EdgeInsets.only(bottom: MediaQuery.paddingOf(context).bottom + Space.l),
      children: [
        Center(
          child: Avatar(name: group.name, seed: group.groupId, size: 64, group: true),
        ),
        const SizedBox(height: Space.m),
        Text(group.name, style: t.headlineSmall, textAlign: TextAlign.center),
        SectionHeader('${group.members.length} members'),
        for (final m in group.members)
          ListTile(
            leading: Avatar(name: m.name, seed: m.fingerprint, size: 40),
            title: Text(idKey(m.fingerprint) == idKey(me) ? 'You' : m.name),
            trailing: idKey(m.fingerprint) == idKey(group.creatorFingerprint)
                ? Text('Creator', style: t.bodySmall)
                : null,
          ),
        if (!group.left) ...[
          if (creator)
            ListTile(
              leading: const Icon(Icons.group_outlined),
              title: const Text('Change members'),
              onTap: () {
                Navigator.pop(context);
                Navigator.of(context).push(MaterialPageRoute<void>(builder: (_) => NewGroupScreen(group: group)));
              },
            ),
          ListTile(
            leading: Icon(Icons.logout_rounded, color: c.err),
            title: Text('Leave group', style: TextStyle(color: c.err)),
            onTap: () async {
              final ok = await showAdaptiveDialog<bool>(
                context: context,
                builder: (d) => AlertDialog.adaptive(
                  title: const Text('Leave this group?'),
                  content: const Text(
                    'Members are told when your message reaches them. You keep the messages on this phone.',
                  ),
                  actions: [
                    TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancel')),
                    TextButton(
                      onPressed: () => Navigator.pop(d, true),
                      style: TextButton.styleFrom(foregroundColor: c.err),
                      child: const Text('Leave'),
                    ),
                  ],
                ),
              );
              if (ok == true && context.mounted) {
                try {
                  await context.services.mutate((api) => api.leaveGroup(group.groupId));
                  if (context.mounted) Navigator.pop(context);
                } on VlFailure catch (e) {
                  if (context.mounted) showMessage(context, e.message);
                }
              }
            },
          ),
        ],
        Padding(
          padding: const EdgeInsets.all(Space.xl),
          child: Text(
            'Only the creator can change members. A member could send different text to different people; '
            'VYRO LINK cannot detect that in groups (same as other pairwise group designs).',
            style: t.bodySmall,
          ),
        ),
      ],
    );
  }
}
