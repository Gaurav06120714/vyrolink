import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';
import '../onboarding/onboarding_flow.dart';

/// Locks the app on start and after 30 s in the background when App lock is
/// on. While locked, nothing of the app is visible or readable by a screen reader.
class AppLockGate extends StatefulWidget {
  const AppLockGate({super.key, required this.child});
  final Widget child;

  @override
  State<AppLockGate> createState() => _AppLockGateState();
}

class _AppLockGateState extends State<AppLockGate> with WidgetsBindingObserver {
  static const relockAfter = Duration(seconds: 30);
  late final AppServices _s = context.services;
  bool _locked = false;
  bool _busy = false;
  DateTime? _leftAt;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _s.appLock.addListener(_changed);
    if (_s.appLock.value) {
      _locked = true;
      WidgetsBinding.instance.addPostFrameCallback((_) => _unlock());
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _s.appLock.removeListener(_changed);
    super.dispose();
  }

  void _changed() {
    if (!_s.appLock.value && _locked) setState(() => _locked = false);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused || state == AppLifecycleState.hidden) {
      _leftAt ??= DateTime.now();
    } else if (state == AppLifecycleState.resumed) {
      final left = _leftAt;
      _leftAt = null;
      if (_s.appLock.value && !_busy && left != null && DateTime.now().difference(left) >= relockAfter) {
        setState(() => _locked = true);
        _unlock();
      }
    }
  }

  Future<void> _unlock() async {
    if (_busy) return;
    _busy = true;
    try {
      final ok = await _s.call((api) => api.authenticate('Unlock VYRO LINK'));
      if (ok && mounted) setState(() => _locked = false);
    } on VlFailure {
      // Stay locked; the button retries.
    } finally {
      _busy = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    return Stack(
      children: [
        ExcludeSemantics(
          excluding: _locked,
          child: TickerMode(enabled: !_locked, child: widget.child),
        ),
        if (_locked)
          Positioned.fill(
            child: Material(
              color: c.bg,
              child: SafeArea(
                child: Center(
                  child: Padding(
                    padding: const EdgeInsets.all(Space.xxl),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Emblem(icon: Icons.lock_outline_rounded, size: 120),
                        const SizedBox(height: Space.xl),
                        Text('VYRO LINK is locked', style: t.headlineSmall),
                        const SizedBox(height: Space.xl),
                        FilledButton(onPressed: _unlock, child: const Text('Unlock')),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class PrivacyScreen extends StatefulWidget {
  const PrivacyScreen({super.key});

  @override
  State<PrivacyScreen> createState() => _PrivacyScreenState();
}

class _PrivacyScreenState extends State<PrivacyScreen> {
  bool? _canAuth;
  bool _blockCapture = !kDebugMode;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final s = context.services;
    try {
      final can = await s.call((api) => api.canAuthenticate());
      final capture = await s.call((api) => api.setting(AppServices.screenCaptureKey));
      if (mounted) {
        setState(() {
          _canAuth = can;
          if (capture != null) _blockCapture = capture == '1';
        });
      }
    } on VlFailure {
      if (mounted) setState(() => _canAuth = false);
    }
  }

  Future<void> _setLock(bool on) async {
    final s = context.services;
    if (on) {
      final ok = await s.call((api) => api.authenticate('Turn on App lock'));
      if (!ok) return;
    }
    await s.call((api) => api.setSetting(AppServices.appLockKey, on ? '1' : null));
    s.appLock.value = on;
    if (mounted) setState(() {});
  }

  Future<void> _setCapture(bool on) async {
    final s = context.services;
    await s.call((api) => api.setSetting(AppServices.screenCaptureKey, on ? '1' : '0'));
    await s.call((api) => api.setScreenCaptureBlocked(on));
    if (mounted) setState(() => _blockCapture = on);
  }

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    final android = defaultTargetPlatform == TargetPlatform.android;
    return Scaffold(
      appBar: AppBar(title: const Text('Privacy')),
      body: ListView(
        padding: const EdgeInsets.only(bottom: Space.xxl),
        children: [
          const SectionHeader('Lock'),
          Section(
            rows: [
              SwitchListTile.adaptive(
                secondary: const Icon(Icons.lock_outline_rounded),
                title: const Text('App lock'),
                subtitle: Text(
                  _canAuth == false
                      ? 'Set a passcode on this phone first'
                      : 'Ask for Face ID, fingerprint or passcode when opening VYRO LINK',
                ),
                value: s.appLock.value,
                onChanged: _canAuth == true ? _setLock : null,
              ),
            ],
            footer:
                'Locks on opening and after 30 seconds in the background. Bluetooth relaying continues while locked.',
          ),
          const SectionHeader('Screen'),
          Section(
            rows: [
              if (android)
                SwitchListTile.adaptive(
                  secondary: const Icon(Icons.screenshot_monitor_outlined),
                  title: const Text('Block screenshots'),
                  subtitle: const Text('Also hides the app in the recent-apps view'),
                  value: _blockCapture,
                  onChanged: _setCapture,
                )
              else
                const ListTile(
                  leading: Icon(Icons.screenshot_monitor_outlined),
                  title: Text('Screenshots'),
                  subtitle: Text(
                    'iPhone does not let apps block screenshots. Anyone you message can still copy what they see.',
                  ),
                ),
            ],
          ),
          const SectionHeader('Notifications'),
          const Section(
            rows: [
              ListTile(
                leading: Icon(Icons.notifications_none_rounded),
                title: Text('Previews are always hidden'),
                subtitle: Text('Notifications only say "You have a new message", never who wrote it or what it says.'),
              ),
            ],
          ),
          const SectionHeader('Erase'),
          Section(
            rows: [
              ListTile(
                leading: Icon(Icons.local_fire_department_outlined, color: VlColors.of(context).err),
                title: Text('Panic wipe', style: TextStyle(color: VlColors.of(context).err)),
                trailing: const Icon(Icons.chevron_right_rounded),
                onTap: () => Routes.panicWipe(context),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
