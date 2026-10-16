import Flutter
import UIKit

/// Registers the host API, the QR scanner view and the Bluetooth relay. The
/// node and relay live as long as the process, so iOS Bluetooth relaunches in
/// the background work without the UI.
public class VlHostPlugin: NSObject, FlutterPlugin {
  private let host = NodeHost()
  private let events: VlEventsApi

  init(messenger: FlutterBinaryMessenger) {
    events = VlEventsApi(binaryMessenger: messenger)
    super.init()
    let events = self.events
    host.relay = BleRelay(
      node: { [weak host] in host?.node },
      onDataChanged: { Task { @MainActor in try? await events.dataChanged() } },
      onStatus: { status in Task { @MainActor in try? await events.networkChanged(status: status) } })
    host.relay?.refreshBattery()
    host.openIfProvisioned()
    if let node = host.node, node.hasIdentity(),
      (try? node.setting(key: NodeHost.relaySetting)).flatMap({ $0 }) == "1"
    {
      host.relay?.setEnabled(true)
    }
  }

  public static func register(with registrar: FlutterPluginRegistrar) {
    let plugin = VlHostPlugin(messenger: registrar.messenger())
    VlHostApiSetup.setUp(binaryMessenger: registrar.messenger(), api: plugin.host)
    let events = plugin.events
    registrar.register(
      QrScannerFactory(onCode: { text in Task { @MainActor in try? await events.qrScanned(text: text) } }),
      withId: "vl_host/qr_scanner")
    registrar.publish(plugin)
    NotificationCenter.default.addObserver(
      forName: UIApplication.didBecomeActiveNotification, object: nil, queue: nil
    ) { [host = plugin.host] _ in
      DispatchQueue.global(qos: .utility).async { try? host.maintain() }
    }
  }
}
