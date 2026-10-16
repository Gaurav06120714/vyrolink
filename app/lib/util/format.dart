import 'package:flutter/material.dart';

DateTime fromSecs(int secs) => DateTime.fromMillisecondsSinceEpoch(secs * 1000);

bool _sameDay(DateTime a, DateTime b) => a.year == b.year && a.month == b.month && a.day == b.day;

String timeOfDay(BuildContext context, DateTime t) =>
    MaterialLocalizations.of(context)
        .formatTimeOfDay(TimeOfDay.fromDateTime(t), alwaysUse24HourFormat: MediaQuery.alwaysUse24HourFormatOf(context));

const _weekdays = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

/// Chat-list style: time today, "Yesterday", weekday this week, else a date.
String shortStamp(BuildContext context, int secs, {DateTime? now}) {
  final t = fromSecs(secs);
  final n = now ?? DateTime.now();
  if (_sameDay(t, n)) return timeOfDay(context, t);
  if (_sameDay(t, n.subtract(const Duration(days: 1)))) return 'Yesterday';
  if (n.difference(t).inDays < 6 && t.isBefore(n)) return _weekdays[t.weekday - 1].substring(0, 3);
  return MaterialLocalizations.of(context).formatShortDate(t);
}

/// Conversation day separator.
String dayLabel(BuildContext context, DateTime t, {DateTime? now}) {
  final n = now ?? DateTime.now();
  if (_sameDay(t, n)) return 'Today';
  if (_sameDay(t, n.subtract(const Duration(days: 1)))) return 'Yesterday';
  if (n.difference(t).inDays < 6 && t.isBefore(n)) return _weekdays[t.weekday - 1];
  return MaterialLocalizations.of(context).formatMediumDate(t);
}

String fullStamp(BuildContext context, int secs) {
  final t = fromSecs(secs);
  return '${MaterialLocalizations.of(context).formatMediumDate(t)}, ${timeOfDay(context, t)}';
}

String formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
}

/// Disappearing-message choices (seconds; 0 = off).
const timerChoices = <int, String>{
  0: 'Off',
  300: '5 minutes',
  3600: '1 hour',
  86400: '1 day',
  604800: '1 week',
  2592000: '30 days',
};

String timerLabel(int secs) => timerChoices[secs] ?? _durationLabel(secs);

String _durationLabel(int secs) {
  final d = Duration(seconds: secs);
  if (d.inDays >= 1) return '${d.inDays} days';
  if (d.inHours >= 1) return '${d.inHours} hours';
  return '${d.inMinutes} minutes';
}
