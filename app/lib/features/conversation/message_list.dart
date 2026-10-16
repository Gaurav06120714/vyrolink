import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/delivery.dart';
import '../../util/format.dart';
import 'attachment_view.dart';

/// What a bubble shows, for one-to-one and group messages alike.
class BubbleData {
  BubbleData({
    required this.id,
    required this.outgoing,
    required this.body,
    required this.createdAtSecs,
    this.author,
    this.state,
    this.status,
    this.deleted = false,
    this.edited = false,
    this.forwarded = false,
    this.pinned = false,
    this.starred = false,
    this.disappearsAtSecs,
    this.replyTo,
    this.reactions = const [],
    this.attachment,
  });

  factory BubbleData.fromMessage(Message m, List<Reaction> reactions) {
    final a = m.attachment;
    // A message with an attachment is fully delivered only when every chunk
    // has its own authenticated proof; until then say how many have it.
    final partial = a != null && m.outgoing && m.state == DeliveryState.delivered && a.deliveredChunks < a.chunkCount;
    return BubbleData(
      id: m.messageId,
      outgoing: m.outgoing,
      body: m.body,
      createdAtSecs: m.createdAtSecs,
      state: m.outgoing ? m.state : null,
      deleted: m.deleted,
      edited: m.edited,
      forwarded: m.forwarded,
      pinned: m.pinned,
      starred: m.starred,
      disappearsAtSecs: m.disappearsAtSecs,
      replyTo: m.replyTo,
      reactions: reactions,
      attachment: a,
      status: partial ? 'Message delivered · ${a.deliveredChunks} of ${a.chunkCount} file parts confirmed' : null,
    );
  }

  final Uint8List id;
  final bool outgoing;
  final String body;
  final int createdAtSecs;

  /// Sender name in groups (incoming only).
  final String? author;
  final DeliveryState? state;

  /// Status line under the newest outgoing message when there is no
  /// per-message state (groups: "Delivered to 2 of 3").
  final String? status;
  final bool deleted, edited, forwarded, pinned, starred;
  final int? disappearsAtSecs;
  final Uint8List? replyTo;
  final List<Reaction> reactions;
  final AttachmentInfo? attachment;

  String get key => idKey(id);
}

/// Reversed, virtualised list (newest at the bottom) with day separators and
/// grouping of consecutive messages from the same side within 3 minutes.
class MessageList extends StatelessWidget {
  const MessageList({
    super.key,
    required this.items,
    required this.reactionSet,
    required this.onLongPress,
    this.controller,
    this.onNearTop,
    this.header,
  });

  /// Newest first.
  final List<BubbleData> items;
  final List<String> reactionSet;
  final void Function(BubbleData) onLongPress;
  final ScrollController? controller;
  final VoidCallback? onNearTop;

  /// Shown above the oldest message (e.g. the first-message notice).
  final Widget? header;

