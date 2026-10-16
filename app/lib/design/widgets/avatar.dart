import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../tokens.dart';

/// Initials on a muted tone derived from the id. No images are ever fetched
/// (Doc 16 D2), so this is the only avatar.
class Avatar extends StatelessWidget {
  const Avatar({super.key, required this.name, required this.seed, this.size = 44, this.group = false});

  final String name;
  final Uint8List seed;
  final double size;
  final bool group;

  static const _hues = [214.0, 168.0, 28.0, 280.0, 340.0, 96.0, 196.0, 250.0];

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final hue = _hues[seed.fold<int>(0, (a, b) => (a * 31 + b) & 0x7fffffff) % _hues.length];
    final bg = HSLColor.fromAHSL(1, hue, dark ? 0.30 : 0.45, dark ? 0.26 : 0.88).toColor();
    final fg = HSLColor.fromAHSL(1, hue, dark ? 0.55 : 0.55, dark ? 0.82 : 0.28).toColor();
    return ExcludeSemantics(
      child: Container(
        width: size,
        height: size,
        alignment: Alignment.center,
        decoration: BoxDecoration(color: bg, borderRadius: BorderRadius.circular(group ? size * 0.3 : size / 2)),
        child: group
            ? Icon(Icons.group_rounded, color: fg, size: size * 0.5)
            : Text(
                initials(name),
                style: TextStyle(
                  color: fg,
                  fontWeight: FontWeight.w600,
                  fontSize: size * 0.38,
                  height: 1,
                  letterSpacing: 0.3,
                ),
                textScaler: TextScaler.noScaling,
              ),
      ),
    );
  }

  static String initials(String name) {
    final parts = name.trim().split(RegExp(r'\s+')).where((p) => p.isNotEmpty).toList();
    if (parts.isEmpty) return '?';
    final first = parts.first.characters.first;
    final last = parts.length > 1 ? parts.last.characters.first : '';
    return (first + last).toUpperCase();
  }
}

/// Small shield shown next to verified contacts.
class VerifiedBadge extends StatelessWidget {
  const VerifiedBadge({super.key, required this.verified, this.size = 16});
  final bool verified;
  final double size;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return Semantics(
      label: verified ? 'Verified' : 'Not verified',
      child: Icon(
        verified ? Icons.verified_user_rounded : Icons.shield_outlined,
        size: size,
        color: verified ? c.ok : c.text2,
      ),
    );
  }
}
