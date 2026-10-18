import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';
import '../chats/chats_screen.dart';
import 'attachment_view.dart';
import 'paper_screen.dart';
import 'share_view.dart';
import 'composer.dart';
import 'message_list.dart';

/// Doc 16 screen 7: one-to-one conversation.
class ConversationScreen extends StatefulWidget {
  const ConversationScreen({super.key, required this.contactId, required this.initialName});

  final Uint8List contactId;
  final String initialName;

  @override
  State<ConversationScreen> createState() => _ConversationScreenState();
}

class _ConversationScreenState extends State<ConversationScreen> with WidgetsBindingObserver {
  static const _page = 60;
  static const editWindowSecs = 24 * 3600;
  static const deleteWindowSecs = 72 * 3600;

  final _text = TextEditingController();
  final _focus = FocusNode();
  final _scroll = ScrollController();
  late final AppServices _s = context.services;

  Contact? _contact;
  List<Message> _messages = const [];
  Map<String, List<Reaction>> _reactions = const {};
  int _timer = 0;
  int _limit = _page;
  bool _loaded = false;
  bool _loadingMore = false;
  VlFailure? _error;
  Message? _replyTo;
  Message? _editing;
  Timer? _draftTimer;
  bool _showJump = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _s.changes.addListener(_load);
    _scroll.addListener(() {
      final show = _scroll.hasClients && _scroll.offset > 400;
      if (show != _showJump) setState(() => _showJump = show);
    });
    _text.addListener(_scheduleDraft);
    _restoreDraft();
    _load();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _s.changes.removeListener(_load);
    _draftTimer?.cancel();
    if (_editing == null) _saveDraft();
    _text.dispose();
    _focus.dispose();
    _scroll.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _load();
  }

  Future<void> _restoreDraft() async {
    try {
      final d = await _s.call((api) => api.draft(widget.contactId));
      if (d != null && _text.text.isEmpty && mounted) _text.text = d;
    } on VlFailure {
      // A missing draft is not worth an error.
    }
  }

  void _scheduleDraft() {
    if (_editing != null) return;
    _draftTimer?.cancel();
    _draftTimer = Timer(const Duration(milliseconds: 700), _saveDraft);
  }

  void _saveDraft() {
    _s.call((api) => api.saveDraft(widget.contactId, _text.text)).ignore();
  }

  Future<void> _load() async {
    try {
      final contacts = await _s.call((api) => api.contacts());
      final contact = contacts.where((c) => idKey(c.contactId) == idKey(widget.contactId)).firstOrNull;
      final messages = await _s.call((api) => api.conversation(widget.contactId, _limit));
      final reactions = <String, List<Reaction>>{};
      await Future.wait([
        for (final m in messages.where((m) => !m.deleted))
          _s.call((api) => api.reactions(m.messageId)).then((r) {
            if (r.isNotEmpty) reactions[idKey(m.messageId)] = r;
          }),
      ]);
      final timer = contact == null ? 0 : await _s.call((api) => api.disappearingTimer(widget.contactId));
      if (!mounted) return;
      setState(() {
        _contact = contact;
        _messages = messages;
        _reactions = reactions;
        _timer = timer;
        _loaded = true;
        _error = null;
        _loadingMore = false;
      });
      final visible = ModalRoute.of(context)?.isCurrent ?? true;
      final foreground = WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed;
      if (contact != null && visible && foreground && messages.any((m) => !m.outgoing)) {
        final n = await _s.call((api) => api.markRead(widget.contactId));
        if (n > 0) _s.bump();
      }
    } on VlFailure catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  void _loadMore() {
    if (_loadingMore || _messages.length < _limit) return;
    _loadingMore = true;
    _limit += _page;
    _load();
  }

  Future<bool> _send(String text) async {
    try {
      final editing = _editing;
      if (editing != null) {
        await _s.mutate((api) => api.editMessage(widget.contactId, editing.messageId, text));
      } else {
        await _s.mutate((api) => api.sendMessage(widget.contactId, text, _replyTo?.messageId, false));
        _draftTimer?.cancel();
        _s.call((api) => api.saveDraft(widget.contactId, '')).ignore();
      }
      setState(() {
        _replyTo = null;
        _editing = null;
      });
      if (_scroll.hasClients) _scroll.jumpTo(0);
      return true;
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
      return false;
    }
  }

  Future<void> _attach() async {
    final choice = await chooseShare(context);
    if (choice == null || !mounted) return;
    if (choice == shareLocation) return _shareLocation();
    if (choice == sharePoll) return _sendPoll();
    final picked = await pickAttachment(context, choice);
    if (picked == null) return;
    final (file, kind, caption) = picked;
    try {
      await _s.mutate(
        (api) => api.sendAttachment(widget.contactId, kind, file.name, file.data!, caption, _replyTo?.messageId),
      );
      HapticFeedback.lightImpact();
      if (!mounted) return;
      setState(() => _replyTo = null);
      if (_scroll.hasClients) _scroll.jumpTo(0);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
  }

  /// ADR-034: one fix, only after the person confirms whom it goes to.
  Future<void> _shareLocation() async {
    final name = _contact?.displayName ?? widget.initialName;
    final ok = await _confirmShare(name);
    if (!ok || !mounted) return;
    try {
      final fix = await _s.call((api) => api.currentLocation());
      if (!mounted) return;
      if (fix == null) {
        showMessage(context, 'Location is off or not allowed for VYRO LINK. Turn it on in Settings to share it.');
        return;
      }
      await _s.mutate((api) => api.sendLocation(widget.contactId, fix));
      HapticFeedback.lightImpact();
      if (mounted && _scroll.hasClients) _scroll.jumpTo(0);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
  }

  Future<bool> _confirmShare(String name) async =>
      await showAdaptiveDialog<bool>(
        context: context,
        builder: (c) => AlertDialog.adaptive(
          title: Text('Share your location with $name?'),
          content: const Text(
            'Your current position is sent once, end-to-end encrypted. Phones that carry it cannot read it. '
            'VYRO LINK does not track you or share it again.',
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
            TextButton(onPressed: () => Navigator.pop(c, true), child: const Text('Share')),
          ],
        ),
      ) ??
      false;

  Future<void> _sendPoll() async {
    final p = await composePoll(context);
    if (p == null || !mounted) return;
    await _run((api) => api.sendPoll(widget.contactId, p.question, p.options, p.multi));
    if (mounted && _scroll.hasClients) _scroll.jumpTo(0);
  }

  Message? _find(BubbleData b) => _messages.where((m) => idKey(m.messageId) == b.key).firstOrNull;

  Future<void> _run(Future<void> Function(VlHostApi api) op, [String? done]) async {
    try {
      await _s.mutate(op);
      if (done != null && mounted) showMessage(context, done);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
  }

  void _actions(BubbleData b) {
    final m = _find(b);
    if (m == null) return;
    HapticFeedback.selectionClick();
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final age = now - m.createdAtSecs;
    final mine = _reactions[b.key]?.where((r) => r.fromMe).firstOrNull?.reaction ?? 0;
    final set = _s.startup.reactions;
    final name = _contact?.displayName ?? widget.initialName;
    showModalBottomSheet<void>(
      context: context,
      useSafeArea: true,
      isScrollControlled: true,
      builder: (sheet) {
        void close() => Navigator.pop(sheet);
        return SingleChildScrollView(
          padding: EdgeInsets.only(bottom: MediaQuery.paddingOf(sheet).bottom + Space.s),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (!m.deleted && set.isNotEmpty)
                LayoutBuilder(
                  // One row of 48 dp targets; scrolls sideways only on very narrow screens.
                  builder: (context, box) => SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    padding: const EdgeInsets.symmetric(horizontal: Space.s),
                    child: ConstrainedBox(
                      constraints: BoxConstraints(minWidth: box.maxWidth - Space.l),
                      child: Row(
                        mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                        children: [
                          for (var i = 0; i < set.length; i++)
                            _ReactionButton(
                              emoji: set[i],
                              selected: mine == i + 1,
                              onTap: () {
                                close();
                                _run((api) => api.react(widget.contactId, m.messageId, mine == i + 1 ? 0 : i + 1));
                              },
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              if (!m.deleted) ...[
                ListTile(
                  leading: const Icon(Icons.reply_rounded),
                  title: const Text('Reply'),
                  onTap: () {
                    close();
                    setState(() {
                      _editing = null;
                      _replyTo = m;
                    });
                    _focus.requestFocus();
                  },
                ),
                if (m.body.isNotEmpty)
                  ListTile(
                    leading: const Icon(Icons.copy_rounded),
                    title: const Text('Copy'),
                    onTap: () {
                      close();
                      Clipboard.setData(ClipboardData(text: m.body));
                      showMessage(context, 'Copied');
                    },
                  ),
                if (m.outgoing && age < editWindowSecs && m.attachment == null)
                  ListTile(
                    leading: const Icon(Icons.edit_rounded),
                    title: const Text('Edit'),
                    onTap: () {
                      close();
                      setState(() {
                        _replyTo = null;
                        _editing = m;
                      });
                      _text.text = m.body;
                      _focus.requestFocus();
                    },
                  ),
                if (m.attachment == null)
                  ListTile(
                    leading: const Icon(Icons.shortcut_rounded),
                    title: const Text('Forward'),
                    onTap: () {
                      close();
                      _forward(m);
                    },
                  ),
                ListTile(
                  leading: Icon(m.pinned ? Icons.push_pin_outlined : Icons.push_pin_rounded),
                  title: Text(m.pinned ? 'Unpin' : 'Pin'),
                  onTap: () {
                    close();
                    _run((api) => api.setPinned(m.messageId, !m.pinned));
                  },
                ),
                ListTile(
                  leading: Icon(m.starred ? Icons.star_outline_rounded : Icons.star_rounded),
                  title: Text(m.starred ? 'Remove star' : 'Star'),
                  onTap: () {
                    close();
                    _run((api) => api.setStarred(m.messageId, !m.starred));
                  },
                ),
              ],
              if (m.outgoing && !m.deleted && m.state != DeliveryState.delivered && m.attachment == null)
                ListTile(
                  leading: const Icon(Icons.qr_code_2_rounded),
                  title: const Text('Show as QR code'),
                  subtitle: const Text('For when Bluetooth can\'t reach them'),
                  onTap: () {
                    close();
                    Navigator.of(context).push(
                      MaterialPageRoute<void>(
                        builder: (_) => PaperScreen(messageId: m.messageId, recipient: name),
                      ),
                    );
                  },
                ),
              if (m.outgoing)
                ListTile(
                  leading: const Icon(Icons.info_outline_rounded),
                  title: const Text('Delivery details'),
                  onTap: () {
                    close();
                    Routes.deliveryDetails(context, m.messageId);
                  },
                ),
              ListTile(
                leading: const Icon(Icons.delete_outline_rounded),
                title: const Text('Delete for me'),
                onTap: () {
                  close();
                  _run((api) => api.deleteForMe(m.messageId), 'Deleted from this phone');
                },
              ),
              if (m.outgoing && !m.deleted && age < deleteWindowSecs)
                ListTile(
                  leading: Icon(Icons.delete_forever_outlined, color: VlColors.of(context).err),
                  title: Text('Delete for everyone', style: TextStyle(color: VlColors.of(context).err)),
                  onTap: () async {
                    close();
                    final ok = await _confirm(
                      'Delete for everyone?',
                      'VYRO LINK asks $name\'s phone to delete it when the request arrives. If they have already read or copied it, it cannot be recalled.',
                      'Delete',
                    );
                    if (ok) _run((api) => api.deleteForEveryone(widget.contactId, m.messageId));
                  },
                ),
            ],
          ),
        );
      },
    );
  }

  Future<bool> _confirm(String title, String body, String action) async =>
      await showAdaptiveDialog<bool>(
        context: context,
        builder: (c) => AlertDialog.adaptive(
          title: Text(title),
          content: Text(body),
          actions: [
            TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
            TextButton(
              onPressed: () => Navigator.pop(c, true),
              style: TextButton.styleFrom(foregroundColor: VlColors.of(c).err),
              child: Text(action),
            ),
          ],
        ),
      ) ??
      false;

  Future<void> _forward(Message m) async {
    final contacts = (await _s.call((api) => api.contacts())).where((c) => !c.blocked).toList();
    if (!mounted) return;
    final target = await showModalBottomSheet<Contact>(
      context: context,
      useSafeArea: true,
      builder: (sheet) => ListView(
        shrinkWrap: true,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
            child: Text('Forward to', style: Theme.of(sheet).textTheme.headlineSmall),
          ),
          for (final c in contacts)
            ListTile(
              leading: Avatar(name: c.displayName, seed: c.contactId, size: 40),
              title: Text(c.displayName),
              onTap: () => Navigator.pop(sheet, c),
            ),
        ],
      ),
    );
    if (target == null) return;
    await _run((api) => api.sendMessage(target.contactId, m.body, null, true), 'Forwarded to ${target.displayName}');
  }

  void _chooseTimer() => showModalBottomSheet<void>(
    context: context,
    useSafeArea: true,
    builder: (sheet) => RadioGroup<int>(
      groupValue: _timer,
      onChanged: (v) {
        Navigator.pop(sheet);
        if (v != null) _run((api) => api.setDisappearingTimer(widget.contactId, v));
      },
      child: ListView(
        shrinkWrap: true,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
            child: Text('Disappearing messages', style: Theme.of(sheet).textTheme.headlineSmall),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
            child: Text(
              'New messages are removed from both phones this long after they are read. Someone can still copy or photograph a message before then.',
              style: Theme.of(sheet).textTheme.bodySmall,
            ),
          ),
          for (final e in timerChoices.entries) RadioListTile<int>(value: e.key, title: Text(e.value)),
        ],
      ),
    ),
  );

  void _showFlagged(bool pinned) => showModalBottomSheet<void>(
    context: context,
    useSafeArea: true,
    isScrollControlled: true,
    builder: (sheet) => FutureBuilder<List<Message>>(
      future: _s.call((api) => api.flagged(widget.contactId, pinned)),
      builder: (context, snap) {
        final items = snap.data ?? const <Message>[];
        return ListView(
          shrinkWrap: true,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.s),
              child: Text(
                pinned ? 'Pinned messages' : 'Starred messages',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
            ),
            if (snap.hasData && items.isEmpty)
              Padding(
                padding: const EdgeInsets.all(Space.xl),
                child: Text(
                  pinned ? 'Long-press a message and choose Pin.' : 'Long-press a message and choose Star.',
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
              ),
            for (final m in items)
              ListTile(
                leading: Icon(pinned ? Icons.push_pin_rounded : Icons.star_rounded),
                title: Text(m.body, maxLines: 3, overflow: TextOverflow.ellipsis),
                subtitle: Text(
                  '${m.outgoing ? 'You' : _contact?.displayName ?? ''} · ${fullStamp(context, m.createdAtSecs)}',
                ),
              ),
          ],
        );
      },
    ),
  );

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final contact = _contact;
    final name = contact?.displayName ?? widget.initialName;
    final subtitle = _timer > 0
        ? 'Disappearing: ${timerLabel(_timer)}'
        : (contact?.verified ?? false)
        ? 'Verified'
        : 'Not verified';

    final items = [for (final m in _messages) BubbleData.fromMessage(m, _reactions[idKey(m.messageId)] ?? const [])];

    return Scaffold(
      appBar: AppBar(
        titleSpacing: 0,
        centerTitle: false,
        title: Semantics(
          button: true,
          label: '$name, $subtitle. Contact details',
          excludeSemantics: true,
          child: InkWell(
            borderRadius: BorderRadius.circular(Radii.card),
            onTap: () => Routes.contact(context, widget.contactId),
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.xs, horizontal: Space.xs),
              child: Row(
                children: [
                  Avatar(name: name, seed: widget.contactId, size: 36),
                  const SizedBox(width: Space.m),
                  Flexible(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(name, style: t.titleMedium, maxLines: 1, overflow: TextOverflow.ellipsis),
                        Row(
                          children: [
                            if (_timer > 0)
                              Icon(Icons.timer_outlined, size: 13, color: c.text2)
                            else
                              VerifiedBadge(verified: contact?.verified ?? false, size: 13),
                            const SizedBox(width: 4),
                            Flexible(child: Text(subtitle, style: t.bodySmall, maxLines: 1)),
                          ],
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        actions: [
          PopupMenuButton<String>(
            tooltip: 'More',
            icon: const Icon(Icons.more_horiz_rounded),
            onSelected: (v) => switch (v) {
              'search' => Routes.search(context, contactId: widget.contactId),
              'timer' => _chooseTimer(),
              'pinned' => _showFlagged(true),
              'starred' => _showFlagged(false),
              _ => Routes.contact(context, widget.contactId),
            },
            itemBuilder: (_) => const [
              PopupMenuItem(value: 'search', child: Text('Search in chat')),
              PopupMenuItem(value: 'timer', child: Text('Disappearing messages')),
              PopupMenuItem(value: 'pinned', child: Text('Pinned messages')),
              PopupMenuItem(value: 'starred', child: Text('Starred messages')),
              PopupMenuItem(value: 'contact', child: Text('Contact details')),
            ],
          ),
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
                : contact == null
                ? const EmptyState(
                    icon: Icons.person_off_outlined,
                    title: 'Contact not found',
                    body: 'This contact is no longer on this phone.',
                  )
                : Stack(
                    children: [
                      MessageList(
                        items: items,
                        controller: _scroll,
                        reactionSet: _s.startup.reactions,
                        onLongPress: _actions,
                        onNearTop: _loadMore,
                        header: _messages.length < _limit ? _FirstNotice(name: name) : null,
                      ),
                      PositionedDirectional(
                        end: Space.l,
                        bottom: Space.l,
                        child: IgnorePointer(
                          ignoring: !_showJump,
                          child: ExcludeSemantics(
                            excluding: !_showJump,
                            child: AnimatedScale(
                              scale: _showJump ? 1 : 0,
                              duration: Motion.micro,
                              child: FloatingActionButton.small(
                                heroTag: null,
                                tooltip: 'Jump to newest',
                                backgroundColor: c.surface,
                                foregroundColor: c.text,
                                onPressed: () => _scroll.animateTo(0, duration: Motion.nav, curve: Curves.easeOutCubic),
                                child: const Icon(Icons.keyboard_arrow_down_rounded),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
          ),
          if (contact != null && contact.blocked)
            Material(
              color: c.surface,
              child: SafeArea(
                top: false,
                child: Padding(
                  padding: const EdgeInsets.all(Space.l),
                  child: Row(
                    children: [
                      Expanded(
                        child: Text('You blocked $name. You will not receive their messages.', style: t.bodyMedium),
                      ),
                      TextButton(
                        onPressed: () => _run((api) => api.setBlocked(widget.contactId, false), 'Unblocked'),
                        child: const Text('Unblock'),
                      ),
                    ],
                  ),
                ),
              ),
            )
          else if (contact != null)
            Composer(
              controller: _text,
              focusNode: _focus,
              maxBytes: _s.startup.maxTextBytes,
              onSend: _send,
              context: _editing != null
                  ? ComposerContext(icon: Icons.edit_rounded, title: 'Editing', text: _editing!.body)
                  : _replyTo != null
                  ? ComposerContext(
                      icon: Icons.reply_rounded,
                      title: 'Replying to ${_replyTo!.outgoing ? 'yourself' : name}',
                      text: _replyTo!.body,
                    )
                  : null,
              onAttach: _editing == null ? _attach : null,
              onCancelContext: () {
                if (_editing != null) _text.clear();
                setState(() {
                  _editing = null;
                  _replyTo = null;
                });
              },
            ),
        ],
      ),
    );
  }
}

class _FirstNotice extends StatelessWidget {
  const _FirstNotice({required this.name});
  final String name;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.l, Space.xl, Space.l, Space.l),
      child: Container(
        padding: const EdgeInsets.all(Space.l),
        decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
        child: Column(
          children: [
            Icon(Icons.lock_outline_rounded, color: c.text2, size: 20),
            const SizedBox(height: Space.s),
            Text(
              'Messages are end-to-end encrypted and travel phone to phone, so they can take a while. '
              'You will see Delivered only when $name\'s phone confirms it.',
              style: t.bodySmall,
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}

class _ReactionButton extends StatelessWidget {
  const _ReactionButton({required this.emoji, required this.selected, required this.onTap});
  final String emoji;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return Semantics(
      button: true,
      selected: selected,
      label: selected ? 'Remove reaction $emoji' : 'React $emoji',
      excludeSemantics: true,
      child: InkWell(
        customBorder: const CircleBorder(),
        onTap: onTap,
        child: Container(
          width: 48,
          height: 48,
          alignment: Alignment.center,
          decoration: BoxDecoration(shape: BoxShape.circle, color: selected ? c.accent.withValues(alpha: 0.16) : null),
          child: Text(emoji, style: const TextStyle(fontSize: 24)),
        ),
      ),
    );
  }
}
