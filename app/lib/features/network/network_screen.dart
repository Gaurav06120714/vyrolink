import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';
import '../home/home_shell.dart';

/// Doc 16 screens 6, 9 and 12: whether this phone is relaying, counters only
/// (never who is nearby), and the platform's limits stated plainly.
class NetworkScreen extends StatelessWidget {
  const NetworkScreen({super.key});

  Future<void> _toggle(BuildContext context, bool on) async {
    final s = context.services;
    try {
      await s.call((api) => api.setRelayEnabled(on));
    } on VlFailure catch (e) {
      if (context.mounted) showMessage(context, e.message);
    }
    await s.refreshNetwork();
  }

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final ios = defaultTargetPlatform == TargetPlatform.iOS;
    return Scaffold(
      body: ValueListenableBuilder<NetworkStatus?>(
        valueListenable: s.network,
        builder: (context, n, _) {
          final (label, tone, _) = describeRelay(n, c);
          final last = n?.lastExchangeSecs;
          return CustomScrollView(
            slivers: [
              const SliverAppBar.large(title: Text('Network')),
              SliverList.list(
                children: [
                  Padding(
                    padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
                    child: Container(
                      padding: const EdgeInsets.all(Space.l),
                      decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
                      child: Row(
                        children: [
                          Container(
                            width: 48,
                            height: 48,
                            decoration: BoxDecoration(color: tone.withValues(alpha: 0.12), shape: BoxShape.circle),
                            child: Icon(Icons.cell_tower_rounded, color: tone),
                          ),
                          const SizedBox(width: Space.l),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(label, style: t.titleMedium!.copyWith(color: tone)),
                                const SizedBox(height: 2),
                                Text(
                                  last == null
                                      ? 'No exchange since VYRO LINK started'
                                      : 'Last exchange ${shortStamp(context, last)}',
                                  style: t.bodySmall,
                                ),
                              ],
                            ),
                          ),
                          Switch.adaptive(
                            value: n?.relayEnabled ?? false,
                            onChanged: n == null ? null : (v) => _toggle(context, v),
                          ),
                        ],
                      ),
                    ),
                  ),
                  if (n != null && n.relayEnabled && n.radio == RadioState.unauthorized)
                    InfoBanner(
                      kind: BannerKind.err,
                      text: 'Bluetooth permission is off for VYRO LINK.',
                      action: 'Open Settings',
                      onAction: () => s.call((api) => api.openAppSettings()),
                    ),
                  if (n != null && n.relayEnabled && n.radio == RadioState.poweredOff)
                    const InfoBanner(kind: BannerKind.err, text: 'Bluetooth is turned off on this phone.'),
                  const SectionHeader('Activity since VYRO LINK started'),
                  Section(
                    rows: [
                      _Stat(
                        icon: Icons.groups_2_outlined,
                        label: 'Phones met in the last day',
                        value: nearbyBucket(n?.devicesSeenRecently ?? 0),
                      ),
                      _Stat(
                        icon: Icons.handshake_outlined,
                        label: 'Exchanges with nearby phones',
                        value: '${n?.encountersSinceLaunch ?? 0}',
                      ),
                      _Stat(icon: Icons.link_rounded, label: 'Open connections', value: '${n?.openLinks ?? 0}'),
                      _Stat(
                        icon: Icons.radar_rounded,
                        label: 'Looking for phones',
                        value: (n?.scanning ?? false) ? 'Yes' : 'No',
                      ),
                      _Stat(
                        icon: Icons.podcasts_rounded,
                        label: 'Visible to nearby phones',
                        value: (n?.advertising ?? false) ? 'Yes' : 'No',
                      ),
                    ],
                    footer: 'VYRO LINK never shows who is nearby. Phones advertise only a fixed VYRO LINK signal with no name or identity.',
                  ),
                  const SectionHeader('Messages on this phone'),
                  Section(
                    rows: [
                      ListTile(
                        leading: const Icon(Icons.outbox_outlined),
                        title: const Text('Waiting to be delivered'),
                        subtitle: const Text('Your messages without a delivery receipt yet'),
                        trailing: const Icon(Icons.chevron_right_rounded),
                        onTap: () => Routes.queue(context),
                      ),
                      _Stat(
                        icon: Icons.inventory_2_outlined,
                        label: 'Carrying for others',
                        value: '${n?.carriedEnvelopes ?? 0} · ${formatBytes(n?.carriedBytes ?? 0)}',
                      ),
                    ],
                    footer: 'Messages you carry for others are encrypted for their recipients. This phone cannot read them, and they are deleted when they expire or are delivered.',
                  ),
                  const SectionHeader('Relay settings'),
                  Section(
                    rows: [
                      ListTile(
                        leading: const Icon(Icons.tune_rounded),
                        title: const Text('Relay mode and storage'),
                        subtitle: Text(
                          ios
                              ? 'How much mail you carry for others'
                              : 'When to relay, and how much mail you carry for others',
                        ),
                        trailing: const Icon(Icons.chevron_right_rounded),
                        onTap: () =>
                            Navigator.of(context)
                                .push(MaterialPageRoute<void>(builder: (_) => const RelaySettingsScreen())),
                      ),
                      ListTile(
                        leading: const Icon(Icons.ios_share_rounded),
                        title: const Text('Export diagnostics'),
                        subtitle: const Text('Counters only: no names, IDs, keys or messages'),
                        onTap: () => _export(context, n),
                      ),
                    ],
                  ),
                  const SectionHeader('What to expect'),
                  Section(
                    rows: [
                      _Note(
                        icon: Icons.phone_iphone_rounded,
                        text: ios
                            ? 'iPhone limits Bluetooth in the background: scanning slows down and is paused when you force-quit VYRO LINK. Keep it open (or in the background, not swiped away) to relay.'
                            : 'Android may stop Bluetooth work in the background to save battery. Messages move most reliably while VYRO LINK is open.',
                      ),
                      const _Note(
                        icon: Icons.schedule_rounded,
                        text: 'Delivery depends on people meeting. A message can take minutes or days, and it expires after about a week.',
                      ),
                      const _Note(
                        icon: Icons.wifi_off_rounded,
                        text: 'No internet is used. Being offline is normal here.',
                      ),
                    ],
                  ),
                  const SizedBox(height: Space.xxl),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}

/// Anonymous count bucket (Doc 16 screen 6): never who, never how many exactly.
String nearbyBucket(int n) => switch (n) {
  0 => 'None',
  <= 2 => '1–2',
  <= 9 => '3–9',
  _ => '10+',
};

/// A plain-text report the user shares on purpose: counters and versions,
/// nothing that identifies a person, a contact or a message.
String diagnosticsReport(NetworkStatus? n, String platform, DateTime now) => [
  'VYRO LINK diagnostics',
  'Generated: ${now.toUtc().toIso8601String()}',
  'App: 0.1.0 (development) · protocol wire v1, link v1',
  'Platform: $platform',
  'Bluetooth: ${n?.radio.name ?? 'unknown'}',
  'Relay enabled: ${n?.relayEnabled ?? false} (mode ${n?.relayMode ?? 0}, charging ${n?.charging ?? false})',
  'Scanning: ${n?.scanning ?? false} · advertising: ${n?.advertising ?? false}',
  'Open links: ${n?.openLinks ?? 0}',
  'Exchanges since start: ${n?.encountersSinceLaunch ?? 0}',
  'Phones met in last day: ${nearbyBucket(n?.devicesSeenRecently ?? 0)}',
  'Own envelopes stored: ${n?.ownEnvelopes ?? 0}',
  'Carried for others: ${n?.carriedEnvelopes ?? 0} envelopes, ${n?.carriedBytes ?? 0} bytes',
].join('\n');

Future<void> _export(BuildContext context, NetworkStatus? n) async {
  final text = diagnosticsReport(n, defaultTargetPlatform.name, DateTime.now());
  try {
    await context.services.call(
      (api) => api.shareFile('vyrolink-diagnostics.txt', Uint8List.fromList(utf8.encode(text))),
    );
  } on VlFailure catch (e) {
    if (context.mounted) showMessage(context, e.message);
  }
}

class RelaySettingsScreen extends StatefulWidget {
  const RelaySettingsScreen({super.key});

  @override
  State<RelaySettingsScreen> createState() => _RelaySettingsScreenState();
}

class _RelaySettingsScreenState extends State<RelaySettingsScreen> {
  double? _capMb;

  @override
  void initState() {
    super.initState();
    context.services.call((api) => api.carryCapMb()).then((mb) {
      if (mounted) setState(() => _capMb = mb.toDouble());
    }).ignore();
  }

  Future<void> _mode(int mode) async {
    final s = context.services;
    await s.call((api) => api.setRelayEnabled(mode != 0));
    if (mode != 0) await s.call((api) => api.setRelayMode(mode));
    await s.refreshNetwork();
  }

  @override
  Widget build(BuildContext context) {
    final s = context.services;
    final t = Theme.of(context).textTheme;
    final android = defaultTargetPlatform == TargetPlatform.android;
    return Scaffold(
      appBar: AppBar(title: const Text('Relay settings')),
      body: ValueListenableBuilder<NetworkStatus?>(
        valueListenable: s.network,
        builder: (context, n, _) {
          final current = (n == null || !n.relayEnabled) ? 0 : (android ? n.relayMode : 2);
          return ListView(
            padding: const EdgeInsets.only(bottom: Space.xxl),
            children: [
              const SectionHeader('Relay'),
              Padding(
                padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
                child: SegmentedButton<int>(
                  segments: [
                    const ButtonSegment(value: 0, label: Text('Off')),
                    if (android) const ButtonSegment(value: 1, label: Text('While charging')),
                    ButtonSegment(value: 2, label: Text(android ? 'Always' : 'On')),
                  ],
                  selected: {current},
                  showSelectedIcon: false,
                  onSelectionChanged: (v) => _mode(v.first),
                ),
              ),
              Padding(
                padding: EdgeInsets.fromLTRB(Space.gutter(context), Space.m, Space.gutter(context), 0),
                child: Text(
                  android
                      ? switch (current) {
                          0 => 'Your own messages wait on this phone until you turn the relay on.',
                          1 =>
                            'Relays only while the phone is charging, so it does not drain the battery during the day.',
                          _ => 'Relays in the background too. Android shows a small "relaying" notification while it runs.',
                        }
                      : current == 0
                      ? 'Your own messages wait on this phone until you turn the relay on.'
                      : 'iPhone relays while VYRO LINK is open, and in the background when iOS allows it. Force-quitting stops it.',
                  style: t.bodySmall,
                ),
              ),
              const SectionHeader('Storage for others\' mail'),
              if (_capMb != null) ...[
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: Space.gutter(context) - Space.s),
                  child: Slider.adaptive(
                    min: 1,
                    max: 100,
                    divisions: 99,
                    value: _capMb!.clamp(1, 100),
                    label: '${_capMb!.round()} MB',
                    semanticFormatterCallback: (v) => '${v.round()} megabytes',
                    onChanged: (v) => setState(() => _capMb = v),
                    onChangeEnd: (v) => s.call((api) => api.setCarryCapMb(v.round())),
                  ),
                ),
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
                  child: Text(
                    'Up to ${_capMb!.round()} MB of encrypted mail for other people. When full, the mail closest to expiry is dropped first. '
                    'Battery cost on this phone has not been measured yet.',
                    style: t.bodySmall,
                  ),
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _Stat extends StatelessWidget {
  const _Stat({required this.icon, required this.label, required this.value});
  final IconData icon;
  final String label, value;

  @override
  Widget build(BuildContext context) => ListTile(
    leading: Icon(icon),
    title: Text(label),
    trailing: Text(value, style: Theme.of(context).textTheme.bodyMedium!.copyWith(color: VlColors.of(context).text2)),
  );
}

class _Note extends StatelessWidget {
  const _Note({required this.icon, required this.text});
  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) => ListTile(
    leading: Icon(icon),
    title: Text(text, style: Theme.of(context).textTheme.bodyMedium),
  );
}
