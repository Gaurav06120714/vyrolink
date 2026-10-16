import 'package:flutter/cupertino.dart' show CupertinoPageTransitionsBuilder;
import 'package:flutter/material.dart';

import 'tokens.dart';

/// Builds the light or dark theme from [VlColors]. System fonts (SF Pro on
/// iOS, Roboto on Android) keep Dynamic Type and font scaling native.
ThemeData buildTheme(Brightness brightness, TargetPlatform platform) {
  final c = brightness == Brightness.light ? VlColors.light : VlColors.dark;
  final ios = platform == TargetPlatform.iOS || platform == TargetPlatform.macOS;

  final scheme = ColorScheme(
    brightness: brightness,
    primary: c.accent,
    onPrimary: c.onAccent,
    secondary: c.accent,
    onSecondary: c.onAccent,
    error: c.err,
    onError: c.onAccent,
    surface: c.surface,
    onSurface: c.text,
    onSurfaceVariant: c.text2,
    surfaceContainerLowest: c.bg,
    surfaceContainerLow: c.bg,
    surfaceContainer: c.surface,
    surfaceContainerHigh: c.surface2,
    surfaceContainerHighest: c.surface2,
    outline: c.border,
    outlineVariant: c.divider,
    surfaceTint: Colors.transparent,
  );

  TextStyle s(double size, double height, FontWeight w, {double tracking = 0}) =>
      TextStyle(fontSize: size, height: height / size, fontWeight: w, letterSpacing: tracking, color: c.text);

  final text = TextTheme(
    displaySmall: s(28, 34, FontWeight.w700, tracking: -0.4),
    headlineSmall: s(22, 28, FontWeight.w600, tracking: -0.2),
    headlineMedium: s(32, 38, FontWeight.w700, tracking: -0.6),
    titleLarge: s(17, 22, FontWeight.w600),
    titleMedium: s(17, 22, FontWeight.w600),
    titleSmall: s(15, 20, FontWeight.w600),
    bodyLarge: s(16, 22, FontWeight.w400),
    bodyMedium: s(15, 20, FontWeight.w400),
    bodySmall: s(13, 18, FontWeight.w400).copyWith(color: c.text2),
    labelLarge: s(16, 22, FontWeight.w600),
    labelMedium: s(13, 18, FontWeight.w600),
    labelSmall: s(11, 14, FontWeight.w600, tracking: 0.2),
  );

  final shape12 = RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.card));

  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    platform: platform,
    colorScheme: scheme,
    scaffoldBackgroundColor: c.bg,
    canvasColor: c.bg,
    textTheme: text,
    extensions: [c],
    splashFactory: ios ? NoSplash.splashFactory : InkSparkle.splashFactory,
    highlightColor: ios ? c.text.withValues(alpha: 0.06) : null,
    materialTapTargetSize: MaterialTapTargetSize.padded,
    visualDensity: VisualDensity.standard,
    pageTransitionsTheme: const PageTransitionsTheme(
      builders: {
        TargetPlatform.iOS: CupertinoPageTransitionsBuilder(),
        TargetPlatform.android: PredictiveBackPageTransitionsBuilder(),
      },
    ),
    appBarTheme: AppBarTheme(
      backgroundColor: c.bg,
      foregroundColor: c.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      surfaceTintColor: Colors.transparent,
      centerTitle: ios,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: c.surface,
      indicatorColor: c.accent.withValues(alpha: 0.14),
      surfaceTintColor: Colors.transparent,
      elevation: 0,
      height: 64,
      labelTextStyle: WidgetStateProperty.resolveWith(
        (st) => text.labelSmall!.copyWith(color: st.contains(WidgetState.selected) ? c.accent : c.text2, fontSize: 12),
      ),
      iconTheme: WidgetStateProperty.resolveWith(
        (st) => IconThemeData(color: st.contains(WidgetState.selected) ? c.accent : c.text2, size: 24),
      ),
    ),
    navigationRailTheme: NavigationRailThemeData(
      backgroundColor: c.surface,
      indicatorColor: c.accent.withValues(alpha: 0.14),
      selectedIconTheme: IconThemeData(color: c.accent),
      unselectedIconTheme: IconThemeData(color: c.text2),
      selectedLabelTextStyle: text.labelMedium!.copyWith(color: c.accent),
      unselectedLabelTextStyle: text.labelMedium!.copyWith(color: c.text2),
    ),
    listTileTheme: ListTileThemeData(
      iconColor: c.text2,
      textColor: c.text,
      titleTextStyle: text.bodyLarge,
      subtitleTextStyle: text.bodySmall,
      minVerticalPadding: Space.m,
      contentPadding: const EdgeInsets.symmetric(horizontal: Space.l),
    ),
    dividerTheme: DividerThemeData(color: c.divider, thickness: 1, space: 1),
    iconTheme: IconThemeData(color: c.text2),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: c.accent,
        foregroundColor: c.onAccent,
        minimumSize: const Size(48, 50),
        shape: shape12,
        textStyle: text.labelLarge,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: c.text,
        minimumSize: const Size(48, 50),
        side: BorderSide(color: c.border),
        shape: shape12,
        textStyle: text.labelLarge,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: c.accent,
        minimumSize: const Size(48, 48),
        textStyle: text.labelLarge,
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(foregroundColor: c.text, minimumSize: const Size(48, 48)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: c.surface,
      hintStyle: text.bodyLarge!.copyWith(color: c.text2),
      contentPadding: const EdgeInsets.symmetric(horizontal: Space.l, vertical: Space.m),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        borderSide: BorderSide(color: c.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        borderSide: BorderSide(color: c.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(Radii.card),
        borderSide: BorderSide(color: c.accent, width: 2),
      ),
    ),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: c.surface,
      surfaceTintColor: Colors.transparent,
      showDragHandle: true,
      dragHandleColor: c.border,
      shape: const RoundedRectangleBorder(borderRadius: BorderRadius.vertical(top: Radius.circular(Radii.sheet))),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: c.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(Radii.sheet)),
      titleTextStyle: text.titleMedium,
      contentTextStyle: text.bodyMedium!.copyWith(color: c.text2),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: c.text,
      contentTextStyle: text.bodyMedium!.copyWith(color: c.bg),
      actionTextColor: c.bg,
      shape: shape12,
    ),
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: SegmentedButton.styleFrom(
        selectedBackgroundColor: c.accent.withValues(alpha: 0.14),
        selectedForegroundColor: c.accent,
        foregroundColor: c.text2,
        side: BorderSide(color: c.divider),
        textStyle: text.labelMedium,
        minimumSize: const Size(48, 44),
      ),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(color: c.accent),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: c.accent,
      foregroundColor: c.onAccent,
      elevation: 0,
      focusElevation: 0,
      hoverElevation: 0,
      highlightElevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
  );
}

/// Monospaced, tabular style for safety numbers and fingerprints.
TextStyle monoStyle(BuildContext context) => TextStyle(
  fontFamily: 'monospace',
  fontFamilyFallback: const ['Menlo', 'Courier'],
  fontSize: 15,
  height: 22 / 15,
  fontWeight: FontWeight.w500,
  fontFeatures: const [FontFeature.tabularFigures()],
  color: VlColors.of(context).text,
);
