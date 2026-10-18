import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vyro_link/app/internet.dart';

import 'fake_host.dart';

void main() {
  test('a sync uploads what the mailbox lacks, then fetches mail for my tags', () async {
    final host = FakeHost()..outbox.add(id(1));
    final calls = <String>[];
    final mailbox = MockClient((r) async {
      final body = jsonDecode(r.body) as Map<String, dynamic>;
      calls.add(r.url.path);
      if (r.url.path == '/v1/put') {
        expect(body['envelopes'], [base64Encode(id(1))]);
        return http.Response('{"accepted":1}', 200);
      }
      expect(body['tags'], [base64Encode(id(7))]);
      return http.Response(
        jsonEncode({
          'envelopes': [base64Encode(id(2))],
        }),
        200,
      );
    });
    var newMail = 0;
    final sync = InternetSync(host, client: mailbox, onNewMail: () => newMail++);
    await sync.sync();
    expect(calls, ['/v1/put', '/v1/get']);
    expect(host.uploaded.single, id(1));
    expect(host.received.single, id(2));
    expect(newMail, 1);
    expect(sync.lastError.value, isNull);

    // Uploaded once only: the next round just fetches.
    calls.clear();
    await sync.sync();
    expect(calls, ['/v1/get']);
  });

  test('a mailbox error keeps the envelope for the next try', () async {
    final host = FakeHost()..outbox.add(id(1));
    final sync = InternetSync(host, client: MockClient((_) async => http.Response('', 503)));
    await sync.sync();
    expect(host.uploaded, isEmpty, reason: 'not marked as held by the mailbox');
    expect(sync.lastError.value, isNotNull);
  });
}
