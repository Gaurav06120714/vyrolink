import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../design/theme.dart';
import '../design/tokens.dart';
import '../design/widgets/states.dart';
import '../features/home/home_shell.dart';
import '../features/onboarding/onboarding_flow.dart';
import '../features/settings/privacy_screen.dart';
import 'failure.dart';
import 'services.dart';

class VyroApp extends StatelessWidget {
  const VyroApp({super.key, required this.services});

  final AppServices services;

  @override
  Widget build(BuildContext context) {
    final platform = Theme.of(context).platform;
    return AppScope(
      services: services,
      child: MaterialApp(
        title: 'VYRO LINK',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light, platform),
        darkTheme: buildTheme(Brightness.dark, platform),
        home: const RootGate(),
      ),
    );
  }
}

enum _Phase { starting, failed, onboarding, ready }

/// Opens the node and chooses between onboarding and the main app. Panic wipe
/// and "reset" come back here through [AppServices.restartApp].
class RootGate extends StatefulWidget {
  const RootGate({super.key});

  @override
  State<RootGate> createState() => _RootGateState();
}

class _RootGateState extends State<RootGate> {
  _Phase _phase = _Phase.starting;
  VlFailure? _failure;

  @override
  void initState() {
    super.initState();
    context.services.restartApp = _start;
    _start();
  }

  Future<void> _start() async {
    final s = context.services;
    setState(() => _phase = _Phase.starting);
    try {
      s.startup = await s.call((api) => api.start());
      s.profileName = s.startup.hasIdentity ? await s.call((api) => api.setting(AppServices.profileNameKey)) ?? '' : '';
      s.appLock.value = s.startup.hasIdentity && await s.call((api) => api.setting(AppServices.appLockKey)) == '1';
      final capture = s.startup.hasIdentity ? await s.call((api) => api.setting(AppServices.screenCaptureKey)) : null;
      if (capture != null) await s.call((api) => api.setScreenCaptureBlocked(capture == '1'));
      await s.refreshNetwork();
      s.bump();
      setState(() => _phase = s.startup.hasIdentity ? _Phase.ready : _Phase.onboarding);
    } on VlFailure catch (e) {
      setState(() {
        _failure = e;
        _phase = _Phase.failed;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return AnnotatedRegion<SystemUiOverlayStyle>(
      value: Theme.of(context).brightness == Brightness.dark ? SystemUiOverlayStyle.light : SystemUiOverlayStyle.dark,
      child: AnimatedSwitcher(
        duration: Motion.reduced(context) ? Duration.zero : Motion.nav,
        child: switch (_phase) {
          _Phase.starting => ColoredBox(key: const ValueKey('s'), color: c.bg, child: const SizedBox.expand()),
          _Phase.failed => _StartFailure(failure: _failure!, onRetry: _start),
          _Phase.onboarding => OnboardingFlow(key: const ValueKey('o'), onDone: _start),
          _Phase.ready => const AppLockGate(key: ValueKey('h'), child: HomeShell()),
        },
      ),
    );
  }
}

/// The encrypted store could not be opened (no passcode, keys lost, damaged
/// file). Offers retry, and a reset that destroys the keys and the files.
class _StartFailure extends StatelessWidget {
  const _StartFailure({required this.failure, required this.onRetry});

  final VlFailure failure;
  final VoidCallback onRetry;

  Future<void> _reset(BuildContext context) async {
    final ok = await showAdaptiveDialog<bool>(
      context: context,
      builder: (c) => AlertDialog.adaptive(
        title: const Text('Reset VYRO LINK on this phone?'),
        content: const Text('Your identity, contacts and messages on this phone are destroyed. This cannot be undone.'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(c, false), child: const Text('Cancel')),
          TextButton(
            onPressed: () => Navigator.pop(c, true),
            style: TextButton.styleFrom(foregroundColor: VlColors.of(c).err),
            child: const Text('Reset'),
          ),
        ],
      ),
    );
    if (ok != true || !context.mounted) return;
    try {
      await context.services.call((api) => api.panicWipe());
      onRetry();
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    body: SafeArea(
      child: EmptyState(
        icon: Icons.lock_outline_rounded,
        title: 'VYRO LINK could not open its encrypted storage',
        body: failure.message,
        action: Column(
          children: [
            FilledButton(onPressed: onRetry, child: const Text('Try again')),
            const SizedBox(height: Space.s),
            TextButton(onPressed: () => _reset(context), child: const Text('Reset this phone')),
          ],
        ),
      ),
    ),
  );
}
