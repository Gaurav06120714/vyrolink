import 'package:flutter/foundation.dart';
import 'package:google_sign_in/google_sign_in.dart';

/// OAuth client IDs from Google Cloud Console → APIs & Services → Credentials.
/// They are public identifiers, not secrets. Leave empty until created; the
/// app then says sign-in isn't set up instead of failing. On iOS the reversed
/// iOS client ID must also be a URL scheme in ios/Runner/Info.plist (SETUP.md).
const googleIosClientId = '427548154015-2il7ro2kqe58ol780j1a2l5b22gfmgj8.apps.googleusercontent.com';

/// The "Web application" client. Android's Credential Manager needs it as
/// `serverClientId`; the Android client itself is matched by package name and
/// signing-certificate SHA-1 in the console, not by anything in the app.
const googleWebClientId = '427548154015-j8ge2jg23rhue1fjlral5ivjlra6qeej.apps.googleusercontent.com';

/// Optional Google sign-in (ADR-030). It only reads the name and email Google
/// returns and keeps them in the encrypted settings on this phone. Nothing is
/// sent to VYRO LINK servers (there are none) or shared with contacts.
class GoogleAccount {
  static const emailKey = 'account.google.email';
  static const nameKey = 'account.google.name';

  /// Off in the web chat: its page loads nothing from other companies.
  static bool get configured =>
      !kIsWeb &&
      switch (defaultTargetPlatform) {
        TargetPlatform.iOS => googleIosClientId.isNotEmpty,
        TargetPlatform.android => googleWebClientId.isNotEmpty,
        _ => false,
      };

  static Future<void>? _ready;

  static Future<void> _init() => _ready ??= GoogleSignIn.instance.initialize(
    clientId: defaultTargetPlatform == TargetPlatform.iOS ? googleIosClientId : null,
    serverClientId: googleWebClientId.isEmpty ? null : googleWebClientId,
  );

  /// Shows Google's own sign-in sheet. Returns null if the person cancels.
  /// Throws [GoogleAccountFailure] with a message for anything else.
  static Future<({String email, String name})?> signIn() async {
    if (!configured) {
      throw const GoogleAccountFailure('Google sign-in isn\'t set up in this build yet.');
    }
    try {
      await _init();
      final a = await GoogleSignIn.instance.authenticate(scopeHint: const ['email']);
      return (email: a.email, name: (a.displayName ?? '').trim());
    } on GoogleSignInException catch (e) {
      return switch (e.code) {
        GoogleSignInExceptionCode.canceled || GoogleSignInExceptionCode.interrupted => null,
        GoogleSignInExceptionCode.clientConfigurationError || GoogleSignInExceptionCode.providerConfigurationError =>
          throw const GoogleAccountFailure('Google sign-in isn\'t set up correctly in this build.'),
        _ => throw const GoogleAccountFailure('Couldn\'t sign in. Google sign-in needs an internet connection.'),
      };
    }
  }

  /// Signs out and revokes the app's access at Google, so the Google SDK keeps
  /// no session on the phone. Errors are ignored: local data goes regardless.
  static Future<void> signOut() async {
    if (!configured) return;
    try {
      await _init();
      await GoogleSignIn.instance.disconnect();
    } on Object {
      // Offline or already signed out.
    }
  }
}

class GoogleAccountFailure implements Exception {
  const GoogleAccountFailure(this.message);
  final String message;
}
