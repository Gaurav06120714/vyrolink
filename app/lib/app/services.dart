import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:vl_host/vl_host.dart';

import 'failure.dart';
import 'google_account.dart';
import 'internet.dart';

/// Everything screens need: the typed native host, change notifications and
/// the network status stream. One instance per app; tests pass a fake host.
class AppServices implements VlEventsApi {
  AppServices(this.api);

  final VlHostApi api;

  /// Optional internet mailbox (ADR-039); started by the root gate when on.
  late final internet = InternetSync(api, onNewMail: bump);

  /// Bumped whenever stored data may have changed (local edits or a Bluetooth
  /// exchange). Live views reload when it changes.
  final changes = ValueNotifier<int>(0);
  final network = ValueNotifier<NetworkStatus?>(null);
  final appLock = ValueNotifier<bool>(false);

  /// Contact [idKey] → unix seconds last seen in Bluetooth range (ADR-029).
  /// Refreshed with the network status; the hosts keep it in memory only.
  final nearby = ValueNotifier<Map<String, int>>(const {});
  final _scans = StreamController<String>.broadcast();

  StartupInfo startup = StartupInfo(hasIdentity: false, maxTextBytes: 1700, reactions: const []);
  String profileName = '';

  /// Re-runs startup (after panic wipe or reset). Set by the root gate.
  VoidCallback restartApp = () {};

  Stream<String> get qrScans => _scans.stream;

  void attach() => VlEventsApi.setUp(this);

  void bump() => changes.value++;

  @override
  void dataChanged() => bump();

  @override
  void networkChanged(NetworkStatus status) => network.value = status;

  @override
  void qrScanned(String text) => _scans.add(text);

  /// Calls the host and turns platform errors into [VlFailure].
  Future<T> call<T>(Future<T> Function(VlHostApi api) op) async {
    try {
      return await op(api);
    } on PlatformException catch (e) {
      throw VlFailure.fromPlatform(e);
    }
  }

  /// A call that changes stored data; live views refresh afterwards.
  Future<T> mutate<T>(Future<T> Function(VlHostApi api) op) async {
    try {
      return await call(op);
    } finally {
      bump();
    }
  }

  /// Erases everything. Google's SDK keeps its own session outside our
  /// databases, so sign out of it first (ADR-030).
  Future<void> panicWipe() async {
    internet.stop();
    await GoogleAccount.signOut();
    await call((api) => api.panicWipe());
  }

  Future<void> refreshNetwork() async {
    try {
      network.value = await api.networkStatus();
      final seen = await api.nearbyContacts(nearbyWindowSecs);
      final next = {for (final n in seen) idKey(n.contactId): n.seenSecs};
      if (!mapEquals(next, nearby.value)) nearby.value = next;
    } on PlatformException {
      // The relay is not set up yet; the network screen shows "unknown".
    }
  }

  /// Starts or stops the internet mailbox from the saved setting.
  Future<void> applyInternet() async {
    final on = startup.hasIdentity && await call((api) => api.setting(InternetSync.enabledKey)) == '1';
    on ? internet.start() : internet.stop();
  }

  /// A contact counts as nearby for this long after the last link that showed them.
  static const nearbyWindowSecs = 10 * 60;

  static const profileNameKey = 'profile.name';
  static const appLockKey = 'privacy.app_lock';
  static const screenCaptureKey = 'privacy.block_screenshots';
}

class AppScope extends InheritedWidget {
  const AppScope({super.key, required this.services, required super.child});

  final AppServices services;

  static AppServices of(BuildContext context) => context.getInheritedWidgetOfExactType<AppScope>()!.services;

  @override
  bool updateShouldNotify(AppScope oldWidget) => services != oldWidget.services;
}

extension AppContext on BuildContext {
  AppServices get services => AppScope.of(this);
}

/// Stable map key for an id.
String idKey(Uint8List id) => id.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