  @override
  Widget build(BuildContext context) {
    final byId = {for (final m in items) m.key: m};
    final newestOutgoing = items.where((m) => m.outgoing && !m.deleted).firstOrNull?.key;
    return NotificationListener<ScrollNotification>(
      onNotification: (n) {
        if (onNearTop != null && n.metrics.extentAfter < 600) onNearTop!();
        return false;
      },
      child: ListView.builder(
        controller: controller,
        reverse: true,
        padding: EdgeInsets.symmetric(horizontal: Space.gutter(context) - Space.xs, vertical: Space.m),
        itemCount: items.length + (header == null ? 0 : 1),
        itemBuilder: (context, i) {
          if (i == items.length) return header!;
          final m = items[i];
          final older = i + 1 < items.length ? items[i + 1] : null;
          final newer = i > 0 ? items[i - 1] : null;
          final day = fromSecs(m.createdAtSecs);
          final newDay = older == null || !_sameDay(fromSecs(older.createdAtSecs), day);
          final joinsOlder = !newDay && _joins(older, m);
          final joinsNewer = newer != null && _sameDay(fromSecs(newer.createdAtSecs), day) && _joins(m, newer);
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (newDay) _DaySeparator(label: dayLabel(context, day)),
              Padding(
                padding: EdgeInsets.only(top: joinsOlder ? 2 : Space.m),
                child: Bubble(
                  data: m,
                  quoted: m.replyTo == null ? null : byId[idKey(m.replyTo!)],
                  hasQuote: m.replyTo != null,
                  tail: !joinsNewer,
                  showAuthor: !m.outgoing && m.author != null && !joinsOlder,
                  showStatus: m.key == newestOutgoing,
                  reactionSet: reactionSet,
                  onLongPress: () => onLongPress(m),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  static bool _sameDay(DateTime a, DateTime b) => a.year == b.year && a.month == b.month && a.day == b.day;

  static bool _joins(BubbleData older, BubbleData newer) =>
      older.outgoing == newer.outgoing &&
      older.author == newer.author &&
      (newer.createdAtSecs - older.createdAtSecs).abs() < 180;
}

class _DaySeparator extends StatelessWidget {
  const _DaySeparator({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: Space.xl, bottom: Space.xs),
      child: Center(
        child: Semantics(
          header: true,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: Space.m, vertical: Space.xs),
            decoration: BoxDecoration(color: c.surface2, borderRadius: BorderRadius.circular(Radii.chip)),
            child: Text(label, style: Theme.of(context).textTheme.labelMedium!.copyWith(color: c.text2)),
          ),
        ),
      ),
    );
  }
}

class Bubble extends StatelessWidget {
  const Bubble({
    super.key,
    required this.data,
    required this.quoted,
    required this.hasQuote,
    required this.tail,
    required this.showAuthor,
    required this.showStatus,
    required this.reactionSet,
    required this.onLongPress,
  });

  final BubbleData data;
  final BubbleData? quoted;
  final bool hasQuote, tail, showAuthor, showStatus;
  final List<String> reactionSet;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final out = data.outgoing;
    final bg = data.deleted ? Colors.transparent : (out ? c.bubbleOut : c.bubbleIn);
    final fg = data.deleted ? c.text2 : (out ? c.onBubbleOut : c.text);
    final meta = fg.withValues(alpha: out && !data.deleted ? 0.78 : 1);
    const r = Radius.circular(Radii.bubble);
    const tailR = Radius.circular(Radii.bubbleTail);
    final radius = BorderRadiusDirectional.only(
      topStart: r,
      topEnd: r,
      bottomStart: !out && tail ? tailR : r,
      bottomEnd: out && tail ? tailR : r,
    );
    final time = timeOfDay(context, fromSecs(data.createdAtSecs));
    final info = data.state == null ? null : DeliveryInfo.of(data.state!);

    final body = data.deleted ? (out ? 'You deleted this message' : 'This message was deleted') : data.body;

    final att = data.deleted ? null : data.attachment;
    final semantics = [
      if (data.author != null && !out) data.author!,
      if (out) 'You',
      if (data.forwarded) 'Forwarded',
      if (att != null) att.kind == attachmentImage ? 'Photo' : 'File ${att.name}',
      body,
      time,
      if (data.edited) 'Edited',
      if (info != null) info.label,
      if (data.pinned) 'Pinned',
      if (data.starred) 'Starred',
      for (final r in data.reactions)
        if (r.reaction >= 1 && r.reaction <= reactionSet.length)
          '${r.fromMe ? 'You' : 'They'} reacted ${reactionSet[r.reaction - 1]}',
    ].join(', ');

    final maxWidth = MediaQuery.sizeOf(context).width * (MediaQuery.sizeOf(context).width >= 600 ? 0.6 : 0.8);

    return Semantics(
      label: semantics,
      excludeSemantics: true,
      onLongPressHint: 'Message actions',
      customSemanticsActions: {const CustomSemanticsAction(label: 'Message actions'): onLongPress},
      child: Align(
        alignment: out ? AlignmentDirectional.centerEnd : AlignmentDirectional.centerStart,
        child: ConstrainedBox(
          constraints: BoxConstraints(maxWidth: maxWidth),
          child: Column(
            crossAxisAlignment: out ? CrossAxisAlignment.end : CrossAxisAlignment.start,
            children: [
              if (showAuthor)
                Padding(
                  padding: const EdgeInsetsDirectional.only(start: Space.m, bottom: 2),
                  child: Text(data.author!, style: t.labelMedium!.copyWith(color: c.accent)),
                ),
              GestureDetector(
                onLongPress: onLongPress,
                child: Container(
                  padding: const EdgeInsets.fromLTRB(Space.m, Space.s, Space.m, 6),
                  decoration: BoxDecoration(
                    color: bg,
                    borderRadius: radius,
                    border: data.deleted ? Border.all(color: c.divider) : null,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (data.forwarded && !data.deleted)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 2),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(Icons.shortcut_rounded, size: 14, color: meta),
                              const SizedBox(width: 4),
                              Text(
                                'Forwarded',
                                style: t.labelSmall!.copyWith(color: meta, fontStyle: FontStyle.italic),
                              ),
                            ],
                          ),
                        ),
                      if (hasQuote && !data.deleted) _Quote(quoted: quoted, onBubble: out),
                      if (att != null) AttachmentView(messageId: data.id, info: att, onBubble: out),
                      if (body.isNotEmpty || att == null)
                        Text(
                          body,
                          style: t.bodyLarge!.copyWith(color: fg, fontStyle: data.deleted ? FontStyle.italic : null),
                        ),
                      const SizedBox(height: 2),
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (data.pinned) ...[
                            Icon(Icons.push_pin_rounded, size: 12, color: meta),
                            const SizedBox(width: 3),
                          ],
                          if (data.starred) ...[
                            Icon(Icons.star_rounded, size: 12, color: meta),
                            const SizedBox(width: 3),
                          ],
                          if (data.disappearsAtSecs != null) ...[
                            Icon(Icons.timer_outlined, size: 12, color: meta),
                            const SizedBox(width: 3),
                          ],
                          if (data.edited && !data.deleted)
                            Text('Edited · ', style: t.bodySmall!.copyWith(color: meta, fontSize: 12)),
                          Text(time, style: t.bodySmall!.copyWith(color: meta, fontSize: 12)),
                          if (data.state != null && !data.deleted) ...[
                            const SizedBox(width: 4),
                            DeliveryGlyph(state: data.state!, size: 14, color: meta),
                          ],
                        ],
                      ),
                    ],
                  ),
                ),
              ),
              if (data.reactions.isNotEmpty && !data.deleted) _Reactions(reactions: data.reactions, set: reactionSet),
              if (showStatus && out && (info != null || data.status != null))
                Padding(
                  padding: const EdgeInsets.only(top: 4, right: 4, left: 4),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      if (data.state != null) ...[
                        DeliveryGlyph(state: data.state!, size: 13),
                        const SizedBox(width: 4),
                      ],
                      Flexible(
                        child: Text(
                          data.status ?? info!.label,
                          style: t.bodySmall!.copyWith(color: info?.tone(c) ?? c.text2),
                        ),
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

class _Quote extends StatelessWidget {
  const _Quote({required this.quoted, required this.onBubble});
  final BubbleData? quoted;
  final bool onBubble;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final fg = onBubble ? c.onBubbleOut : c.text;
    final q = quoted;
    final who = q == null ? '' : (q.outgoing ? 'You' : (q.author ?? 'Them'));
    final text = q == null ? 'Original message not on this phone' : (q.deleted ? 'Deleted message' : q.body);
    return Container(
      margin: const EdgeInsets.only(bottom: Space.xs, top: 2),
      padding: const EdgeInsets.fromLTRB(Space.s, Space.xs, Space.s, Space.xs),
      decoration: BoxDecoration(
        color: fg.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(Radii.chip),
        border: BorderDirectional(start: BorderSide(color: fg.withValues(alpha: 0.6), width: 3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          if (who.isNotEmpty) Text(who, style: t.labelMedium!.copyWith(color: fg)),
          Text(
            text,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: t.bodyMedium!.copyWith(
              color: fg.withValues(alpha: 0.85),
              fontStyle: q == null ? FontStyle.italic : null,
            ),
          ),
        ],
      ),
    );
  }
}

class _Reactions extends StatelessWidget {
  const _Reactions({required this.reactions, required this.set});
  final List<Reaction> reactions;
  final List<String> set;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final shown = reactions.where((r) => r.reaction >= 1 && r.reaction <= set.length).toList();
    if (shown.isEmpty) return const SizedBox.shrink();
    return Transform.translate(
      offset: const Offset(0, -4),
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: Space.s),
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          color: c.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: c.divider),
        ),
        child: Text(shown.map((r) => set[r.reaction - 1]).join(' '), style: const TextStyle(fontSize: 14)),
      ),
    );
  }
}
