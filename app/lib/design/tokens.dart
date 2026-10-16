import 'package:flutter/material.dart';

/// Brand tokens from Doc 16 §3–§5. Widgets read colours through [VlColors]
/// (a theme extension), never from literals.
abstract final class Space {
  static const xs = 4.0;
  static const s = 8.0;
  static const m = 12.0;
  static const l = 16.0;
  static const xl = 24.0;
  static const xxl = 32.0;
  static const xxxl = 48.0;

  /// Side gutter: 16 on phones, 24 from 600 dp.
  static double gutter(BuildContext context) => MediaQuery.sizeOf(context).width >= 600 ? xl : l;
}

abstract final class Radii {
  static const chip = 8.0;
  static const card = 12.0;
  static const bubble = 18.0;
  static const bubbleTail = 4.0;
  static const sheet = 20.0;
}

abstract final class Motion {
  static const micro = Duration(milliseconds: 150);
  static const nav = Duration(milliseconds: 250);
  static const pulse = Duration(seconds: 2);

  static bool reduced(BuildContext context) => MediaQuery.disableAnimationsOf(context);
}

@immutable
class VlColors extends ThemeExtension<VlColors> {
  const VlColors({
    required this.bg,
    required this.surface,
    required this.surface2,
    required this.text,
    required this.text2,
    required this.accent,
    required this.onAccent,
    required this.bubbleOut,
    required this.onBubbleOut,
    required this.bubbleIn,
    required this.ok,
    required this.warn,
    required this.err,
    required this.border,
    required this.divider,
  });

  final Color bg, surface, surface2, text, text2, accent, onAccent;
  final Color bubbleOut, onBubbleOut, bubbleIn, ok, warn, err, border, divider;

  /// Contrast verified in Doc 16 §3 (all text ≥ 4.5:1, UI parts ≥ 3:1).
  /// `surface2` and `divider` are decorative only and never carry text alone.
  static const light = VlColors(
    bg: Color(0xFFF6F6F3),
    surface: Color(0xFFFFFFFF),
    surface2: Color(0xFFECECE8),
    text: Color(0xFF121417),
    text2: Color(0xFF565C64),
    accent: Color(0xFF1F4FD8),
    onAccent: Color(0xFFFFFFFF),
    bubbleOut: Color(0xFF1F4FD8),
    onBubbleOut: Color(0xFFFFFFFF),
    bubbleIn: Color(0xFFECECE8),
    ok: Color(0xFF1E7A46),
    warn: Color(0xFF9A5B00),
    err: Color(0xFFB3261E),
    border: Color(0xFF8A9099),
    divider: Color(0xFFE3E3DE),
  );

  static const dark = VlColors(
    bg: Color(0xFF0B0D10),
    surface: Color(0xFF16191E),
    surface2: Color(0xFF1E2228),
    text: Color(0xFFECEEF1),
    text2: Color(0xFFA4AAB3),
    accent: Color(0xFF8FA8FF),
    onAccent: Color(0xFF0B0D10),
    bubbleOut: Color(0xFF2D4FB8),
    onBubbleOut: Color(0xFFECEEF1),
    bubbleIn: Color(0xFF23272E),
    ok: Color(0xFF5FD394),
    warn: Color(0xFFF2B451),
    err: Color(0xFFFF8A80),
    border: Color(0xFF6B727C),
    divider: Color(0xFF262A31),
  );

  static VlColors of(BuildContext context) => Theme.of(context).extension<VlColors>()!;

  @override
  VlColors copyWith() => this;

  @override
  VlColors lerp(VlColors? other, double t) {
    if (other == null) return this;
    Color l(Color a, Color b) => Color.lerp(a, b, t)!;
    return VlColors(
      bg: l(bg, other.bg),
      surface: l(surface, other.surface),
      surface2: l(surface2, other.surface2),
      text: l(text, other.text),
      text2: l(text2, other.text2),
      accent: l(accent, other.accent),
      onAccent: l(onAccent, other.onAccent),
      bubbleOut: l(bubbleOut, other.bubbleOut),
      onBubbleOut: l(onBubbleOut, other.onBubbleOut),
      bubbleIn: l(bubbleIn, other.bubbleIn),
      ok: l(ok, other.ok),
      warn: l(warn, other.warn),
      err: l(err, other.err),
      border: l(border, other.border),
      divider: l(divider, other.divider),
    );
  }
}
