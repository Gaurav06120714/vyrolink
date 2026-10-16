import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../app/failure.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';

/// Doc 16 screens 1, 2 and 13: three explanation pages, identity creation, then
/// a contextual Bluetooth request (never all permissions at once).
class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({super.key, required this.onDone});

  final VoidCallback onDone;

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  final _pages = PageController();
  int _page = 0;

  static const _intro = [
    _IntroPage(
      icon: Icons.wifi_off_rounded,
      title: 'Messages without\nthe internet',
      body: 'VYRO LINK needs no SIM, no Wi-Fi and no account. Messages are end-to-end encrypted on your phone before they go anywhere.',
    ),
    _IntroPage(
      icon: Icons.device_hub_rounded,
      title: 'They travel\nphone to phone',
      body: 'When phones with VYRO LINK come near each other, they pass encrypted messages along over Bluetooth. Delivery can take minutes or days, depending on who you meet.',
    ),
    _IntroPage(
      icon: Icons.qr_code_2_rounded,
      title: 'Add people\nin person',
      body: 'You add a contact by scanning each other\'s code face to face. Nobody else can read your messages, including the phones that carry them.',
    ),
  ];

  void _next() {
    if (_page < _intro.length) {
      _pages.nextPage(
        duration: Motion.reduced(context) ? const Duration(milliseconds: 1) : Motion.nav,
        curve: Curves.easeOutCubic,
      );
    }
  }

  @override
  void dispose() {
    _pages.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final onIntro = _page < _intro.length;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            Align(
              alignment: AlignmentDirectional.centerEnd,
              child: AnimatedOpacity(
                opacity: onIntro && _page > 0 ? 1 : 0,
                duration: Motion.micro,
                child: TextButton(
                  onPressed: onIntro && _page > 0
                      ? () => _pages.animateToPage(_intro.length, duration: Motion.nav, curve: Curves.easeOutCubic)
                      : null,
                  child: const Text('Skip'),
                ),
              ),
            ),
            Expanded(
              child: PageView(
                controller: _pages,
                physics: onIntro ? const PageScrollPhysics() : const NeverScrollableScrollPhysics(),
                onPageChanged: (p) => setState(() => _page = p),
                children: [
                  ..._intro,
                  _IdentityPage(
                    onCreated: () => _pages.nextPage(duration: Motion.nav, curve: Curves.easeOutCubic),
                  ),
                  _BluetoothPage(onDone: widget.onDone),
                ],
              ),
            ),
            if (onIntro) ...[
              _Dots(count: _intro.length, index: _page, color: c.accent, idle: c.divider),
              Padding(
                padding: EdgeInsets.fromLTRB(Space.gutter(context), Space.xl, Space.gutter(context), Space.l),
                child: SizedBox(
                  width: double.infinity,
                  child: FilledButton(
                    onPressed: _next,
                    child: Text(_page == _intro.length - 1 ? 'Set up this phone' : 'Continue'),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _Dots extends StatelessWidget {
  const _Dots({required this.count, required this.index, required this.color, required this.idle});
  final int count, index;
  final Color color, idle;

  @override
  Widget build(BuildContext context) => Semantics(
    label: 'Page ${index + 1} of $count',
    child: Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        for (var i = 0; i < count; i++)
          AnimatedContainer(
            duration: Motion.micro,
            margin: const EdgeInsets.symmetric(horizontal: 3),
            width: i == index ? 20 : 6,
            height: 6,
            decoration: BoxDecoration(color: i == index ? color : idle, borderRadius: BorderRadius.circular(3)),
          ),
      ],
    ),
  );
}

/// A quiet illustration: the icon inside concentric rings ("signal" motif).
class Emblem extends StatelessWidget {
  const Emblem({super.key, required this.icon, this.size = 168});
  final IconData icon;
  final double size;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    return ExcludeSemantics(
      child: SizedBox(
        width: size,
        height: size,
        child: Stack(
          alignment: Alignment.center,
          children: [
            for (final (f, a) in [(1.0, 0.05), (0.74, 0.08), (0.50, 0.14)])
              Container(
                width: size * f,
                height: size * f,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: c.accent.withValues(alpha: a),
                ),
              ),
            Icon(icon, size: size * 0.26, color: c.accent),
          ],
        ),
      ),
    );
  }
}

class _IntroPage extends StatelessWidget {
  const _IntroPage({required this.icon, required this.title, required this.body});
  final IconData icon;
  final String title, body;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    return SingleChildScrollView(
      padding: EdgeInsets.symmetric(horizontal: Space.gutter(context) + Space.s),
      child: Column(
        children: [
          const SizedBox(height: Space.xxl),
          Emblem(icon: icon),
          const SizedBox(height: Space.xxl),
          Semantics(
            header: true,
            child: Text(title, style: t.displaySmall, textAlign: TextAlign.center),
          ),
          const SizedBox(height: Space.l),
          Text(
            body,
            style: t.bodyLarge!.copyWith(color: c.text2),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _IdentityPage extends StatefulWidget {
  const _IdentityPage({required this.onCreated});
  final VoidCallback onCreated;

  @override
  State<_IdentityPage> createState() => _IdentityPageState();
}

class _IdentityPageState extends State<_IdentityPage> {
  final _name = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    final s = context.services;
    final name = _name.text.trim();
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await s.call((api) => api.createIdentity());
      await s.call((api) => api.setSetting(AppServices.profileNameKey, name));
      s.profileName = name;
      HapticFeedback.lightImpact();
      widget.onCreated();
    } on VlFailure catch (e) {
      setState(() => _error = e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    return SingleChildScrollView(
      padding: EdgeInsets.symmetric(horizontal: Space.gutter(context) + Space.s),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(height: Space.xl),
          const Center(child: Emblem(icon: Icons.key_rounded, size: 120)),
          const SizedBox(height: Space.xl),
          Semantics(
            header: true,
            child: Text('Your identity', style: t.displaySmall, textAlign: TextAlign.center),
          ),
          const SizedBox(height: Space.m),
          Text(
            'Keys are created on this phone and never leave it. Your name is shared only with people you pair with.',
            style: t.bodyLarge!.copyWith(color: c.text2),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: Space.xl),
          TextField(
            controller: _name,
            enabled: !_busy,
            maxLength: 32,
            textCapitalization: TextCapitalization.words,
            textInputAction: TextInputAction.done,
            autofillHints: const [AutofillHints.nickname],
            decoration: const InputDecoration(labelText: 'Your name', hintText: 'For example, Asha'),
            onSubmitted: (_) => _create(),
          ),
          if (_error != null) InfoBanner(kind: BannerKind.err, text: _error!),
          const SizedBox(height: Space.l),
          FilledButton(
            onPressed: _busy ? null : _create,
            child: _busy
                ? const SizedBox.square(dimension: 20, child: CircularProgressIndicator.adaptive(strokeWidth: 2))
                : const Text('Create my identity'),
          ),
          const SizedBox(height: Space.xl),
        ],
      ),
    );
  }
}

class _BluetoothPage extends StatefulWidget {
  const _BluetoothPage({required this.onDone});
  final VoidCallback onDone;

  @override
  State<_BluetoothPage> createState() => _BluetoothPageState();
}

class _BluetoothPageState extends State<_BluetoothPage> {
  bool _busy = false;

  Future<void> _enable() async {
    setState(() => _busy = true);
    try {
      await context.services.call((api) => api.setRelayEnabled(true));
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
    widget.onDone();
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    Widget point(IconData i, String s) => Padding(
      padding: const EdgeInsets.only(bottom: Space.l),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(i, size: 22, color: c.accent),
          const SizedBox(width: Space.m),
          Expanded(child: Text(s, style: t.bodyMedium)),
        ],
      ),
    );
    return SingleChildScrollView(
      padding: EdgeInsets.symmetric(horizontal: Space.gutter(context) + Space.s),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SizedBox(height: Space.xl),
          const Center(child: Emblem(icon: Icons.bluetooth_rounded, size: 120)),
          const SizedBox(height: Space.xl),
          Semantics(
            header: true,
            child: Text('Carry messages\nwith Bluetooth', style: t.displaySmall, textAlign: TextAlign.center),
          ),
          const SizedBox(height: Space.xl),
          point(
            Icons.lock_outline_rounded,
            'Phones exchange only encrypted envelopes. A phone carrying your message cannot read it.',
          ),
          point(Icons.visibility_off_outlined, 'VYRO LINK never shows who is nearby and does not use your location.'),
          point(
            Icons.battery_5_bar_rounded,
            'Your phone may limit Bluetooth in the background. Messages move most reliably while VYRO LINK is open.',
          ),
          const SizedBox(height: Space.s),
          FilledButton(onPressed: _busy ? null : _enable, child: const Text('Turn on Bluetooth relay')),
          TextButton(onPressed: _busy ? null : widget.onDone, child: const Text('Not now')),
          const SizedBox(height: Space.l),
        ],
      ),
    );
  }
}
