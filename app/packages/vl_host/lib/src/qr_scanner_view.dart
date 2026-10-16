import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

/// Native camera preview that reports QR text through `VlEventsApi.qrScanned`.
/// Show it only after camera permission is granted.
class QrScannerView extends StatelessWidget {
  const QrScannerView({super.key});

  static const _viewType = 'vl_host/qr_scanner';

  @override
  Widget build(BuildContext context) => switch (defaultTargetPlatform) {
    TargetPlatform.iOS => const UiKitView(viewType: _viewType),
    TargetPlatform.android => const AndroidView(viewType: _viewType),
    _ => const SizedBox.expand(),
  };
}
