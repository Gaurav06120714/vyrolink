// Picks the host: the browser one on the web, the native plugin elsewhere.
export 'host_native.dart' if (dart.library.js_interop) 'web_host_entry.dart';
