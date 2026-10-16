import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../tokens.dart';

/// How each delivery state is presented (Doc 18 §7.4, Doc 16 §7). Only
/// [DeliveryState.delivered] and [DeliveryState.read] use the "ok" colour;
/// relay custody is never styled like delivery.
class DeliveryInfo {
  const DeliveryInfo(this.icon, this.label, this.explanation, this.tone);

  final IconData icon;
  final String label;
  final String explanation;
  final Color Function(VlColors) tone;

  static DeliveryInfo of(DeliveryState s) => switch (s) {
    DeliveryState.queued || DeliveryState.sentToNearby => DeliveryInfo(
      Icons.schedule_rounded,
      'Waiting for nearby devices',
      'Encrypted and saved on this phone. It moves when another phone with VYRO LINK comes near.',
      (c) => c.text2,
    ),
    DeliveryState.acceptedByRelay => DeliveryInfo(
      Icons.swap_horiz_rounded,
      'Accepted by a relay',
      'A nearby phone took an encrypted copy to pass on. The recipient may not have it yet.',
      (c) => c.text2,
    ),
    DeliveryState.delivered => DeliveryInfo(
      Icons.check_circle_rounded,
      'Delivered',
      "The recipient's phone confirmed it with an authenticated receipt.",
      (c) => c.ok,
    ),
    DeliveryState.read => DeliveryInfo(Icons.done_all_rounded, 'Read', 'The recipient opened it.', (c) => c.ok),
    DeliveryState.expired => DeliveryInfo(
      Icons.error_outline_rounded,
      'Not delivered (expired)',
      'No nearby phone took a copy before it expired.',
      (c) => c.err,
    ),
    DeliveryState.deliveryUnknown => DeliveryInfo(
      Icons.help_outline_rounded,
      'Delivery unknown',
      'Copies were handed on but no receipt came back before they expired. It may have arrived.',
      (c) => c.warn,
    ),
    DeliveryState.failed => DeliveryInfo(
      Icons.error_rounded,
      'Failed',
      'This phone could not encrypt or store the message.',
      (c) => c.err,
    ),
    DeliveryState.received => DeliveryInfo(
      Icons.call_received_rounded,
      'Received',
      'You received this message.',
      (c) => c.text2,
    ),
  };
}

/// State icon with a cross-fade when it changes (no bounce; Doc 16 §5).
class DeliveryGlyph extends StatelessWidget {
  const DeliveryGlyph({super.key, required this.state, this.size = 14, this.color});

  final DeliveryState state;
  final double size;

  /// Overrides the tone, e.g. on a coloured bubble.
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final info = DeliveryInfo.of(state);
    return Semantics(
      label: info.label,
      child: AnimatedSwitcher(
        duration: Motion.reduced(context) ? Duration.zero : Motion.micro,
        child: Icon(info.icon, key: ValueKey(state), size: size, color: color ?? info.tone(VlColors.of(context))),
      ),
    );
  }
}
