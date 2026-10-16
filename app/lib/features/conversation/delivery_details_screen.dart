import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/live.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/delivery.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';

/// Doc 16 screen 8: an honest timeline for one of my messages, from the
/// state history the core records. Relay custody is never presented as
/// delivery.
class DeliveryDetailsScreen extends StatelessWidget {
  const DeliveryDetailsScreen({super.key, required this.messageId});

  final Uint8List messageId;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Delivery details')),
      body: Live<(Message?, List<MessageEvent>)>(
        load: (api) async => (await api.message(messageId), await api.messageEvents(messageId)),
        builder: (context, data) {
          final (message, events) = data;
          if (message == null) {
            return const EmptyState(
              icon: Icons.inbox_outlined,
              title: 'Message not found',
              body: 'It was deleted from this phone or has disappeared.',
            );
          }
          final c = VlColors.of(context);
          final t = Theme.of(context).textTheme;
          final current = DeliveryInfo.of(message.state);
          return ListView(
            padding: const EdgeInsets.only(bottom: Space.xxl),
            children: [
              Padding(
                padding: EdgeInsets.all(Space.gutter(context)),
                child: Container(
                  padding: const EdgeInsets.all(Space.l),
                  decoration: BoxDecoration(color: c.surface, borderRadius: BorderRadius.circular(Radii.card)),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        message.deleted ? 'Deleted message' : message.body,
                        style: t.bodyLarge,
                        maxLines: 6,
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: Space.m),
                      Row(
                        children: [
                          DeliveryGlyph(state: message.state, size: 20),
                          const SizedBox(width: Space.s),
                          Expanded(
                            child: Text(current.label, style: t.titleSmall!.copyWith(color: current.tone(c))),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.xs),
                      Text(current.explanation, style: t.bodySmall),
                      if (message.attachment case final a?)
                        Padding(
                          padding: const EdgeInsets.only(top: Space.s),
                          child: Text(
                            'Attachment: ${a.deliveredChunks} of ${a.chunkCount} parts confirmed by the recipient\'s phone.',
                            style: t.bodySmall,
                          ),
                        ),
                      if (message.attachment == null &&
                          (message.state == DeliveryState.expired ||
                              message.state == DeliveryState.deliveryUnknown)) ...[
                        const SizedBox(height: Space.m),
                        OutlinedButton.icon(
                          onPressed: () async {
                            try {
                              await context.services.mutate((api) => api.resend(messageId));
                              if (context.mounted) {
                                showMessage(context, 'Queued again. It leaves when a phone comes near.');
                              }
                            } on VlFailure catch (e) {
                              if (context.mounted) showMessage(context, e.message);
                            }
                          },
                          icon: const Icon(Icons.refresh_rounded),
                          label: const Text('Send again'),
                        ),
                        Text(
                          'Encrypts it again with the same identity, so if it already arrived it will not appear twice.',
                          style: t.bodySmall,
                        ),
                      ],
                    ],
                  ),
                ),
              ),
              const SectionHeader('Timeline'),
              Section(
                rows: [
                  for (final e in events)
                    ListTile(
                      leading: DeliveryGlyph(state: e.state, size: 20),
                      title: Text(
                        e.state == DeliveryState.queued ? 'Encrypted and queued' : DeliveryInfo.of(e.state).label,
                      ),
                      subtitle: Text(fullStamp(context, e.atSecs)),
                    ),
                ],
                footer: 'Times come from this phone. Relays never report back, so a copy can travel further than shown here.',
              ),
              const SectionHeader('What these mean'),
              Section(
                rows: [
                  for (final s in const [
                    DeliveryState.queued,
                    DeliveryState.acceptedByRelay,
                    DeliveryState.delivered,
                    DeliveryState.deliveryUnknown,
                    DeliveryState.expired,
                  ])
                    ListTile(
                      leading: DeliveryGlyph(state: s, size: 20),
                      title: Text(DeliveryInfo.of(s).label),
                      subtitle: Text(DeliveryInfo.of(s).explanation),
                    ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}
