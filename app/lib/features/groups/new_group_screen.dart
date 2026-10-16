import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/live.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';

/// Creates a group, or (with [group]) changes its members. Groups hold 2–8
/// people including you (ADR-024); only the creator can change members.
class NewGroupScreen extends StatefulWidget {
  const NewGroupScreen({super.key, this.group});

  final Group? group;

  @override
  State<NewGroupScreen> createState() => _NewGroupScreenState();
}

class _NewGroupScreenState extends State<NewGroupScreen> {
  static const maxOthers = 7;
  late final _name = TextEditingController(text: widget.group?.name ?? '');
  final _selected = <String, Contact>{};
  bool _seeded = false;
  bool _busy = false;

  bool get _editing => widget.group != null;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _seed(List<Contact> contacts) {
    if (_seeded || widget.group == null) return;
    _seeded = true;
    final members = {for (final m in widget.group!.members) idKey(m.fingerprint)};
    for (final c in contacts) {
      if (members.contains(idKey(c.fingerprint))) _selected[idKey(c.contactId)] = c;
    }
  }

  Future<void> _save() async {
    final s = context.services;
    final name = _name.text.trim();
    final ids = _selected.values.map((c) => c.contactId).toList();
    final me = s.profileName.isEmpty ? 'Me' : s.profileName;
    setState(() => _busy = true);
    try {
      if (_editing) {
        await s.mutate((api) => api.setGroupMembers(widget.group!.groupId, me, ids));
        if (mounted) Navigator.pop(context);
      } else {
        final id = await s.mutate((api) => api.createGroup(name, me, ids));
        final group = (await s.call((api) => api.groups())).firstWhere((g) => idKey(g.groupId) == idKey(id));
        if (!mounted) return;
        Navigator.pop(context);
        Routes.group(context, group);
      }
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    final valid = _name.text.trim().isNotEmpty && _selected.isNotEmpty && _selected.length <= maxOthers;
    return Scaffold(
      appBar: AppBar(
        title: Text(_editing ? 'Members' : 'New group'),
        actions: [TextButton(onPressed: valid && !_busy ? _save : null, child: Text(_editing ? 'Save' : 'Create'))],
      ),
      body: Live<List<Contact>>(
        load: (api) => api.contacts(),
        builder: (context, contacts) {
          _seed(contacts);
          final usable = contacts.where((x) => !x.blocked).toList()
            ..sort((a, b) => a.displayName.toLowerCase().compareTo(b.displayName.toLowerCase()));
          return ListView(
            children: [
              if (!_editing)
                Padding(
                  padding: EdgeInsets.fromLTRB(Space.gutter(context), Space.s, Space.gutter(context), 0),
                  child: TextField(
                    controller: _name,
                    maxLength: 32,
                    textCapitalization: TextCapitalization.words,
                    decoration: const InputDecoration(labelText: 'Group name'),
                    onChanged: (_) => setState(() {}),
                  ),
                ),
              Padding(
                padding: EdgeInsets.symmetric(horizontal: Space.gutter(context)),
                child: Text(
                  'Messages go to each member separately over Bluetooth, so they may arrive at different times. '
                  'Everyone in the group sees the member list.',
                  style: t.bodySmall,
                ),
              ),
              SectionHeader('Members · ${_selected.length + 1} of ${maxOthers + 1}'),
              for (final contact in usable)
                CheckboxListTile(
                  value: _selected.containsKey(idKey(contact.contactId)),
                  onChanged: (v) => setState(() {
                    final k = idKey(contact.contactId);
                    if (v == true && _selected.length < maxOthers) {
                      _selected[k] = contact;
                    } else {
                      _selected.remove(k);
                    }
                  }),
                  secondary: Avatar(name: contact.displayName, seed: contact.contactId, size: 40),
                  title: Text(contact.displayName),
                  controlAffinity: ListTileControlAffinity.trailing,
                ),
              if (usable.isEmpty)
                Padding(
                  padding: const EdgeInsets.all(Space.xl),
                  child: Text('Add contacts in person first.', style: t.bodyMedium!.copyWith(color: c.text2)),
                ),
            ],
          );
        },
      ),
    );
  }
}
