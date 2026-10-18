import 'package:vl_host/vl_host.dart';

import 'web_host.dart';

/// The web chat: the Rust core in WebAssembly (ADR-040).
VlHostApi createHost() => WebHost();

void attachHostEvents(VlHostApi host, VlEventsApi events) {
  if (host is WebHost) host.events = events;
}
