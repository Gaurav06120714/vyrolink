import 'package:flutter/services.dart';

/// Error codes produced by the native host: `VlError` case names from the
/// Rust core plus host-side `KeyStore`, `Bluetooth`, `NotStarted`,
/// `NotAvailable` and `Host`.
enum VlErrorCode {
  noIdentity('NoIdentity'),
  unknownContact('UnknownContact'),
  noPairingInProgress('NoPairingInProgress'),
  alreadyPaired('AlreadyPaired'),
  outOfOneTimeKeys('OutOfOneTimeKeys'),
  textTooLong('TextTooLong'),
  unknownLink('UnknownLink'),
  contactBlocked('ContactBlocked'),
  unknownMessage('UnknownMessage'),
  notAllowed('NotAllowed'),
  invalidReaction('InvalidReaction'),
  invalidTimer('InvalidTimer'),
  wiped('Wiped'),
  unknownGroup('UnknownGroup'),
  notGroupCreator('NotGroupCreator'),
  invalidGroupSize('InvalidGroupSize'),
  ownPairingCode('OwnPairingCode'),
  invalidInput('InvalidInput'),
  storage('Storage'),
  crypto('Crypto'),
  keyStore('KeyStore'),
  bluetooth('Bluetooth'),
  notStarted('NotStarted'),
  notAvailable('NotAvailable'),
  unknown('');

  const VlErrorCode(this.wire);
  final String wire;

  static VlErrorCode parse(String code) => values.firstWhere((v) => v.wire == code, orElse: () => VlErrorCode.unknown);
}

class VlFailure implements Exception {
  const VlFailure(this.code, [this.detail]);

  factory VlFailure.fromPlatform(PlatformException e) => VlFailure(VlErrorCode.parse(e.code), e.message);

  final VlErrorCode code;

  /// Technical detail for diagnostics; never shown as the main message and
  /// never contains message text or keys.
  final String? detail;

  /// Plain-language explanation for the user.
  String get message => switch (code) {
    VlErrorCode.alreadyPaired => 'You are already paired with this person.',
    VlErrorCode.ownPairingCode => 'That is your own code. Scan the code on the other phone.',
    VlErrorCode.noPairingInProgress => 'Show your code first, then scan theirs.',
    VlErrorCode.invalidInput => 'This is not a VYRO LINK code.',
    VlErrorCode.crypto => 'This code could not be verified. Scan it again.',
    VlErrorCode.outOfOneTimeKeys => 'Pair with this contact again in person to keep messaging.',
    VlErrorCode.textTooLong => 'This message is too long.',
    VlErrorCode.contactBlocked => 'You blocked this contact. Unblock them to send messages.',
    VlErrorCode.notAllowed => 'This message can no longer be changed.',
    VlErrorCode.unknownMessage => 'That message is no longer on this phone.',
    VlErrorCode.unknownContact => 'That contact is no longer on this phone.',
    VlErrorCode.unknownGroup => 'That group is no longer on this phone.',
    VlErrorCode.notGroupCreator => 'Only the person who created the group can change its members.',
    VlErrorCode.invalidGroupSize => 'A group has 2 to 8 people, including you.',
    VlErrorCode.invalidTimer => 'Choose a time between 1 minute and 1 year, or off.',
    VlErrorCode.keyStore =>
      'Secure storage is unavailable. VYRO LINK needs a device passcode to protect your messages.',
    VlErrorCode.storage => 'The encrypted storage could not be read or written.',
    VlErrorCode.wiped => 'This phone was wiped. Restart VYRO LINK to set it up again.',
    VlErrorCode.bluetooth => 'Bluetooth is not ready.',
    VlErrorCode.notAvailable => 'Not available in this build.',
    _ => 'Something went wrong. Please try again.',
  };

  @override
  String toString() => 'VlFailure(${code.name})';
}
