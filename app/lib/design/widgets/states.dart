import 'package:flutter/material.dart';

import '../../app/failure.dart';
import '../tokens.dart';

/// Line-art icon, title, one sentence and at most one primary action.
class EmptyState extends StatelessWidget {
  const EmptyState({super.key, required this.icon, required this.title, required this.body, this.action});

  final IconData icon;
  final String title;
  final String body;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(Space.xxl),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 360),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 72,
                height: 72,
                decoration: BoxDecoration(color: c.surface2, shape: BoxShape.circle),
                child: Icon(icon, size: 32, color: c.text2),
              ),
              const SizedBox(height: Space.xl),
              Text(title, style: t.titleMedium, textAlign: TextAlign.center),
              const SizedBox(height: Space.s),
              Text(
                body,
                style: t.bodyMedium!.copyWith(color: c.text2),
                textAlign: TextAlign.center,
              ),
              if (action != null) ...[const SizedBox(height: Space.xl), action!],
            ],
          ),
        ),
      ),
    );
  }
}

class ErrorState extends StatelessWidget {
  const ErrorState({super.key, required this.failure, this.onRetry});

  final VlFailure failure;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) => EmptyState(
    icon: Icons.error_outline_rounded,
    title: 'Could not load this',
    body: failure.message,
    action: onRetry == null ? null : OutlinedButton(onPressed: onRetry, child: const Text('Try again')),
  );
}

enum BannerKind { info, warn, err }

/// Inline banner. Errors are never auto-dismissed (Doc 16 §7).
class InfoBanner extends StatelessWidget {
  const InfoBanner({super.key, required this.kind, required this.text, this.icon, this.action, this.onAction});

  final BannerKind kind;
  final String text;
  final IconData? icon;
  final String? action;
  final VoidCallback? onAction;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final tone = switch (kind) {
      BannerKind.info => c.accent,
      BannerKind.warn => c.warn,
      BannerKind.err => c.err,
    };
    final glyph =
        icon ??
        switch (kind) {
          BannerKind.info => Icons.info_outline_rounded,
          BannerKind.warn => Icons.warning_amber_rounded,
          BannerKind.err => Icons.error_outline_rounded,
        };
    return Semantics(
      container: true,
      liveRegion: kind == BannerKind.err,
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: Space.l, vertical: Space.s),
        padding: const EdgeInsets.fromLTRB(Space.m, Space.m, Space.s, Space.m),
        decoration: BoxDecoration(color: tone.withValues(alpha: 0.10), borderRadius: BorderRadius.circular(Radii.card)),
        child: Row(
          children: [
            Icon(glyph, color: tone, size: 20),
            const SizedBox(width: Space.m),
            Expanded(child: Text(text, style: Theme.of(context).textTheme.bodyMedium)),
            if (action != null) TextButton(onPressed: onAction, child: Text(action!)),
          ],
        ),
      ),
    );
  }
}

/// Uppercase caption above a group of rows.
class SectionHeader extends StatelessWidget {
  const SectionHeader(this.title, {super.key});
  final String title;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(Space.xl, Space.xl, Space.xl, Space.s),
    child: Semantics(
      header: true,
      child: Text(
        title.toUpperCase(),
        style: Theme.of(context).textTheme.labelSmall!.copyWith(color: VlColors.of(context).text2),
      ),
    ),
  );
}

/// A rounded surface holding rows separated by hairlines (settings style).
class Section extends StatelessWidget {
  const Section({super.key, required this.rows, this.footer});

  final List<Widget> rows;
  final String? footer;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final items = <Widget>[];
    for (var i = 0; i < rows.length; i++) {
      if (i > 0) items.add(const Divider(indent: Space.l));
      items.add(rows[i]);
    }
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: Space.l),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Material(
            color: c.surface,
            clipBehavior: Clip.antiAlias,
            borderRadius: BorderRadius.circular(Radii.card),
            child: Column(children: items),
          ),
          if (footer != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.s, Space.s, Space.s, 0),
              child: Text(footer!, style: Theme.of(context).textTheme.bodySmall),
            ),
        ],
      ),
    );
  }
}

void showMessage(BuildContext context, String text) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text(text)));
}

/// Asks for a short name. The dialog owns its text controller, so it is
/// disposed only after the dialog has finished closing; disposing it right
/// after `showDialog` returns breaks the closing animation.
Future<String?> askName(BuildContext context, {required String title, required String initial, String? note}) =>
    showAdaptiveDialog<String>(
      context: context,
      builder: (_) => _NameDialog(title: title, initial: initial, note: note),
    );

class _NameDialog extends StatefulWidget {
  const _NameDialog({required this.title, required this.initial, this.note});
  final String title;
  final String initial;
  final String? note;

  @override
  State<_NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<_NameDialog> {
  late final _text = TextEditingController(text: widget.initial);

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(controller: _text, autofocus: true, maxLength: 32, textCapitalization: TextCapitalization.words),
          if (widget.note != null) Text(widget.note!, style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        TextButton(onPressed: () => Navigator.pop(context, _text.text.trim()), child: const Text('Save')),
      ],
    );
  }
}
