import 'dart:typed_data';

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/routes.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/avatar.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';

/// Local full-text search over decrypted messages (SQLCipher FTS5 in the
/// core). Nothing leaves the phone. Optionally limited to one contact.
class SearchScreen extends StatefulWidget {
  const SearchScreen({super.key, this.contactId});

  final Uint8List? contactId;

  @override
  State<SearchScreen> createState() => _SearchScreenState();
}

class _SearchScreenState extends State<SearchScreen> {
  final _query = TextEditingController();
  Timer? _debounce;
  List<Message>? _results;
  Map<String, Contact> _contacts = const {};
  VlFailure? _error;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    context.services.call((api) => api.contacts()).then((cs) {
      if (mounted) setState(() => _contacts = {for (final c in cs) idKey(c.contactId): c});
    }).ignore();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _query.dispose();
    super.dispose();
  }

  void _changed(String q) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 200), () => _run(q));
  }

  Future<void> _run(String q) async {
    final gen = ++_generation;
    if (q.trim().isEmpty) {
      setState(() => _results = null);
      return;
    }
    try {
      final r = await context.services.call((api) => api.search(q, widget.contactId, 100));
      if (mounted && gen == _generation) {
        setState(() {
          _results = r;
          _error = null;
        });
      }
    } on VlFailure catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  void _open(Message m) {
    final c = _contacts[idKey(m.contactId)];
    if (c != null) Routes.conversation(context, c);
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).textTheme;
    final c = VlColors.of(context);
    final scoped = widget.contactId == null ? null : _contacts[idKey(widget.contactId!)];
    final results = _results;
    return Scaffold(
      appBar: AppBar(
        titleSpacing: 0,
        title: TextField(
          controller: _query,
          autofocus: true,
          textInputAction: TextInputAction.search,
          onChanged: _changed,
          onSubmitted: _run,
          decoration: InputDecoration(
            hintText: scoped == null ? 'Search messages' : 'Search in ${scoped.displayName}',
            filled: false,
            border: InputBorder.none,
            enabledBorder: InputBorder.none,
            focusedBorder: InputBorder.none,
            prefixIcon: const Icon(Icons.search_rounded),
          ),
        ),
      ),
      body: _error != null
          ? ErrorState(failure: _error!)
          : results == null
          ? const EmptyState(
              icon: Icons.manage_search_rounded,
              title: 'Search this phone',
              body: 'Searches happen on this phone only. Word beginnings match, so "lun" finds "lunch".',
            )
          : results.isEmpty
          ? EmptyState(
              icon: Icons.search_off_rounded,
              title: 'No matches',
              body: 'Nothing on this phone matches "${_query.text}".',
            )
          : ListView.separated(
              itemCount: results.length,
              separatorBuilder: (_, _) => const Divider(indent: 72),
              itemBuilder: (context, i) {
                final m = results[i];
                final who = _contacts[idKey(m.contactId)];
                return ListTile(
                  leading: m.outgoing
                      ? CircleAvatar(
                          backgroundColor: c.surface2,
                          child: Icon(Icons.north_east_rounded, color: c.text2, size: 18),
                        )
                      : Avatar(name: who?.displayName ?? '?', seed: m.contactId, size: 40),
                  title: _Highlighted(text: m.body, query: _query.text, style: t.bodyLarge!),
                  subtitle: Text(
                    '${m.outgoing ? 'You to ${who?.displayName ?? ''}' : who?.displayName ?? ''} · ${fullStamp(context, m.createdAtSecs)}',
                  ),
                  onTap: () => _open(m),
                );
              },
            ),
    );
  }
}

/// Bolds the query's words where they start a word in the text.
class _Highlighted extends StatelessWidget {
  const _Highlighted({required this.text, required this.query, required this.style});
  final String text, query;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final terms = query.toLowerCase().split(RegExp(r'\s+')).where((w) => w.isNotEmpty).toList();
    final spans = <TextSpan>[];
    final lower = text.toLowerCase();
    var i = 0;
    while (i < text.length) {
      int? hit;
      var len = 0;
      for (final w in terms) {
        final j = lower.indexOf(w, i);
        if (j >= 0 && (hit == null || j < hit)) {
          hit = j;
          len = w.length;
        }
      }
      if (hit == null) {
        spans.add(TextSpan(text: text.substring(i)));
        break;
      }
      if (hit > i) spans.add(TextSpan(text: text.substring(i, hit)));
      spans.add(
        TextSpan(
          text: text.substring(hit, hit + len),
          style: TextStyle(fontWeight: FontWeight.w700, color: VlColors.of(context).accent),
        ),
      );
      i = hit + len;
    }
    return Text.rich(
      TextSpan(style: style, children: spans),
      maxLines: 3,
      overflow: TextOverflow.ellipsis,
    );
  }
}
