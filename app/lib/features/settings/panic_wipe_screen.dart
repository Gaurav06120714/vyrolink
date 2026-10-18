import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/failure.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';
import '../onboarding/onboarding_flow.dart';

/// Panic wipe: the host destroys both database keys in the Keychain/Keystore
/// first (cryptographic erasure), then deletes the encrypted files. Success is
/// shown only after the host reports both steps done.
class PanicWipeScreen extends StatefulWidget {
  const PanicWipeScreen({super.key});

  @override
  State<PanicWipeScreen> createState() => _PanicWipeScreenState();
}

class _PanicWipeScreenState extends State<PanicWipeScreen> {
  bool _busy = false;
  VlFailure? _failure;

  Future<void> _wipe() async {
    final c = VlColors.of(context);
    final ok = await showAdaptiveDialog<bool>(
      context: context,
      builder: (d) => AlertDialog.adaptive(
        title: const Text('Erase everything now?'),
        content: const Text(
          'Your identity, contacts, messages and the mail you carry for others are destroyed. Contacts must pair with you again. This cannot be undone.',
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancel')),
          TextButton(
            onPressed: () => Navigator.pop(d, true),
            style: TextButton.styleFrom(foregroundColor: c.err),
            child: const Text('Erase'),
          ),
        ],
      ),
    );
    if (ok != true || !mounted) return;
    setState(() {
      _busy = true;
      _failure = null;
    });
    final s = context.services;
    try {
      await s.panicWipe();
      HapticFeedback.heavyImpact();
      if (!mounted) return;
      Navigator.of(context).popUntil((r) => r.isFirst);
      s.restartApp();
    } on VlFailure catch (e) {
      if (mounted) {
        setState(() {
          _failure = e;
          _busy = false;
        });
      }
    }
  }

  String _failureText(VlFailure f) => switch (f.code) {
    VlErrorCode.keyStore => 'The keys could not be deleted, so nothing was erased. Your data is unchanged. Try again, or restart the phone and try again.',
    VlErrorCode.storage => 'The keys were destroyed, so the remaining files can no longer be decrypted, but some files could not be removed. Restart VYRO LINK; it will start fresh.',
    _ => 'The wipe did not finish: ${f.message}',
  };

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Panic wipe')),
      body: ListView(
        padding: EdgeInsets.all(Space.gutter(context)),
        children: [
          const SizedBox(height: Space.l),
          const Center(child: Emblem(icon: Icons.local_fire_department_outlined, size: 120)),
          const SizedBox(height: Space.xl),
          Text('Erase VYRO LINK from this phone', style: t.headlineSmall, textAlign: TextAlign.center),
          const SizedBox(height: Space.m),
          Text(
            'The encryption keys are destroyed first, so the stored data can never be decrypted again, even if parts of it remain on the flash memory. Then the encrypted files are deleted.',
            style: t.bodyMedium!.copyWith(color: c.text2),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: Space.l),
          Text(
            'Copies of your messages on other people\'s phones are not affected.',
            style: t.bodySmall,
            textAlign: TextAlign.center,
          ),
          if (_failure != null) ...[
            const SizedBox(height: Space.l),
            InfoBanner(kind: BannerKind.err, text: _failureText(_failure!)),
          ],
          const SizedBox(height: Space.xxl),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: c.err, foregroundColor: c.onAccent),
            onPressed: _busy ? null : _wipe,
            child: _busy
                ? const SizedBox.square(dimension: 20, child: CircularProgressIndicator.adaptive(strokeWidth: 2))
                : const Text('Erase everything'),
          ),
        ],
      ),
    );
  }
}
