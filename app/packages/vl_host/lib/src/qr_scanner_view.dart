import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

/// Native camera preview that reports QR text through `VlEventsApi.qrScanned`.
/// Show it only after camera permission is granted.
class QrScannerView extends StatelessWidget {
  const QrScannerView({super.key});

  static const _viewType = 'vl_host/qr_scanner';

  /// The web chat's scanner (a browser camera view), set by its host. In a
  /// browser `defaultTargetPlatform` is the visitor's OS, so check web first.
  static Widget Function()? webBuilder;

  @override
  Widget build(BuildContext context) {
    if (kIsWeb) return webBuilder?.call() ?? const SizedBox.expand();
    return switch (defaultTargetPlatform) {
      TargetPlatform.iOS => const UiKitView(viewType: _viewType),
      TargetPlatform.android => const AndroidView(viewType: _viewType),
      _ => const SizedBox.expand(),
    };
  }
}
