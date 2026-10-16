import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../design/tokens.dart';

/// Reply or edit context shown above the input.
class ComposerContext {
  const ComposerContext({required this.icon, required this.title, required this.text});
  final IconData icon;
  final String title;
  final String text;
}

/// Message input that grows to six lines, counts UTF-8 bytes against the
/// core's limit and keeps clear of the keyboard and home indicator.
class Composer extends StatefulWidget {
  const Composer({
    super.key,
    required this.controller,
    required this.maxBytes,
    required this.onSend,
    this.focusNode,
    this.context,
    this.onCancelContext,
    this.hint = 'Message',
    this.sendLabel = 'Send',
    this.onAttach,
  });

  final TextEditingController controller;
  final FocusNode? focusNode;
  final int maxBytes;
  final Future<bool> Function(String text) onSend;
  final ComposerContext? context;
  final VoidCallback? onCancelContext;
  final String hint;
  final String sendLabel;

  /// Shows an attach button when set.
  final VoidCallback? onAttach;

  @override
  State<Composer> createState() => _ComposerState();
}

class _ComposerState extends State<Composer> {
  bool _sending = false;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_changed);
  }

  @override
  void didUpdateWidget(Composer old) {
    super.didUpdateWidget(old);
    if (old.controller != widget.controller) {
      old.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    super.dispose();
  }

  void _changed() => setState(() {});

  Future<void> _send() async {
    final text = widget.controller.text.trim();
    if (text.isEmpty || _sending) return;
    setState(() => _sending = true);
    final ok = await widget.onSend(text);
    if (!mounted) return;
    setState(() => _sending = false);
    if (ok) {
      HapticFeedback.lightImpact();
      widget.controller.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final text = widget.controller.text;
    final bytes = utf8.encode(text).length;
    final left = widget.maxBytes - bytes;
    final over = left < 0;
    final canSend = text.trim().isNotEmpty && !over && !_sending;
    final ctx = widget.context;

    return Material(
      color: c.surface,
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Divider(height: 1, color: c.divider),
            if (ctx != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(Space.l, Space.s, Space.xs, 0),
                child: Row(
                  children: [
                    Icon(ctx.icon, size: 20, color: c.accent),
                    const SizedBox(width: Space.m),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(ctx.title, style: t.labelMedium!.copyWith(color: c.accent)),
                          Text(ctx.text, maxLines: 1, overflow: TextOverflow.ellipsis, style: t.bodySmall),
                        ],
                      ),
                    ),
                    IconButton(
                      tooltip: 'Cancel',
                      onPressed: widget.onCancelContext,
                      icon: const Icon(Icons.close_rounded),
                    ),
                  ],
                ),
              ),
            Padding(
              padding: const EdgeInsets.fromLTRB(Space.m, Space.s, Space.s, Space.s),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  if (widget.onAttach != null)
                    IconButton(
                      tooltip: 'Attach photo or file',
                      onPressed: widget.onAttach,
                      icon: const Icon(Icons.add_circle_outline_rounded),
                    ),
                  Expanded(
                    child: TextField(
                      controller: widget.controller,
                      focusNode: widget.focusNode,
                      minLines: 1,
                      maxLines: 6,
                      keyboardType: TextInputType.multiline,
                      textCapitalization: TextCapitalization.sentences,
                      style: t.bodyLarge,
                      decoration: InputDecoration(
                        hintText: widget.hint,
                        isDense: true,
                        fillColor: c.bg,
                        contentPadding: const EdgeInsets.symmetric(horizontal: Space.l, vertical: 11),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(22),
                          borderSide: BorderSide.none,
                        ),
                        enabledBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(22),
                          borderSide: BorderSide(color: c.divider),
                        ),
                        focusedBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(22),
                          borderSide: BorderSide(color: over ? c.err : c.accent, width: 1.5),
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(width: Space.s),
                  AnimatedSwitcher(
                    duration: Motion.micro,
                    child: IconButton.filled(
                      key: ValueKey(canSend),
                      tooltip: widget.sendLabel,
                      onPressed: canSend ? _send : null,
                      style: IconButton.styleFrom(
                        backgroundColor: c.accent,
                        foregroundColor: c.onAccent,
                        disabledBackgroundColor: c.surface2,
                        disabledForegroundColor: c.text2,
                        minimumSize: const Size(44, 44),
                      ),
                      icon: Icon(ctx?.icon == Icons.edit_rounded ? Icons.check_rounded : Icons.arrow_upward_rounded),
                    ),
                  ),
                ],
              ),
            ),
            if (left < widget.maxBytes * 0.15)
              Padding(
                padding: const EdgeInsets.only(right: Space.xl, bottom: Space.xs),
                child: Align(
                  alignment: AlignmentDirectional.centerEnd,
                  child: Semantics(
                    liveRegion: true,
                    child: Text(
                      over ? '${-left} bytes over the limit' : '$left bytes left',
                      style: t.bodySmall!.copyWith(color: over ? c.err : c.text2),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
