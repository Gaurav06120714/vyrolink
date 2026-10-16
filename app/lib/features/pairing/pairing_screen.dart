import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/identity.dart';
import '../../design/widgets/states.dart';

/// Doc 16 screen 5: mutual, in-person exchange of pairing codes. Each phone
/// shows its code and scans the other's. The camera is requested only here.
class PairingScreen extends StatefulWidget {
  const PairingScreen({super.key});

  @override
  State<PairingScreen> createState() => _PairingScreenState();
}

class _PairingScreenState extends State<PairingScreen> {
  int _tab = 0;
  QrCode? _qr;
  VlFailure? _loadError;
  PermissionState? _camera;
  Contact? _paired;

  /// The scanned person was already a contact: their keys were refreshed.
  bool _refreshed = false;
  String? _scanError;
  bool _busy = false;
  StreamSubscription<String>? _scans;
  Timer? _clearError;

  @override
  void initState() {
    super.initState();
    final s = context.services;
    _scans = s.qrScans.listen(_onScan);
    _prepare();
  }

  @override
  void dispose() {
    _scans?.cancel();
    _clearError?.cancel();
    super.dispose();
  }

  /// One pairing code per visit to this screen: the other phone may scan it
  /// while we scan theirs, so it must not change underneath them.
  Future<void> _prepare() async {
    final s = context.services;
    try {
      final name = s.profileName.isEmpty ? 'VYRO LINK user' : s.profileName;
      final code = await s.call((api) => api.pairingCode(name));
      final qr = await s.call((api) => api.qrCode(code));
      final camera = await s.call((api) => api.cameraPermission());
      if (mounted) {
        setState(() {
          _qr = qr;
          _camera = camera;
        });
      }
    } on VlFailure catch (e) {
      if (mounted) setState(() => _loadError = e);
    }
  }

  Future<void> _requestCamera() async {
    final state = await context.services.call((api) => api.requestCameraPermission());
    if (mounted) setState(() => _camera = state);
  }

  Future<void> _onScan(String text) async {
    if (_busy || _paired != null || _tab != 1) return;
    setState(() => _busy = true);
    final s = context.services;
    try {
      final known = {for (final c in await s.call((api) => api.contacts())) idKey(c.contactId)};
      final contact = await s.mutate((api) => api.completePairing(text));
      HapticFeedback.lightImpact();
      if (!mounted) return;
      setState(() {
        _paired = contact;
        _refreshed = known.contains(idKey(contact.contactId));
        _scanError = null;
        _tab = 0;
      });
      _showVerify(contact);
    } on VlFailure catch (e) {
      if (!mounted) return;
      HapticFeedback.heavyImpact();
      setState(() => _scanError = e.message);
      _clearError?.cancel();
      _clearError = Timer(const Duration(seconds: 6), () {
        if (mounted) setState(() => _scanError = null);
      });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _showVerify(Contact contact) => showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (_) => VerifySheet(contact: contact),
  );

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final gutter = Space.gutter(context);
    return Scaffold(
      appBar: AppBar(title: const Text('Add contact')),
      body: SafeArea(
        child: _loadError != null
            ? ErrorState(failure: _loadError!, onRetry: _prepare)
            : ListView(
                padding: EdgeInsets.fromLTRB(gutter, Space.s, gutter, Space.xl),
                children: [
                  SegmentedButton<int>(
                    segments: const [
                      ButtonSegment(value: 0, icon: Icon(Icons.qr_code_2_rounded), label: Text('My code')),
                      ButtonSegment(value: 1, icon: Icon(Icons.qr_code_scanner_rounded), label: Text('Scan code')),
                    ],
                    selected: {_tab},
                    showSelectedIcon: false,
                    onSelectionChanged: (v) => setState(() => _tab = v.first),
                  ),
                  const SizedBox(height: Space.xl),
                  Center(
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 340),
                      child: _tab == 0 ? _myCode(c, t) : _scanner(c, t),
                    ),
                  ),
                  const SizedBox(height: Space.xl),
                  _Steps(paired: _paired, refreshed: _refreshed),
                  if (_paired != null) ...[
                    const SizedBox(height: Space.l),
                    FilledButton(
                      onPressed: () {
                        Navigator.pop(context);
                        Routes.conversation(context, _paired!);
                      },
                      child: Text('Message ${_paired!.displayName}'),
                    ),
                    TextButton(onPressed: () => _showVerify(_paired!), child: const Text('Compare safety numbers')),
                  ],
                ],
              ),
      ),
    );
  }

  Widget _myCode(VlColors c, TextTheme t) => Column(
    children: [
      if (_qr == null)
        const AspectRatio(aspectRatio: 1, child: Center(child: CircularProgressIndicator.adaptive()))
      else
        QrPanel(code: _qr!, semanticLabel: 'Your VYRO LINK pairing code'),
      const SizedBox(height: Space.l),
      Text(
        'Ask them to scan this code with VYRO LINK, then scan theirs.',
        style: t.bodyMedium!.copyWith(color: c.text2),
        textAlign: TextAlign.center,
      ),
    ],
  );

  Widget _scanner(VlColors c, TextTheme t) {
    final camera = _camera;
    if (camera == PermissionState.granted) {
      return Column(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(Radii.sheet),
            child: AspectRatio(
              aspectRatio: 1,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  const QrScannerView(),
                  const IgnorePointer(child: CustomPaint(painter: _Viewfinder())),
                  if (_busy)
                    const ColoredBox(
                      color: Colors.black38,
                      child: Center(child: CircularProgressIndicator.adaptive()),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: Space.l),
          if (_scanError != null)
            InfoBanner(kind: BannerKind.err, text: _scanError!)
          else
            Text(
              'Point the camera at the code on their phone.',
              style: t.bodyMedium!.copyWith(color: c.text2),
              textAlign: TextAlign.center,
            ),
        ],
      );
    }
    final denied = camera == PermissionState.denied || camera == PermissionState.restricted;
    return Container(
      padding: const EdgeInsets.all(Space.xl),
      decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.sheet)),
      child: Column(
        children: [
          Icon(Icons.photo_camera_outlined, size: 40, color: c.text2),
          const SizedBox(height: Space.l),
          Text(denied ? 'Camera access is off' : 'Scan with the camera', style: t.titleMedium),
          const SizedBox(height: Space.s),
          Text(
            denied
                ? 'Allow camera access for VYRO LINK in Settings to scan codes. It is used only on this screen.'
                : 'VYRO LINK uses the camera only to read the code on their phone. Nothing is saved or sent.',
            style: t.bodyMedium!.copyWith(color: c.text2),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: Space.l),
          FilledButton(
            onPressed: denied ? () => context.services.call((api) => api.openAppSettings()) : _requestCamera,
            child: Text(denied ? 'Open Settings' : 'Allow camera'),
          ),
        ],
      ),
    );
  }
}

