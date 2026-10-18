import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:vl_host/vl_host.dart';

import '../../app/failure.dart';
import '../../app/services.dart';
import '../../design/tokens.dart';
import '../../design/widgets/states.dart';
import '../../util/format.dart';

const attachmentImage = 1;
const attachmentFile = 2;

/// Reads a baseline/progressive JPEG's dimensions from its header without
/// decoding pixels. Returns null if the bytes are not a well-formed JPEG.
/// Received images are only decoded when this succeeds and the size is sane,
/// so a tiny file cannot claim a huge canvas (decompression bomb).
(int, int)? jpegSize(Uint8List b) {
  if (b.length < 4 || b[0] != 0xFF || b[1] != 0xD8) return null;
  var i = 2;
  while (i + 9 < b.length) {
    if (b[i] != 0xFF) return null;
    final marker = b[i + 1];
    final len = (b[i + 2] << 8) | b[i + 3];
    if (len < 2) return null;
    final isFrame = marker >= 0xC0 && marker <= 0xCF && marker != 0xC4 && marker != 0xC8 && marker != 0xCC;
    if (isFrame) {
      final h = (b[i + 5] << 8) | b[i + 6];
      final w = (b[i + 7] << 8) | b[i + 8];
      return (w == 0 || h == 0) ? null : (w, h);
    }
    i += 2 + len;
  }
  return null;
}

const maxImageSide = 4096;

/// Shows an attachment inside a bubble: the image, or a file row with a share
/// action, plus transfer progress while chunks are still arriving.
class AttachmentView extends StatefulWidget {
  const AttachmentView({super.key, required this.messageId, required this.info, required this.onBubble});

  final Uint8List messageId;
  final AttachmentInfo info;
  final bool onBubble;

  @override
  State<AttachmentView> createState() => _AttachmentViewState();
}

class _AttachmentViewState extends State<AttachmentView> {
  Uint8List? _data;
  bool _loading = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _maybeLoad();
  }

  @override
  void didUpdateWidget(AttachmentView old) {
    super.didUpdateWidget(old);
    _maybeLoad();
  }

  Future<void> _maybeLoad() async {
    if (!widget.info.complete || _data != null || _loading) return;
    _loading = true;
    try {
      final d = await context.services.call((api) => api.attachmentData(widget.messageId));
      if (mounted) setState(() => _data = d);
    } on VlFailure {
      // Shown as unavailable.
    } finally {
      _loading = false;
    }
  }

  Future<void> _share() async {
    final d = _data;
    if (d == null) return;
    try {
      await context.services.call((api) => api.shareFile(_name, d));
    } on VlFailure catch (e) {
      if (mounted) showMessage(context, e.message);
    }
  }

  String get _name =>
      widget.info.name.isEmpty ? (widget.info.kind == attachmentImage ? 'photo.jpg' : 'file') : widget.info.name;

  @override
  Widget build(BuildContext context) {
    final c = VlColors.of(context);
    final t = Theme.of(context).textTheme;
    final info = widget.info;
    final fg = widget.onBubble ? c.onBubbleOut : c.text;
    final data = _data;

    if (!info.complete) {
      final progress = info.chunkCount == 0 ? 0.0 : info.receivedChunks / info.chunkCount;
      return Semantics(
        label:
            'Receiving ${info.kind == attachmentImage ? 'photo' : 'file'}, ${info.receivedChunks} of ${info.chunkCount} parts',
        excludeSemantics: true,
        child: Container(
          width: 220,
          padding: const EdgeInsets.all(Space.m),
          margin: const EdgeInsets.only(bottom: Space.xs),
          decoration: BoxDecoration(color: fg.withValues(alpha: 0.08), borderRadius: BorderRadius.circular(Radii.card)),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Icon(
                    info.kind == attachmentImage ? Icons.image_outlined : Icons.description_outlined,
                    color: fg,
                    size: 20,
                  ),
                  const SizedBox(width: Space.s),
                  Expanded(
                    child: Text(
                      'Receiving ${info.receivedChunks} of ${info.chunkCount} parts',
                      style: t.bodySmall!.copyWith(color: fg),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: Space.s),
              LinearProgressIndicator(value: progress, backgroundColor: fg.withValues(alpha: 0.12), color: c.accent),
            ],
          ),
        ),
      );
    }

    if (info.kind == attachmentImage && data != null) {
      final size = jpegSize(data);
      if (size != null && size.$1 <= maxImageSide && size.$2 <= maxImageSide) {
        final (w, h) = size;
        final dpr = MediaQuery.devicePixelRatioOf(context);
        return Padding(
          padding: const EdgeInsets.only(bottom: Space.xs),
          child: Semantics(
            image: true,
            label: 'Photo',
            child: GestureDetector(
              onTap: () => _openViewer(context, data),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(Radii.card),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 260, maxHeight: 320),
                  child: AspectRatio(
                    aspectRatio: w / h,
                    child: Image.memory(
                      data,
                      fit: BoxFit.cover,
                      cacheWidth: (260 * dpr).round(),
                      gaplessPlayback: true,
                      errorBuilder: (_, _, _) => const SizedBox.shrink(),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      }
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: Space.xs),
      child: Material(
        color: fg.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(Radii.card),
        child: InkWell(
          borderRadius: BorderRadius.circular(Radii.card),
          onTap: data == null ? null : _share,
          child: Padding(
            padding: const EdgeInsets.all(Space.m),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  info.kind == attachmentImage ? Icons.broken_image_outlined : Icons.description_outlined,
                  color: fg,
                ),
                const SizedBox(width: Space.m),
                Flexible(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        _name,
                        style: t.titleSmall!.copyWith(color: fg),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        info.kind == attachmentImage
                            ? 'Image could not be shown · ${formatBytes(info.totalLen)}'
                            : formatBytes(info.totalLen),
                        style: t.bodySmall!.copyWith(color: fg.withValues(alpha: 0.8)),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: Space.s),
                Icon(Icons.ios_share_rounded, color: fg, size: 20, semanticLabel: 'Share'),
              ],
            ),
          ),
        ),
      ),
    );
  }

  void _openViewer(BuildContext context, Uint8List data) => Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => Scaffold(
        backgroundColor: Colors.black,
        appBar: AppBar(
          backgroundColor: Colors.black,
          foregroundColor: Colors.white,
          actions: [IconButton(tooltip: 'Share', onPressed: _share, icon: const Icon(Icons.ios_share_rounded))],
        ),
        body: Center(child: InteractiveViewer(child: Image.memory(data, gaplessPlayback: true))),
      ),
    ),
  );
}

