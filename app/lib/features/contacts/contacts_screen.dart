import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/live.dart';
import '../../app/routes.dart';
import '../../design/tokens.dart';
import '../../app/services.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';

/// Doc 16 screen 4: paired contacts with verification badges.
class ContactsScreen extends StatelessWidget {
  const ContactsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      floatingActionButton: FloatingActionButton.extended(
        heroTag: null,
        onPressed: () => Routes.pairing(context),
        icon: const Icon(Icons.qr_code_scanner_rounded),
        label: const Text('Add contact'),
      ),
      body: CustomScrollView(
        slivers: [
          const SliverAppBar.large(title: Text('Contacts')),
          Live<List<Contact>>(
            sliver: true,
            load: (api) => api.contacts(),
            builder: (context, contacts) {
              if (contacts.isEmpty) {
                return const SliverFillRemaining(
                  hasScrollBody: false,
                  child: EmptyState(
                    icon: Icons.people_outline_rounded,
                    title: 'No contacts yet',
                    body: 'Contacts are added face to face by scanning each other\'s VYRO LINK code. No phone numbers, no address book.',
                  ),
                );
              }
              final sorted = [...contacts]
                ..sort((a, b) => a.displayName.toLowerCase().compareTo(b.displayName.toLowerCase()));
              return SliverList.builder(
                itemCount: sorted.length,
                itemBuilder: (context, i) {
                  final c = sorted[i];
                  return ValueListenableBuilder<Map<String, int>>(
                    valueListenable: context.services.nearby,
                    builder: (context, nearby, _) {
                      final status = [
                        if (nearby.containsKey(idKey(c.contactId))) 'Nearby',
                        c.verified ? 'Verified' : 'Not verified',
                        if (c.blocked) 'Blocked',
                      ].join(' · ');
                      return ListTile(
                        contentPadding: EdgeInsets.symmetric(horizontal: Space.gutter(context), vertical: Space.xs),
                        leading: Avatar(name: c.displayName, seed: c.contactId, size: 44),
                        title: Text(c.displayName, maxLines: 2, overflow: TextOverflow.ellipsis),
                        subtitle: Text(status),
                        trailing: VerifiedBadge(verified: c.verified, size: 20),
                        onTap: () => Routes.contact(context, c.contactId),
                      );
                    },
                  );
                },
              );
            },
          ),
          const SliverPadding(padding: EdgeInsets.only(bottom: 96)),
        ],
      ),
    );
  }
}
