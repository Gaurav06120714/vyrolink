import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/google_account.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';
import '../home/home_shell.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  PermissionState? _camera;
  Uint8List? _fingerprint;
  String? _googleEmail;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    final s = context.services;
    try {
      final camera = await s.call((api) => api.cameraPermission());
      final fp = await s.call((api) => api.myFingerprint());
      final google = await s.call((api) => api.setting(GoogleAccount.emailKey));
      if (mounted) {
        setState(() {
          _camera = camera;
          _fingerprint = fp;
          _googleEmail = google;
        });
      }
    } on VlFailure {
      // Shown as unknown.
    }
  }

  Future<void> _google() async {
    final s = context.services;
    if (_googleEmail != null) {
      final ok = await showAdaptiveDialog<bool>(
        context: context,
        builder: (d) => AlertDialog.adaptive(
          title: const Text('Sign out of Google?'),
          content: const Text('Your Google name and email are removed from this phone. Your chats and ID stay.'),
          actions: [
            TextButton(onPressed: () => Navigator.pop(d, false), child: const Text('Cancel')),
            TextButton(onPressed: () => Navigator.pop(d, true), child: const Text('Sign out')),
          ],
        ),
      );
      if (ok != true) return;
      await GoogleAccount.signOut();
      await s.mutate((api) async {
        await api.setSetting(GoogleAccount.emailKey, null);
        await api.setSetting(GoogleAccount.nameKey, null);
      });
      if (mounted) setState(() => _googleEmail = null);
      return;
    }
    try {
      final g = await GoogleAccount.signIn();
      if (g == null) return;
      await s.mutate((api) async {
        await api.setSetting(GoogleAccount.emailKey, g.email);
        await api.setSetting(GoogleAccount.nameKey, g.name);
      });
      if (mounted) setState(() => _googleEmail = g.email);
    } on GoogleAccountFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
  }

  Future<void> _editName() async {
    final s = context.services;
    final name = await askName(
      context,
      title: 'Your name',
      initial: s.profileName,
      note: 'Used in codes you show from now on. People you already paired with keep the name they saw.',
    );
    if (name == null) return;
    await s.call((api) => api.setSetting(AppServices.profileNameKey, name));
    setState(() => s.profileName = name);
  }

  Future<void> _notifications() async {
    final s = context.services;
    final state = await s.call((api) => api.requestNotificationPermission());
    if (!mounted) return;
    if (state == PermissionState.granted) {
      showMessage(context, 'Notifications are on. They never show the sender or the text.');
    } else {
      showMessage(context, 'Notifications are off. You can change this in Settings.');
    }
  }

  String _permissionLabel(PermissionState? p) => switch (p) {
    PermissionState.granted => 'Allowed',
    PermissionState.denied => 'Off',
    PermissionState.restricted => 'Restricted',
    PermissionState.notDetermined => 'Not asked yet',
    null => '—',
  };

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final name = s.profileName.isEmpty ? 'No name set' : s.profileName;
    return Scaffold(
      body: CustomScrollView(
        slivers: [
          const SliverAppBar.large(title: Text('Settings')),
          SliverList.list(
            children: [
              Section(
                rows: [
                  ListTile(
                    contentPadding: const EdgeInsets.symmetric(horizontal: Space.l, vertical: Space.s),
                    leading: Avatar(name: name, seed: _fingerprint ?? Uint8List(1), size: 52),
                    title: Text(name, style: t.titleMedium),
                    subtitle: const Text('Shared only with people you pair with'),
                    trailing: const Icon(Icons.edit_outlined),
                    onTap: _editName,
                  ),
                  ListTile(
                    leading: const Icon(Icons.badge_outlined),
                    title: const Text('My ID'),
                    subtitle: const Text('Your unique 32-character code and its QR'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => Routes.myId(context),
                  ),
                  ListTile(
                    leading: const Icon(Icons.account_circle_outlined),
                    title: Text(_googleEmail ?? 'Sign in with Google'),
                    subtitle: Text(
                      _googleEmail == null
                          ? 'Optional. VYRO LINK works fully without it.'
                          : 'Google account · kept on this phone only',
                    ),
                    trailing: _googleEmail == null ? const Icon(Icons.chevron_right_rounded) : const Text('Sign out'),
                    onTap: _google,
                  ),
                ],
              ),
              const SectionHeader('Messaging'),
              Section(
                rows: [
                  ListTile(
                    leading: const Icon(Icons.cell_tower_rounded),
                    title: const Text('Bluetooth relay'),
                    trailing: ValueListenableBuilder<NetworkStatus?>(
                      valueListenable: s.network,
                      builder: (_, n, _) => Text(
                        (n?.relayEnabled ?? false) ? 'On' : 'Off',
                        style: t.bodyMedium!.copyWith(color: c.text2),
                      ),
                    ),
                    onTap: () => HomeShell.showTab(context, 2),
                  ),
                  ListTile(
                    leading: const Icon(Icons.notifications_none_rounded),
                    title: const Text('Notifications'),
                    subtitle: const Text('Only "You have a new message". No names, no text.'),
                    onTap: _notifications,
                  ),
                ],
              ),
              const SectionHeader('Permissions'),
              Section(
                rows: [
                  ValueListenableBuilder<NetworkStatus?>(
                    valueListenable: s.network,
                    builder: (_, n, _) => ListTile(
                      leading: const Icon(Icons.bluetooth_rounded),
                      title: const Text('Bluetooth'),
                      subtitle: const Text('Passes encrypted messages to nearby phones'),
                      trailing: Text(switch (n?.radio) {
                        RadioState.poweredOn => 'Allowed',
                        RadioState.unauthorized => 'Off',
                        RadioState.poweredOff => 'Bluetooth off',
                        RadioState.unsupported => 'Unavailable',
                        _ => 'Not asked yet',
                      }, style: t.bodyMedium!.copyWith(color: c.text2)),
                    ),
                  ),
                  ListTile(
                    leading: const Icon(Icons.photo_camera_outlined),
                    title: const Text('Camera'),
                    subtitle: const Text('Only to scan codes when adding a contact'),
                    trailing: Text(_permissionLabel(_camera), style: t.bodyMedium!.copyWith(color: c.text2)),
                  ),
                  ListTile(
                    leading: const Icon(Icons.open_in_new_rounded),
                    title: const Text('Open system settings'),
                    onTap: () => s.call((api) => api.openAppSettings()),
                  ),
                ],
                footer:
                    'VYRO LINK never asks for your contacts, microphone or photo library. It asks for location only '
                    'when you tap Share location, and sends it once, encrypted. The photo picker hands over only the '
                    'photo you choose.',
              ),
              const SectionHeader('Privacy and security'),
              Section(
                rows: [
                  ListTile(
                    leading: const Icon(Icons.shield_outlined),
                    title: const Text('How VYRO LINK protects you'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => Routes.about(context),
                  ),
                  ListTile(
                    leading: const Icon(Icons.lock_outline_rounded),
                    title: const Text('Privacy'),
                    subtitle: const Text('App lock, screenshots, notifications'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => Routes.privacy(context),
                  ),
                  ListTile(
                    leading: Icon(Icons.local_fire_department_outlined, color: c.err),
                    title: Text('Panic wipe', style: TextStyle(color: c.err)),
                    subtitle: const Text('Destroy keys and erase everything on this phone'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => Routes.panicWipe(context),
                  ),
                ],
              ),
              if (kDebugMode) ...[
                const SectionHeader('Development'),
                Section(
                  rows: [
                    ListTile(
                      leading: const Icon(Icons.science_outlined),
                      title: const Text('Developer tools'),
                      subtitle: const Text('Debug builds only'),
                      trailing: const Icon(Icons.chevron_right_rounded),
                      onTap: () => Routes.developer(context),
                    ),
                  ],
                ),
              ],
              const SizedBox(height: Space.xxl),
            ],
          ),
        ],
      ),
    );
  }
}
