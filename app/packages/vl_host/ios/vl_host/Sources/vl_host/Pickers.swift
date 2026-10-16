import Flutter
import LocalAuthentication
import PhotosUI
import UIKit
import UniformTypeIdentifiers

/// System pickers, the share sheet and device authentication. Everything runs
/// on the main thread and returns through async continuations.
@MainActor
enum Pickers {
  static let maxImageBytes = 16 * 1024
  static let maxFileBytes = 64 * 1024

  private static var topController: UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    var top = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController
    while let presented = top?.presentedViewController { top = presented }
    return top
  }

  // MARK: Images

  private final class ImageDelegate: NSObject, PHPickerViewControllerDelegate {
    var done: ((UIImage?) -> Void)?
    func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
      picker.dismiss(animated: true)
      guard let provider = results.first?.itemProvider, provider.canLoadObject(ofClass: UIImage.self) else {
        done?(nil)
        return
      }
      provider.loadObject(ofClass: UIImage.self) { object, _ in
        let image = object as? UIImage
        DispatchQueue.main.async { self.done?(image) }
      }
    }
  }

  private static var imageDelegate: ImageDelegate?

  /// Opens the photo picker (no photo-library permission needed) and returns a
  /// JPEG of at most 16 KiB. Re-encoding from pixels drops EXIF metadata such
  /// as location.
  static func pickImage() async -> PickedFile? {
    guard let top = topController else { return nil }
    var config = PHPickerConfiguration()
    config.filter = .images
    config.selectionLimit = 1
    let picker = PHPickerViewController(configuration: config)
    let delegate = ImageDelegate()
    imageDelegate = delegate
    picker.delegate = delegate
    let image: UIImage? = await withCheckedContinuation { cont in
      delegate.done = { cont.resume(returning: $0) }
      top.present(picker, animated: true)
    }
    imageDelegate = nil
    guard let image, let jpeg = compress(image) else { return nil }
    return PickedFile(name: "photo.jpg", size: Int64(jpeg.count), data: FlutterStandardTypedData(bytes: jpeg))
  }

  /// Shrinks and recompresses until the JPEG fits 16 KiB.
  static func compress(_ image: UIImage) -> Data? {
    var side: CGFloat = 640
    while side >= 120 {
      let scale = min(1, side / max(image.size.width, image.size.height))
      let size = CGSize(width: max(1, image.size.width * scale), height: max(1, image.size.height * scale))
      let format = UIGraphicsImageRendererFormat()
      format.scale = 1
      let resized = UIGraphicsImageRenderer(size: size, format: format).image { _ in
        image.draw(in: CGRect(origin: .zero, size: size))
      }
      for quality in stride(from: 0.7, through: 0.2, by: -0.1) {
        if let data = resized.jpegData(compressionQuality: quality), data.count <= maxImageBytes { return data }
      }
      side *= 0.75
    }
    return nil
  }

  // MARK: Files

  private final class DocumentDelegate: NSObject, UIDocumentPickerDelegate {
    var done: ((URL?) -> Void)?
    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
      done?(urls.first)
    }
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) { done?(nil) }
  }

  private static var documentDelegate: DocumentDelegate?

  static func pickFile() async -> PickedFile? {
    guard let top = topController else { return nil }
    let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.item], asCopy: true)
    let delegate = DocumentDelegate()
    documentDelegate = delegate
    picker.delegate = delegate
    let url: URL? = await withCheckedContinuation { cont in
      delegate.done = { cont.resume(returning: $0) }
      top.present(picker, animated: true)
    }
    documentDelegate = nil
    guard let url else { return nil }
    defer { try? FileManager.default.removeItem(at: url) }
    let size = (try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? 0
    let name = String(url.lastPathComponent.prefix(64))
    guard size > 0, size <= maxFileBytes, let data = try? Data(contentsOf: url), data.count <= maxFileBytes else {
      return PickedFile(name: name, size: Int64(size), data: nil)
    }
    return PickedFile(name: name, size: Int64(data.count), data: FlutterStandardTypedData(bytes: data))
  }

  // MARK: Share

  static func share(name: String, data: Data) async {
    guard let top = topController else { return }
    let safe = name.replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: ":", with: "_")
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent("share-\(UUID().uuidString)")
    let url = dir.appendingPathComponent(safe.isEmpty ? "attachment" : safe)
    do {
      try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
      try data.write(to: url, options: .completeFileProtection)
    } catch { return }
    let sheet = UIActivityViewController(activityItems: [url], applicationActivities: nil)
    sheet.popoverPresentationController?.sourceView = top.view
    sheet.popoverPresentationController?.sourceRect = CGRect(
      x: top.view.bounds.midX, y: top.view.bounds.midY, width: 0, height: 0)
    await withCheckedContinuation { (cont: CheckedContinuation<Void, Never>) in
      sheet.completionWithItemsHandler = { _, _, _, _ in
        try? FileManager.default.removeItem(at: dir)
        cont.resume()
      }
      top.present(sheet, animated: true)
    }
  }

  // MARK: App lock

  // LAContext policy checks are safe off the main thread (Pigeon calls this on a task queue).
  nonisolated static func canAuthenticate() -> Bool {
    LAContext().canEvaluatePolicy(.deviceOwnerAuthentication, error: nil)
  }

  /// Face ID / Touch ID with the device passcode as fallback.
  static func authenticate(reason: String) async -> Bool {
    (try? await LAContext().evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason)) ?? false
  }
}
