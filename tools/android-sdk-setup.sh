#!/bin/sh
# Accept the Android SDK licences (interactive; you decide) and install the
# packages VYRO LINK needs. See SETUP.md §7.
set -e
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
SDK="$HOME/Library/Android/sdk"
SDKM=/opt/homebrew/share/android-commandlinetools/cmdline-tools/latest/bin/sdkmanager

"$SDKM" --sdk_root="$SDK" --licenses
"$SDKM" --sdk_root="$SDK" \
  "platform-tools" "platforms;android-36" "platforms;android-37.0" \
  "build-tools;36.0.0" "ndk;28.2.13676358" "cmdline-tools;latest" \
  "emulator" "system-images;android-37.0;google_apis;arm64-v8a"
flutter config --android-sdk "$SDK"
echo "Android SDK setup finished."
