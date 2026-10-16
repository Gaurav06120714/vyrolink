import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../theme.dart';
import '../tokens.dart';

/// Draws a QR symbol black on white with a 4-module quiet zone, regardless of
/// theme, so any camera can read it (Doc 16 §7 QR panel).
class QrPanel extends StatelessWidget {
  const QrPanel({super.key, required this.code, required this.semanticLabel});

  final QrCode code;
  final String semanticLabel;

  @override
  Widget build(BuildContext context) => Semantics(
    label: semanticLabel,
    image: true,
    child: Container(
      decoration: BoxDecoration(color: Colors.white, borderRadius: BorderRadius.circular(Radii.sheet)),
      padding: const EdgeInsets.all(Space.m),
      child: AspectRatio(aspectRatio: 1, child: CustomPaint(painter: _QrPainter(code))),
    ),
  );
}

class _QrPainter extends CustomPainter {
  _QrPainter(this.code);
  final QrCode code;

  @override
  void paint(Canvas canvas, Size size) {
    const quiet = 4;
    final n = code.size;
    final cell = size.width / (n + quiet * 2);
    final paint = Paint()
      ..color = Colors.black
      ..isAntiAlias = false;
    final path = Path();
    for (var y = 0; y < n; y++) {
      for (var x = 0; x < n; x++) {
        if (code.modules[y * n + x] == 1) {
          path.addRect(Rect.fromLTWH((x + quiet) * cell, (y + quiet) * cell, cell + 0.5, cell + 0.5));
        }
      }
    }
    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(_QrPainter old) => old.code != code;
}

/// 60 digits as 12 blocks of 5 (Doc 16 §7), selectable, read digit by digit.
class SafetyNumber extends StatelessWidget {
  const SafetyNumber({super.key, required this.number});
  final String number;

  @override
  Widget build(BuildContext context) {
    final digits = number.replaceAll(RegExp(r'\s'), '');
    final blocks = [for (var i = 0; i < digits.length; i += 5) digits.substring(i, (i + 5).clamp(0, digits.length))];
    return Semantics(
      label: 'Safety number: ${blocks.map((b) => b.split('').join(' ')).join(', ')}',
      child: ExcludeSemantics(
        child: Container(
          padding: const EdgeInsets.all(Space.l),
          decoration: BoxDecoration(
            color: VlColors.of(context).surface2,
            borderRadius: BorderRadius.circular(Radii.card),
          ),
          child: SelectableText(
            [for (var i = 0; i < blocks.length; i += 4) blocks.skip(i).take(4).join('  ')].join('\n'),
            textAlign: TextAlign.center,
            style: monoStyle(context).copyWith(fontSize: 17, height: 1.6),
          ),
        ),
      ),
    );
  }
}

/// A 32-byte key as eight groups of hex, for the About screen.
String fingerprintText(Uint8List key) {
  final hex = key.map((b) => b.toRadixString(16).padLeft(2, '0')).join().toUpperCase();
  return [for (var i = 0; i < hex.length; i += 8) hex.substring(i, i + 8)].join(' ');
}
