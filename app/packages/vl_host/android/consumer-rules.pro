# Applied to every app that uses vl_host (release builds shrink code with R8).
# JNA finds fields such as com.sun.jna.Pointer.peer and native callbacks by
# name; renaming or removing them breaks every call into the Rust core
# ("Can't obtain peer field ID for class com.sun.jna.Pointer").
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
# UniFFI-generated bindings are reached from native code and via JNA.
-keep class uniffi.** { *; }
-dontwarn java.awt.**
