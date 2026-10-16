import AVFoundation
import Flutter
import UIKit

/// Camera preview that reports QR text (pairing codes) through `onCode`.
/// Uses AVFoundation's built-in QR detector: no third-party scanner, nothing
/// leaves the device. The camera runs only while this view exists.
final class QrScanner: NSObject, FlutterPlatformView, AVCaptureMetadataOutputObjectsDelegate {
  static func permission() -> PermissionState {
    switch AVCaptureDevice.authorizationStatus(for: .video) {
    case .authorized: return .granted
    case .denied: return .denied
    case .restricted: return .restricted
    default: return .notDetermined
    }
  }

  static func requestPermission() async -> PermissionState {
    await AVCaptureDevice.requestAccess(for: .video) ? .granted : .denied
  }

  private final class Preview: UIView {
    override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }
    var previewLayer: AVCaptureVideoPreviewLayer? { layer as? AVCaptureVideoPreviewLayer }
  }

  private let preview = Preview()
  private let session = AVCaptureSession()
  private let sessionQueue = DispatchQueue(label: "com.vyrolink.camera")
  private let onCode: (String) -> Void
  private var lastCode: String?
  private var lastAt = Date.distantPast

  init(onCode: @escaping (String) -> Void) {
    self.onCode = onCode
    super.init()
    preview.backgroundColor = .black
    guard Self.permission() == .granted,
      let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back),
      let input = try? AVCaptureDeviceInput(device: device), session.canAddInput(input)
    else { return }
    session.addInput(input)
    let output = AVCaptureMetadataOutput()
    guard session.canAddOutput(output) else { return }
    session.addOutput(output)
    output.setMetadataObjectsDelegate(self, queue: .main)
    output.metadataObjectTypes = [.qr]
    preview.previewLayer?.session = session
    preview.previewLayer?.videoGravity = .resizeAspectFill
    sessionQueue.async { [session] in session.startRunning() }
  }

  deinit {
    sessionQueue.async { [session] in session.stopRunning() }
  }

  func view() -> UIView { preview }

  func metadataOutput(
    _ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject], from connection: AVCaptureConnection
  ) {
    for case let code as AVMetadataMachineReadableCodeObject in metadataObjects {
      guard let text = code.stringValue else { continue }
      // The same code stays in view for a while; report it once per 2 s.
      if text == lastCode, Date().timeIntervalSince(lastAt) < 2 { continue }
      lastCode = text
      lastAt = Date()
      onCode(text)
    }
  }
}

final class QrScannerFactory: NSObject, FlutterPlatformViewFactory {
  private let onCode: (String) -> Void
  init(onCode: @escaping (String) -> Void) { self.onCode = onCode }

  func create(withFrame frame: CGRect, viewIdentifier viewId: Int64, arguments args: Any?) -> FlutterPlatformView {
    QrScanner(onCode: onCode)
  }
}
