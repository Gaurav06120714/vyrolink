import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/nearby.dart';
import '../../design/widgets/states.dart';

/// Great-circle distance in metres between two fixes (haversine).
double distanceMetres(int lat1E7, int lon1E7, int lat2E7, int lon2E7) {
  const r = 6371000.0;
  final p1 = lat1E7 / 1e7 * math.pi / 180, p2 = lat2E7 / 1e7 * math.pi / 180;
  final dp = p2 - p1, dl = (lon2E7 - lon1E7) / 1e7 * math.pi / 180;
  final a = math.pow(math.sin(dp / 2), 2) + math.cos(p1) * math.cos(p2) * math.pow(math.sin(dl / 2), 2);
  return 2 * r * math.asin(math.sqrt(a.toDouble()));
}

/// Initial compass bearing from the first fix to the second, 0–360°.
double bearingDegrees(int lat1E7, int lon1E7, int lat2E7, int lon2E7) {
  final p1 = lat1E7 / 1e7 * math.pi / 180, p2 = lat2E7 / 1e7 * math.pi / 180;
  final dl = (lon2E7 - lon1E7) / 1e7 * math.pi / 180;
  final y = math.sin(dl) * math.cos(p2);
  final x = math.cos(p1) * math.sin(p2) - math.sin(p1) * math.cos(p2) * math.cos(dl);
  return (math.atan2(y, x) * 180 / math.pi + 360) % 360;
}

String compassPoint(double degrees) =>
    const ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'][((degrees + 22.5) % 360) ~/ 45];

String formatDistance(double m) => m < 1000 ? '${m.round()} m' : '${(m / 1000).toStringAsFixed(m < 10000 ? 1 : 0)} km';

/// A shared location: when, how accurate, and — computed on this phone with
/// no internet — how far away and in which direction it is from you.
class LocationCard extends StatefulWidget {
  const LocationCard({super.key, required this.info, required this.onBubble, required this.label});
  final LocationInfo info;
  final bool onBubble;
  final String label;

  @override
  State<LocationCard> createState() => _LocationCardState();
}

class _LocationCardState extends State<LocationCard> {
  String? _fromMe;
  bool _busy = false;

  Future<void> _measure() async {
    setState(() => _busy = true);
    try {
      final me = await context.services.call((api) => api.currentLocation());
      if (!mounted) return;
      if (me == null) {
        showMessage(context, 'Your location is not available. Allow it in Settings to measure the distance.');
      } else {
        final i = widget.info;
        final d = distanceMetres(me.latE7, me.lonE7, i.latE7, i.lonE7);
        final b = bearingDegrees(me.latE7, me.lonE7, i.latE7, i.lonE7);
        setState(() => _fromMe = d < 15 ? 'Right here' : '${formatDistance(d)} ${compassPoint(b)} of you');
      }
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final fg = widget.onBubble ? Colors.white : c.text;
    final i = widget.info;
    final lat = (i.latE7 / 1e7).toStringAsFixed(5), lon = (i.lonE7 / 1e7).toStringAsFixed(5);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.location_on_rounded, color: fg, size: 22),
            const SizedBox(width: Space.xs),
            Text('Location', style: t.titleSmall!.copyWith(color: fg)),
          ],
        ),
        const SizedBox(height: 2),
        Text(
          '$lat, $lon · ±${i.accuracyM} m · ${seenAgo(i.capturedMs ~/ 1000)}',
          style: t.bodySmall!.copyWith(color: fg.withValues(alpha: 0.85)),
        ),
        if (_fromMe != null) ...[
          const SizedBox(height: Space.xs),
          Text(_fromMe!, style: t.titleSmall!.copyWith(color: fg)),
        ],
        const SizedBox(height: Space.xs),
        Wrap(
          spacing: Space.s,
          children: [
            TextButton(
              onPressed: _busy ? null : _measure,
              style: TextButton.styleFrom(foregroundColor: fg, padding: EdgeInsets.zero),
              child: Text(_busy ? 'Measuring…' : 'Distance from me'),
            ),
            TextButton(
              onPressed: () => context.services.call((api) => api.openMap(i.latE7, i.lonE7, widget.label)),
              style: TextButton.styleFrom(foregroundColor: fg, padding: EdgeInsets.zero),
              child: const Text('Open in Maps'),
            ),
          ],
        ),
      ],
    );
  }
}

/// A poll inside a bubble. Tapping an answer sends my whole new choice.
class PollCard extends StatelessWidget {
  const PollCard({super.key, required this.pollId, required this.poll, required this.onBubble});
  final Uint8List pollId;
  final PollInfo poll;
  final bool onBubble;

  Future<void> _tap(BuildContext context, int i) async {
    final bit = 1 << i;
    final next = poll.multi ? poll.mine ^ bit : (poll.mine == bit ? 0 : bit);
    try {
      await context.services.mutate((api) => api.vote(pollId, next));
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final fg = onBubble ? Colors.white : c.text;
    final most = poll.counts.fold<int>(0, math.max);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.poll_outlined, color: fg, size: 20),
            const SizedBox(width: Space.xs),
            Flexible(
              child: Text(poll.question, style: t.titleSmall!.copyWith(color: fg)),
            ),
          ],
        ),
        Text(
          '${poll.multi ? 'Choose any' : 'Choose one'} · ${poll.voters} ${poll.voters == 1 ? 'vote' : 'votes'}',
          style: t.bodySmall!.copyWith(color: fg.withValues(alpha: 0.8)),
        ),
        const SizedBox(height: Space.xs),
        for (var i = 0; i < poll.options.length; i++)
          _Option(
            label: poll.options[i],
            count: poll.counts[i],
            fraction: most == 0 ? 0 : poll.counts[i] / most,
            chosen: poll.mine & (1 << i) != 0,
            multi: poll.multi,
            fg: fg,
            onTap: () => _tap(context, i),
          ),
      ],
    );
  }
}

