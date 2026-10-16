import 'dart:async';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/services.dart';
import '../../design/tokens.dart';
import '../chats/chats_screen.dart';
import '../contacts/contacts_screen.dart';
import '../network/network_screen.dart';
import '../settings/settings_screen.dart';

/// Four top-level destinations (Doc 16 §6): bottom bar on phones, rail from 840 dp.
class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  static void showTab(BuildContext context, int index) =>
      context.findAncestorStateOfType<_HomeShellState>()?._select(index);

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> with WidgetsBindingObserver {
  int _index = 0;
  Timer? _poll;

  static const _destinations = [
    (Icons.chat_bubble_outline_rounded, Icons.chat_bubble_rounded, 'Chats'),
    (Icons.people_outline_rounded, Icons.people_rounded, 'Contacts'),
    (Icons.cell_tower_rounded, Icons.cell_tower_rounded, 'Network'),
    (Icons.settings_outlined, Icons.settings_rounded, 'Settings'),
  ];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _startPolling();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _poll?.cancel();
    super.dispose();
  }

  /// Network counters change without data events (scanning, radio state), so
  /// refresh them every few seconds while the app is in front.
  void _startPolling() {
    _poll?.cancel();
    _poll = Timer.periodic(const Duration(seconds: 5), (_) => context.services.refreshNetwork());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      context.services
        ..refreshNetwork()
        ..bump();
      _startPolling();
    } else if (state == AppLifecycleState.paused) {
      _poll?.cancel();
    }
  }

  void _select(int i) => setState(() => _index = i);

  @override
  Widget build(BuildContext context) {
    final wide = MediaQuery.sizeOf(context).width >= 840;
    final body = IndexedStack(
      index: _index,
      children: const [ChatsScreen(), ContactsScreen(), NetworkScreen(), SettingsScreen()],
    );
    if (wide) {
      return Scaffold(
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: _index,
              onDestinationSelected: _select,
              labelType: NavigationRailLabelType.all,
              destinations: [
                for (final (icon, selected, label) in _destinations)
                  NavigationRailDestination(icon: Icon(icon), selectedIcon: Icon(selected), label: Text(label)),
              ],
            ),
            const VerticalDivider(width: 1),
            Expanded(child: body),
          ],
        ),
      );
    }
    return Scaffold(
      body: body,
      bottomNavigationBar: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(top: BorderSide(color: VlColors.of(context).divider)),
        ),
        child: NavigationBar(
          selectedIndex: _index,
          onDestinationSelected: _select,
          destinations: [
            for (final (icon, selected, label) in _destinations)
              NavigationDestination(icon: Icon(icon), selectedIcon: Icon(selected), label: label),
          ],
        ),
      ),
    );
  }
}

/// Relay state in the Chats header (Doc 16 §6). Tapping opens Network.
class NetworkChip extends StatelessWidget {
  const NetworkChip({super.key});

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return ValueListenableBuilder<NetworkStatus?>(
      valueListenable: context.services.network,
      builder: (context, n, _) {
        final (label, tone, live) = describeRelay(n, c);
        return Semantics(
          button: true,
          label: 'Network: $label',
          child: InkWell(
            borderRadius: BorderRadius.circular(Radii.chip),
            onTap: () => HomeShell.showTab(context, 2),
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 48),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: Space.s),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _Pulse(color: tone, live: live),
                    const SizedBox(width: Space.s),
                    ExcludeSemantics(
                      child: Text(label, style: Theme.of(context).textTheme.labelMedium!.copyWith(color: tone)),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

/// (label, colour, animate?) for the current relay state.
(String, Color, bool) describeRelay(NetworkStatus? n, VlColors c) {
  if (n == null) return ('Starting', c.text2, false);
  if (!n.relayEnabled) return ('Relay off', c.text2, false);
  return switch (n.radio) {
    RadioState.poweredOff => ('Bluetooth off', c.err, false),
    RadioState.unauthorized => ('Permission needed', c.err, false),
    RadioState.unsupported => ('No Bluetooth', c.err, false),
    RadioState.unknown => ('Starting', c.text2, false),
    RadioState.poweredOn => (n.openLinks > 0 ? 'Exchanging' : 'Relaying', c.ok, true),
  };
}

class _Pulse extends StatefulWidget {
  const _Pulse({required this.color, required this.live});
  final Color color;
  final bool live;

  @override
  State<_Pulse> createState() => _PulseState();
}

class _PulseState extends State<_Pulse> with SingleTickerProviderStateMixin {
  late final _c = AnimationController(vsync: this, duration: Motion.pulse);

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _sync();
  }

  @override
  void didUpdateWidget(_Pulse old) {
    super.didUpdateWidget(old);
    _sync();
  }

  void _sync() {
    if (widget.live && !Motion.reduced(context)) {
      if (!_c.isAnimating) _c.repeat(reverse: true);
    } else {
      _c
        ..stop()
        ..value = 1;
    }
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => FadeTransition(
    opacity: Tween(begin: 0.35, end: 1.0).animate(CurvedAnimation(parent: _c, curve: Curves.easeInOut)),
    child: Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(color: widget.color, shape: BoxShape.circle),
    ),
  );
}
