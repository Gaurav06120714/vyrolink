# Publishing VYRO LINK on Google Play

Prepared 18 October 2026. Play's rules change; re-check anything marked *verify* in Play Console.

## What is already done in the app

- **Target API:** API 37 (Play needs ≥ 36 for new apps from 31 August 2026).
- **Bundle:** `flutter build appbundle --release` builds `app/build/app/outputs/bundle/release/app-release.aab`, with arm64, 32-bit ARM and x86_64.
- **Release signing:** uses your upload key from `app/android/key.properties`. Without that file it falls back to the debug key, and Play rejects debug-signed bundles.
- **Permissions:** only what features need. Bluetooth scan is marked "never for location". Location is foreground only, after a tap. The foreground service type is `connectedDevice`.
- **Backups:** off. No ads, no analytics, no tracking SDKs.
- **Privacy policy:** live on the website, `#privacy` section. It covers the optional internet mailbox and the same-ID-after-reinstall copy.

## Steps only you can do

1. **Developer account:** create one at <https://play.google.com/console>, as a personal account. There is a one-time fee and identity verification (ID and possibly a phone check).
2. **Upload key:** create it on this Mac and pick your own password. Never share it, and back up the `.jks` file somewhere safe. If it is lost, you need Google's key-reset process.

   ```bash
   keytool -genkeypair -v -keystore ~/vyrolink-upload.jks -alias upload -keyalg RSA -keysize 4096 -validity 10000
   ```

   Then create `app/android/key.properties` (git ignores it):

   ```
   storeFile=/Users/apple/vyrolink-upload.jks
   storePassword=YOUR_PASSWORD
   keyAlias=upload
   keyPassword=YOUR_PASSWORD
   ```

3. **Build:** `cd app && flutter build appbundle --release`, then upload the `.aab`. Keep **Play App Signing** on (the default): Google holds the app signing key, and your key is only for uploads.
4. **Closed test, required for new personal accounts:** at least **12 testers opted in for 14 days in a row** before you can apply for production. If fewer than 12 are opted in, the clock can restart. Testers should really use it: Google now checks engagement (*verify*). Plan for 15+ testers to be safe. Each tester needs an Android phone and a Google account, and joins through the opt-in link.
5. **Apply for production** after the 14 days, answering Google's questions about the test.

## Store listing (ready to paste)

- **App name:** VYRO LINK
- **Short description (≤ 80 chars):** Encrypted messages that travel phone to phone over Bluetooth. No internet needed.
- **Full description:**

  > VYRO LINK lets you message people nearby with no SIM, no Wi-Fi and no mobile data. Each phone carries sealed, end-to-end encrypted messages and hands them to the next VYRO LINK phone it meets, until they reach the person they are for.
  >
  > • No account, no phone number: add contacts face to face by scanning a QR code
  > • End-to-end encrypted: phones that carry your message cannot read it
  > • Honest delivery: "Delivered" appears only when the other phone confirms it
  > • Replies, reactions, edits, groups of up to 8, polls, small photos and files
  > • Share your location once, only when you choose
  > • Optional: use the internet when available, to get messages through faster
  > • Panic wipe erases everything in one step
  >
  > Delivery depends on people meeting, so messages can take minutes or days.

- **Category:** Communication. **Tags:** messaging, privacy, offline.
- **Contact email:** hello@vyrolink.me. **Website:** your site. **Privacy policy URL:** `https://<your-site>/#privacy`
- **Screenshots:** at least 2 phone screenshots. Release builds block screenshots when that setting is on, so turn off Settings → Privacy → Block screenshots before taking them.
- **Graphics:** ready in `brand/`: `play-icon-512.png` (app icon) and `play-feature-graphic-1024x500.png` (feature graphic). They are drawn from exact geometry by `brand/make_brand.py`, which also writes the Android, iOS and web icons.

## App content forms

### Data safety
- **Does your app collect or share user data?** **No.**
  - Messages, photos, files, polls and locations are end-to-end encrypted, so no one in between (including you) can read them. Play's guidance says that data does not count as collected.
  - The Google sign-in name and email stay on the phone, so they are not collected.
- **Is data encrypted in transit?** **Yes.**
- **Can users request deletion?** Data lives only on the phone: delete it in the app, or use Panic wipe.
- **Mailbox IP addresses:** used only in memory for one minute for rate limiting. No logs, no location lookup. That is ephemeral processing (*verify* the wording in the form).

### Foreground service declaration (`connectedDevice`)
- **Feature:** "Bluetooth relay". After the user turns it on, the app keeps a Bluetooth LE connection service running, to exchange encrypted messages with nearby VYRO LINK phones. A persistent notification shows while it runs.
- **User impact if deferred:** messages the user sent, and messages carried for others, stop moving while the phone is in a pocket. Delivery would wait until the app is opened.
- **Video (30–60 s):**
  1. Open Network → Relay settings and choose "Always".
  2. Show the relaying notification.
  3. Lock the phone and keep it near a second phone.
  4. Show the message reaching "Delivered".

### Other declarations
- **Location permissions:** foreground only, so no special declaration is needed. If asked: "One position, read only after the user taps Share location in a chat and confirms."
- **Content rating questionnaire:** Communication app.
  - Users can communicate with each other: **yes**.
  - Users can share their location: **yes**, with chosen contacts only.
  - No violence, gambling and so on.
  - Expected rating: **Teen / PEGI 12** (*verify*).
- **Target audience:** 13+ (choose 18+ if you prefer). Not designed for children.
- **Ads:** none.
- **App access:** everything works without login. Note for reviewers: "Pair two phones by scanning each other's QR code in Contacts → Add contact."
- **User-generated content:** private messages only, with no public feed. Blocking is in each contact's profile.
- **Government / financial / health:** none.

### Encryption export (outside Play)
The app uses standard, published encryption (Olm, ChaCha20-Poly1305, Noise). Distribution from the US can need a one-time notice to the US Bureau of Industry and Security, under the ENC exception for mass-market or publicly available encryption. Rules differ by country, so check what applies to you. This is not legal advice.
