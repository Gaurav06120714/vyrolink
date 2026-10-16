import 'dart:async';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../design/widgets/states.dart';
import 'failure.dart';
import 'services.dart';

/// Loads data from the host and reloads whenever stored data changes.
/// Keeps showing the previous result while reloading, so lists never flash.
/// A spinner appears only if the first load takes longer than 300 ms.
class Live<T> extends StatefulWidget {
  const Live({super.key, required this.load, required this.builder, this.sliver = false});

  final Future<T> Function(VlHostApi api) load;

  /// The builder returns a sliver; placeholders are wrapped accordingly.
  final bool sliver;
  final Widget Function(BuildContext context, T data) builder;

  @override
  State<Live<T>> createState() => _LiveState<T>();
}

class _LiveState<T> extends State<Live<T>> {
  AppServices? _services;
  T? _data;
  bool _hasData = false;
  VlFailure? _error;
  bool _slow = false;
  int _generation = 0;
  Timer? _slowTimer;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final s = context.services;
    if (s != _services) {
      _services?.changes.removeListener(_reload);
      _services = s..changes.addListener(_reload);
      _reload();
    }
  }

  @override
  void dispose() {
    _services?.changes.removeListener(_reload);
    _slowTimer?.cancel();
    super.dispose();
  }

  Future<void> _reload() async {
    final gen = ++_generation;
    if (!_hasData) {
      _slowTimer?.cancel();
      _slowTimer = Timer(const Duration(milliseconds: 300), () {
        if (mounted && !_hasData) setState(() => _slow = true);
      });
    }
    try {
      final data = await _services!.call(widget.load);
      if (!mounted || gen != _generation) return;
      setState(() {
        _data = data;
        _hasData = true;
        _error = null;
      });
    } on VlFailure catch (e) {
      if (!mounted || gen != _generation) return;
      setState(() => _error = e);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_hasData) return widget.builder(context, _data as T);
    final placeholder = _error != null
        ? ErrorState(failure: _error!, onRetry: _reload)
        : _slow
        ? const Center(child: CircularProgressIndicator.adaptive())
        : const SizedBox.shrink();
    return widget.sliver ? SliverFillRemaining(hasScrollBody: false, child: placeholder) : placeholder;
  }
}
