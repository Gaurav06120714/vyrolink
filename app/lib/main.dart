import 'package:flutter/material.dart';

import 'app/app.dart';
import 'app/services.dart';
import 'web/host.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final host = createHost();
  final services = AppServices(host)..attach();
  attachHostEvents(host, services);
  runApp(VyroApp(services: services));
}
