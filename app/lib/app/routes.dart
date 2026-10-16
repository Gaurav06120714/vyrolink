import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../features/contacts/contact_screen.dart';
import '../features/conversation/conversation_screen.dart';
import '../features/conversation/delivery_details_screen.dart';
import '../features/groups/group_screen.dart';
import '../features/groups/new_group_screen.dart';
import '../features/network/queue_screen.dart';
import '../features/pairing/pairing_screen.dart';
import '../features/search/search_screen.dart';
import '../features/settings/about_screen.dart';
import '../features/settings/developer_screen.dart';
import '../features/settings/my_id_screen.dart';
import '../features/settings/privacy_screen.dart';
import '../features/settings/panic_wipe_screen.dart';

/// Typed navigation: every destination is a method with the arguments it needs.
abstract final class Routes {
  static Future<T?> _push<T>(BuildContext context, String name, Widget screen) => Navigator.of(context).push<T>(
    MaterialPageRoute(
      settings: RouteSettings(name: name),
      builder: (_) => screen,
    ),
  );

  static Future<void> conversation(BuildContext context, Contact contact) =>
      _push(context, '/chat', ConversationScreen(contactId: contact.contactId, initialName: contact.displayName));

  static Future<void> contact(BuildContext context, Uint8List contactId) =>
      _push(context, '/contact', ContactScreen(contactId: contactId));

  static Future<void> pairing(BuildContext context) => _push(context, '/pair', const PairingScreen());

  static Future<void> deliveryDetails(BuildContext context, Uint8List messageId) =>
      _push(context, '/delivery', DeliveryDetailsScreen(messageId: messageId));

  static Future<void> group(BuildContext context, Group group) =>
      _push(context, '/group', GroupScreen(groupId: group.groupId, initialName: group.name));

  static Future<void> newGroup(BuildContext context) => _push(context, '/group/new', const NewGroupScreen());

  static Future<void> search(BuildContext context, {Uint8List? contactId}) =>
      _push(context, '/search', SearchScreen(contactId: contactId));

  static Future<void> queue(BuildContext context) => _push(context, '/queue', const QueueScreen());

  static Future<void> about(BuildContext context) => _push(context, '/about', const AboutScreen());

  static Future<void> myId(BuildContext context) => _push(context, '/me', const MyIdScreen());

  static Future<void> privacy(BuildContext context) => _push(context, '/privacy', const PrivacyScreen());

  static Future<void> panicWipe(BuildContext context) => _push(context, '/wipe', const PanicWipeScreen());

  static Future<void> developer(BuildContext context) => _push(context, '/dev', const DeveloperScreen());
}