/// What the + button can send besides text.
const shareLocation = 3;
const sharePoll = 4;

/// Photo, file, location (chats only) or poll.
Future<int?> chooseShare(BuildContext context, {bool location = true}) => showModalBottomSheet<int>(
  context: context,
  useSafeArea: true,
  builder: (sheet) => SafeArea(
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        ListTile(
          leading: const Icon(Icons.photo_outlined),
          title: const Text('Photo'),
          subtitle: const Text('Compressed to 16 KB so it can travel over Bluetooth'),
          onTap: () => Navigator.pop(sheet, attachmentImage),
        ),
        ListTile(
          leading: const Icon(Icons.description_outlined),
          title: const Text('File'),
          subtitle: const Text('Up to 64 KB'),
          onTap: () => Navigator.pop(sheet, attachmentFile),
        ),
        if (location)
          ListTile(
            leading: const Icon(Icons.location_on_outlined),
            title: const Text('Location'),
            subtitle: const Text('Your current position, once, encrypted'),
            onTap: () => Navigator.pop(sheet, shareLocation),
          ),
        ListTile(
          leading: const Icon(Icons.poll_outlined),
          title: const Text('Poll'),
          subtitle: const Text('A question with up to 10 answers'),
          onTap: () => Navigator.pop(sheet, sharePoll),
        ),
      ],
    ),
  ),
);

/// Picks a photo or file (`kind`) and confirms it (with an optional caption) before sending.
Future<(PickedFile, int, String)?> pickAttachment(BuildContext context, int kind) async {
  final s = context.services;
  PickedFile? picked;
  try {
    picked = await s.call((api) => kind == attachmentImage ? api.pickImage() : api.pickFile());
  } on VlFailure catch (e) {
    if (context.mounted) showMessage(context, e.message);
    return null;
  }
  if (picked == null || !context.mounted) return null;
  final data = picked.data;
  if (data == null) {
    showMessage(context, '${picked.name} is ${formatBytes(picked.size)}. Files can be at most 64 KB.');
    return null;
  }
  final caption = TextEditingController();
  final ok = await showModalBottomSheet<bool>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (sheet) => Padding(
      padding: EdgeInsets.fromLTRB(Space.xl, 0, Space.xl, MediaQuery.viewInsetsOf(sheet).bottom + Space.xl),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(kind == attachmentImage ? 'Send photo' : 'Send file', style: Theme.of(sheet).textTheme.headlineSmall),
          const SizedBox(height: Space.l),
          if (kind == attachmentImage && jpegSize(data) != null)
            ClipRRect(
              borderRadius: BorderRadius.circular(Radii.card),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 260),
                child: Image.memory(data, fit: BoxFit.contain, cacheWidth: 800),
              ),
            )
          else
            ListTile(
              leading: const Icon(Icons.description_outlined),
              title: Text(picked!.name),
              subtitle: Text(formatBytes(data.length)),
            ),
          const SizedBox(height: Space.s),
          Text(
            '${formatBytes(data.length)} · travels in ${(data.length / 1400).ceil()} encrypted parts. '
            'Large items take longer to reach people through other phones.',
            style: Theme.of(sheet).textTheme.bodySmall,
          ),
          const SizedBox(height: Space.m),
          TextField(
            controller: caption,
            maxLength: 1024,
            decoration: const InputDecoration(hintText: 'Add a caption (optional)'),
          ),
          FilledButton(onPressed: () => Navigator.pop(sheet, true), child: const Text('Send')),
        ],
      ),
    ),
  );
  final text = caption.text.trim();
  caption.dispose();
  return ok == true ? (picked, kind, text) : null;
}
