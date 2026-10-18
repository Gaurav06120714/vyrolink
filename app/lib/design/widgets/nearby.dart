import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../app/services.dart';
import '../tokens.dart';

/// "Nearby" for a paired contact whose phone was just in Bluetooth range
/// (ADR-029). With `detail`, it says when they were seen, and when they are
/// not near now it falls back to "Last seen nearby …" (ADR-033).
class NearbyTag extends StatelessWidget {
  const NearbyTag({super.key, required this.contactId, this.detail = false, this.lastSeenSecs});
  final Uint8List contactId;
  final bool detail;
  final int? lastSeenSecs;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<Map<String, int>>(
      valueListenable: context.services.nearby,
      builder: (context, nearby, _) {
        final seen = nearby[idKey(contactId)];
        final c = VlColors.of(context);
        if (seen == null) {
          final last = lastSeenSecs;
          if (!detail || last == null) return const SizedBox.shrink();
          return Text(
            'Last seen nearby ${seenAgo(last)}',
            style: Theme.of(context).textTheme.labelMedium!.copyWith(color: c.text2),
          );
        }
        final style = Theme.of(context).textTheme.labelMedium!.copyWith(color: c.ok);
        final text = detail ? 'Nearby · seen ${seenAgo(seen)}' : 'Nearby';
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(color: c.ok, shape: BoxShape.circle),
            ),
            const SizedBox(width: Space.xs),
            Text(text, style: style),
          ],
        );
      },
    );
  }
}

/// "just now", "4 min ago", "3 h ago" or "2 days ago", from unix seconds.
String seenAgo(int seenSecs, {DateTime? now}) {
  final nowSecs = (now ?? DateTime.now()).millisecondsSinceEpoch ~/ 1000;
  final mins = (nowSecs - seenSecs) ~/ 60;
  if (mins < 1) return 'just now';
  if (mins < 60) return '$mins min ago';
  if (mins < 48 * 60) return '${mins ~/ 60} h ago';
  return '${mins ~/ (24 * 60)} days ago';
}