class _Option extends StatelessWidget {
  const _Option({
    required this.label,
    required this.count,
    required this.fraction,
    required this.chosen,
    required this.multi,
    required this.fg,
    required this.onTap,
  });
  final String label;
  final int count;
  final double fraction;
  final bool chosen;
  final bool multi;
  final Color fg;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final icon = multi
        ? (chosen ? Icons.check_box_rounded : Icons.check_box_outline_blank_rounded)
        : (chosen ? Icons.radio_button_checked_rounded : Icons.radio_button_unchecked_rounded);
    return Semantics(
      button: true,
      selected: chosen,
      label: '$label, $count ${count == 1 ? 'vote' : 'votes'}',
      excludeSemantics: true,
      child: InkWell(
        onTap: onTap,
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 44),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: Space.xs),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(icon, color: fg, size: 20),
                    const SizedBox(width: Space.s),
                    Expanded(
                      child: Text(label, style: t.bodyMedium!.copyWith(color: fg)),
                    ),
                    Text('$count', style: t.labelMedium!.copyWith(color: fg)),
                  ],
                ),
                const SizedBox(height: 4),
                ClipRRect(
                  borderRadius: BorderRadius.circular(2),
                  child: LinearProgressIndicator(
                    value: fraction,
                    minHeight: 4,
                    color: fg,
                    backgroundColor: fg.withValues(alpha: 0.2),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Asks for a question and 2–10 answers.
Future<({String question, List<String> options, bool multi})?> composePoll(BuildContext context) =>
    showModalBottomSheet<({String question, List<String> options, bool multi})>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (_) => const _PollComposer(),
    );

class _PollComposer extends StatefulWidget {
  const _PollComposer();

  @override
  State<_PollComposer> createState() => _PollComposerState();
}

class _PollComposerState extends State<_PollComposer> {
  final _question = TextEditingController();
  final _options = [TextEditingController(), TextEditingController()];
  bool _multi = false;

  @override
  void dispose() {
    _question.dispose();
    for (final o in _options) {
      o.dispose();
    }
    super.dispose();
  }

  List<String> get _filled => _options.map((o) => o.text.trim()).where((o) => o.isNotEmpty).toList();
  bool get _valid => _question.text.trim().isNotEmpty && _filled.length >= 2;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: ListView(
        shrinkWrap: true,
        padding: EdgeInsets.all(Space.gutter(context)),
        children: [
          Text('New poll', style: t.headlineSmall),
          const SizedBox(height: Space.m),
          TextField(
            controller: _question,
            maxLength: 300,
            autofocus: true,
            decoration: const InputDecoration(labelText: 'Question'),
            onChanged: (_) => setState(() {}),
          ),
          for (var i = 0; i < _options.length; i++)
            TextField(
              controller: _options[i],
              maxLength: 100,
              decoration: InputDecoration(labelText: 'Answer ${i + 1}'),
              onChanged: (_) => setState(() {}),
            ),
          if (_options.length < 10)
            Align(
              alignment: AlignmentDirectional.centerStart,
              child: TextButton.icon(
                onPressed: () => setState(() => _options.add(TextEditingController())),
                icon: const Icon(Icons.add_rounded),
                label: const Text('Add answer'),
              ),
            ),
          SwitchListTile.adaptive(
            contentPadding: EdgeInsets.zero,
            title: const Text('Allow several answers'),
            value: _multi,
            onChanged: (v) => setState(() => _multi = v),
          ),
          const SizedBox(height: Space.s),
          FilledButton(
            onPressed: _valid
                ? () => Navigator.pop(context, (question: _question.text.trim(), options: _filled, multi: _multi))
                : null,
            child: const Text('Send poll'),
          ),
        ],
      ),
    );
  }
}

/// An introduction offered to me (ADR-037): accept or decline, then progress.
class IntroCard extends StatelessWidget {
  const IntroCard({super.key, required this.body, required this.intro, required this.onBubble});
  final String body;
  final IntroInfo intro;
  final bool onBubble;

  Future<void> _do(BuildContext context, Future<void> Function(VlHostApi api) op) async {
    try {
      await context.services.mutate(op);
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final fg = onBubble ? Colors.white : c.text;
    final name = intro.otherName;
    final status = switch (intro.state) {
      1 => 'Accepted. Waiting for $name to accept too.',
      2 => 'You declined.',
      3 => 'Connected. You can now message $name. Compare your safety number in person to verify.',
      4 => 'Could not connect: the card did not belong to $name.',
      _ => 'They will be added as a contact you have not verified in person.',
    };
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(body, style: t.bodyLarge!.copyWith(color: fg)),
        const SizedBox(height: Space.xs),
        Text(status, style: t.bodySmall!.copyWith(color: fg.withValues(alpha: 0.85))),
        if (intro.state == 0)
          Wrap(
            spacing: Space.s,
            children: [
              TextButton(
                onPressed: () {
                  final me = context.services.profileName;
                  _do(context, (api) => api.acceptIntro(intro.introId, me.isEmpty ? 'VYRO LINK user' : me));
                },
                style: TextButton.styleFrom(foregroundColor: fg, padding: EdgeInsets.zero),
                child: const Text('Accept'),
              ),
              TextButton(
                onPressed: () => _do(context, (api) => api.declineIntro(intro.introId)),
                style: TextButton.styleFrom(foregroundColor: fg, padding: EdgeInsets.zero),
                child: const Text('Decline'),
              ),
            ],
          ),
      ],
    );
  }
}