class _Steps extends StatelessWidget {
  const _Steps({required this.paired, required this.refreshed});
  final Contact? paired;
  final bool refreshed;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    Widget step(bool done, String text) => Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.xs),
      child: Row(
        children: [
          Icon(
            done ? Icons.check_circle_rounded : Icons.radio_button_unchecked_rounded,
            size: 20,
            color: done ? c.ok : c.text2,
            semanticLabel: done ? 'Done' : 'To do',
          ),
          const SizedBox(width: Space.m),
          Expanded(child: Text(text, style: t.bodyMedium)),
        ],
      ),
    );
    return Container(
      padding: const EdgeInsets.all(Space.l),
      decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          step(
            paired != null,
            paired == null
                ? 'Scan their code'
                : refreshed
                ? 'Keys refreshed with ${paired!.displayName}; your messages are kept'
                : 'You scanned ${paired!.displayName}',
          ),
          step(false, 'They scan your code (their phone confirms it, not yours)'),
        ],
      ),
    );
  }
}

class _Viewfinder extends CustomPainter {
  const _Viewfinder();

  @override
  void paint(Canvas canvas, Size size) {
    final p = Paint()
      ..color = Colors.white
      ..strokeWidth = 4
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;
    final inset = size.width * 0.16;
    final r = Rect.fromLTRB(inset, inset, size.width - inset, size.height - inset);
    final len = size.width * 0.12;
    for (final (corner, dx, dy) in [
      (r.topLeft, 1.0, 1.0),
      (r.topRight, -1.0, 1.0),
      (r.bottomLeft, 1.0, -1.0),
      (r.bottomRight, -1.0, -1.0),
    ]) {
      canvas
        ..drawLine(corner, corner.translate(dx * len, 0), p)
        ..drawLine(corner, corner.translate(0, dy * len), p);
    }
  }

  @override
  bool shouldRepaint(_Viewfinder oldDelegate) => false;
}

/// Shows the safety number and lets the user mark the contact verified after
/// comparing it with the other phone in person.
class VerifySheet extends StatefulWidget {
  const VerifySheet({super.key, required this.contact});
  final Contact contact;

  @override
  State<VerifySheet> createState() => _VerifySheetState();
}

class _VerifySheetState extends State<VerifySheet> {
  late bool _verified = widget.contact.verified;

  Future<void> _set(bool v) async {
    await context.services.mutate((api) => api.setVerified(widget.contact.contactId, v));
    if (v) HapticFeedback.lightImpact();
    if (mounted) setState(() => _verified = v);
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    final name = widget.contact.displayName;
    return SingleChildScrollView(
      padding: EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, Space.xl + MediaQuery.paddingOf(context).bottom),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Safety number with $name', style: t.headlineSmall),
          const SizedBox(height: Space.s),
          Text(
            'Compare these numbers with the ones on $name\'s phone. If they match, nobody swapped your codes.',
            style: t.bodyMedium!.copyWith(color: c.text2),
          ),
          const SizedBox(height: Space.l),
          SafetyNumber(number: widget.contact.safetyNumber),
          const SizedBox(height: Space.xl),
          if (_verified) ...[
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.verified_user_rounded, color: c.ok),
                const SizedBox(width: Space.s),
                Text('Verified', style: t.titleSmall!.copyWith(color: c.ok)),
              ],
            ),
            TextButton(onPressed: () => _set(false), child: const Text('Clear verification')),
          ] else ...[
            FilledButton(onPressed: () => _set(true), child: const Text('They match. Mark as verified')),
            TextButton(onPressed: () => Navigator.pop(context), child: const Text('Not now')),
          ],
        ],
      ),
    );
  }
}
