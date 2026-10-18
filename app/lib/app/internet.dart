import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:vl_host/vl_host.dart';

/// Optional internet mailbox (ADR-039). Off by default. When on, and while the
/// app is open, the sealed envelopes this phone holds (its own and those it
/// carries for others) go up to the mailbox, and envelopes addressed to its
/// current tags come down and take the same path as Bluetooth deliveries.
/// The mailbox cannot read them; it sees when this phone connects and its tags.
class InternetSync {
  InternetSync(this.api, {http.Client? client, this.onNewMail}) : _http = client ?? http.Client();

  final VlHostApi api;
  final http.Client _http;

  /// Called when mail arrived or my messages changed state (uploaded).
  final VoidCallback? onNewMail;

  static const enabledKey = 'internet.enabled';
  static const mailbox = String.fromEnvironment('VL_MAILBOX', defaultValue: 'https://vyrolink-mailbox.onrender.com');
  static const every = Duration(seconds: 10);

  /// When the mailbox last answered, and the last problem (null when fine).
  final lastOk = ValueNotifier<DateTime?>(null);
  final lastError = ValueNotifier<String?>(null);

  Timer? _timer;
  bool _busy = false;

  bool get running => _timer != null;

  void start() {
    _timer ??= Timer.periodic(every, (_) => sync());
    sync();
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
  }

  Future<Map<String, dynamic>> _post(String path, Map<String, Object> body) async {
    // A free Render instance can take most of a minute to wake up.
    final r = await _http
        .post(Uri.parse('$mailbox$path'), headers: {'content-type': 'application/json'}, body: jsonEncode(body))
        .timeout(const Duration(seconds: 60));
    if (r.statusCode != 200) throw http.ClientException('mailbox answered ${r.statusCode}');
    return jsonDecode(r.body) as Map<String, dynamic>;
  }

  /// One round: upload what the mailbox lacks, then fetch my mail. Safe to
  /// call at any time; overlapping calls are skipped.
  Future<void> sync() async {
    if (_busy) return;
    _busy = true;
    try {
      // Up to a few batches per round, so a full outbox drains quickly.
      var fresh = false;
      for (var i = 0; i < 4; i++) {
        final out = await api.internetOutbox();
        if (out.isEmpty) break;
        await _post('/v1/put', {'envelopes': out.map(base64Encode).toList()});
        await api.internetUploaded(out);
        fresh = true;
        if (out.length < 64) break;
      }
      final tags = await api.internetTags();
      if (tags.isNotEmpty) {
        final got = await _post('/v1/get', {'tags': tags.map(base64Encode).toList()});
        for (final e in (got['envelopes'] as List).cast<String>()) {
          fresh |= await api.receiveInternet(base64Decode(e));
        }
      }
      if (fresh) onNewMail?.call();
      lastOk.value = DateTime.now();
      lastError.value = null;
    } on Object catch (e) {
      // Offline or the mailbox is asleep: Bluetooth keeps working; try again later.
      lastError.value = e is TimeoutException ? 'The mailbox did not answer' : 'No connection to the mailbox';
    } finally {
      _busy = false;
    }
  }
}
