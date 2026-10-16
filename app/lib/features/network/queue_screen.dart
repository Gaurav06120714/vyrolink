import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/live.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/delivery.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';

/// Doc 16 screen 10: my messages still waiting for delivery proof, by contact.
class QueueScreen extends StatelessWidget {
  const QueueScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Waiting to be delivered')),
      body: Live<(List<Message>, Map<String, Contact>)>(
        load: (api) async =>
            (await api.pendingOutgoing(), {for (final c in await api.contacts()) idKey(c.contactId): c}),
        builder: (context, data) {
          final (pending, contacts) = data;
          if (pending.isEmpty) {
            return const EmptyState(
              icon: Icons.done_all_rounded,
              title: 'Nothing waiting',
              body: 'Every message you sent has a delivery receipt or has expired.',
            );
          }
          final c = VlColors.of(context);
          final groups = <String, List<Message>>{};
          for (final m in pending) {
            groups.putIfAbsent(idKey(m.contactId), () => []).add(m);
          }
          return ListView(
            padding: const EdgeInsets.only(bottom: Space.xxl),
            children: [
              Padding(
                padding: EdgeInsets.fromLTRB(Space.gutter(context), Space.s, Space.gutter(context), 0),
                child: Text(
                  'Copies already handed to other phones cannot be recalled. Deleting here removes the message from this phone only.',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              for (final entry in groups.entries) ...[
                SectionHeader(contacts[entry.key]?.displayName ?? 'Unknown contact'),
                Section(
                  rows: [
                    for (final m in entry.value)
                      ListTile(
                        leading: contacts[entry.key] == null
                            ? null
                            : Avatar(name: contacts[entry.key]!.displayName, seed: m.contactId, size: 36),
                        title: Text(m.body, maxLines: 2, overflow: TextOverflow.ellipsis),
                        subtitle: Row(
                          children: [
                            DeliveryGlyph(state: m.state, size: 14),
                            const SizedBox(width: 4),
                            Flexible(
                              child: Text(
                                '${DeliveryInfo.of(m.state).label} · ${shortStamp(context, m.createdAtSecs)}',
                                style: TextStyle(color: DeliveryInfo.of(m.state).tone(c)),
                              ),
                            ),
                          ],
                        ),
                        trailing: const Icon(Icons.chevron_right_rounded),
                        onTap: () => Routes.deliveryDetails(context, m.messageId),
                      ),
                  ],
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}
