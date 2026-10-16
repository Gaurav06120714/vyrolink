import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vl_host/vl_host.dart';
import 'package:vyro_link/app/failure.dart';
import 'package:vyro_link/design/tokens.dart';
import 'package:vyro_link/design/widgets/avatar.dart';
import 'package:vyro_link/design/widgets/delivery.dart';
import 'package:vyro_link/design/widgets/identity.dart';
import 'package:vyro_link/util/format.dart';

void main() {
  group('errors from the host', () {
    test('every core error code maps to a typed failure', () {
      for (final code in VlErrorCode.values.where((c) => c != VlErrorCode.unknown)) {
        expect(VlFailure.fromPlatform(PlatformException(code: code.wire)).code, code);
      }
      expect(VlFailure.fromPlatform(PlatformException(code: 'Nonsense')).code, VlErrorCode.unknown);
    });

    test('messages are plain language and never echo the technical detail', () {
      final f = VlFailure.fromPlatform(PlatformException(code: 'Storage', message: 'sqlite: file is not a database'));
      expect(f.message, isNot(contains('sqlite')));
      expect(VlFailure(VlErrorCode.ownPairingCode).message, contains('your own code'));
    });
  });

  group('delivery states (ADR-015)', () {
    test('only delivered and read use the success tone', () {
      const c = VlColors.light;
      for (final s in DeliveryState.values) {
        final ok = DeliveryInfo.of(s).tone(c) == c.ok;
        expect(ok, s == DeliveryState.delivered || s == DeliveryState.read, reason: '$s');
      }
    });

    test('relay custody is never labelled as delivery', () {
      expect(DeliveryInfo.of(DeliveryState.acceptedByRelay).label, isNot(contains('Delivered')));
      expect(DeliveryInfo.of(DeliveryState.sentToNearby).label, 'Waiting for nearby devices');
      expect(DeliveryInfo.of(DeliveryState.acceptedByRelay).explanation, contains('may not have it'));
    });
  });

  test('initials', () {
    expect(Avatar.initials('asha k'), 'AK');
    expect(Avatar.initials('  '), '?');
    expect(Avatar.initials('Ölo'), 'Ö');
  });

  test('fingerprint is grouped hex', () {
    final text = fingerprintText(Uint8List.fromList(List.generate(32, (i) => i)));
    expect(text.split(' '), hasLength(8));
    expect(text, startsWith('00010203 '));
  });

  test('timer labels and bytes', () {
    expect(timerLabel(0), 'Off');
    expect(timerLabel(86400), '1 day');
    expect(timerLabel(7200), '2 hours');
    expect(formatBytes(512), '512 B');
    expect(formatBytes(2048), '2.0 KB');
  });
}
