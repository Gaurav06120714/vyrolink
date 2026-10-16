import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import 'app/app.dart';
import 'app/services.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  final services = AppServices(VlHostApi())..attach();
  runApp(VyroApp(services: services));
}
