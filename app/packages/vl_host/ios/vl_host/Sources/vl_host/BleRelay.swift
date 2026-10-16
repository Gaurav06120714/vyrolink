import CoreBluetooth
import Foundation
import UIKit
import UserNotifications
import VlCore

/// Doc 21 L0: the only thing VYRO LINK advertises. No name, no identifiers.
let vlServiceUUID = CBUUID(string: "3B29F840-0F6B-4221-9AEB-88FF70CCF5CD")
/// One characteristic per link: the central writes (without response), the
/// peripheral notifies. Bytes are the L1 frame stream; Rust reassembles them.
let vlLinkUUID = CBUUID(string: "60EAE795-7B2A-49EE-B8C2-7430FF5FF590")

/// Dual-role Bluetooth transport. Rust decides what to send (ADR-022); this
/// class only moves bytes, slices them to the negotiated size, waits for the
/// platform's flow control and ends sessions once both sides go quiet
/// (research/algorithms/06). iOS limits background work: it may pause
/// scanning, coalesce discoveries and never relaunches a force-quit app.
final class BleRelay: NSObject {
  static let idleSeconds: TimeInterval = 4
  static let sessionLimit: TimeInterval = 60
  static let cooldown: TimeInterval = 45
  static let maxCentralLinks = 2
  static let maintainEvery: TimeInterval = 15 * 60

  private let queue = DispatchQueue(label: "com.vyrolink.ble")
  private let nodeProvider: () -> VlNode?
  private let onDataChanged: () -> Void
  private let onStatus: (NetworkStatus) -> Void

  private var central: CBCentralManager?
  private var peripheralManager: CBPeripheralManager?
  private var linkCharacteristic: CBMutableCharacteristic?
  private var enabled = false
  private var serviceAdded = false

  private final class Session {
    let linkId: UInt64
    let started = Date()
    var lastActivity = Date()
    var outbox = Data()
    var exchanged = false
    var peripheral: CBPeripheral?
    var characteristic: CBCharacteristic?
    var central: CBCentral?
    init(linkId: UInt64) { self.linkId = linkId }
  }

  private var asCentral: [UUID: Session] = [:]
  private var connecting: [UUID: CBPeripheral] = [:]
  private var asPeripheral: [UUID: Session] = [:]
  private var cooldownUntil: [UUID: Date] = [:]
  /// Last time each (rotating) peer identifier was seen; only the count is reported.
  private var seenAt: [UUID: Date] = [:]
  private var encounters = 0
  private var lastExchange: Date?
  private var lastMaintain = Date.distantPast
  private var lastUnread: Int?
  private var timer: DispatchSourceTimer?
  private var changePending = false

  init(
    node: @escaping () -> VlNode?, onDataChanged: @escaping () -> Void,
    onStatus: @escaping (NetworkStatus) -> Void
  ) {
    nodeProvider = node
    self.onDataChanged = onDataChanged
    self.onStatus = onStatus
  }

  private var now: UInt64 { UInt64(Date().timeIntervalSince1970) }

  /// Updated on the main thread; read when building a status snapshot.
  private var batteryState: UIDevice.BatteryState = .unknown

  func refreshBattery() {
    DispatchQueue.main.async {
      UIDevice.current.isBatteryMonitoringEnabled = true
      let state = UIDevice.current.batteryState
      self.queue.async { self.batteryState = state }
    }
  }

  // MARK: Control

  func setEnabled(_ on: Bool) {
    queue.async {
      self.enabled = on
      if on {
        if self.central == nil {
          self.central = CBCentralManager(
            delegate: self, queue: self.queue,
            options: [CBCentralManagerOptionRestoreIdentifierKey: "com.vyrolink.central"])
          self.peripheralManager = CBPeripheralManager(
            delegate: self, queue: self.queue,
            options: [CBPeripheralManagerOptionRestoreIdentifierKey: "com.vyrolink.peripheral"])
        }
        self.startRadio()
        self.startTimer()
      } else {
        self.stopRadio()
      }
      self.publish()
    }
  }

  /// Stops everything synchronously (panic wipe).
  func shutdown() {
    queue.sync {
      enabled = false
      stopRadio()
    }
  }

  /// New local mail: forget cooldowns so the next discovery connects at once.
  func outboxChanged() { queue.async { self.cooldownUntil.removeAll() } }

  func notifyDataChanged() { DispatchQueue.main.async { self.onDataChanged() } }

  func status() -> NetworkStatus {
    let stats = try? nodeProvider()?.stats()
    return queue.sync { snapshot(stats) }
  }

  /// Call on `queue` only.
  private func snapshot(_ stats: NodeStats?) -> NetworkStatus {
    NetworkStatus(
      radio: radioState(), relayEnabled: enabled, relayMode: 2,
      charging: [.charging, .full].contains(batteryState),
      devicesSeenRecently: Int64(seenAt.values.filter { $0 > Date().addingTimeInterval(-86_400) }.count),
      scanning: central?.isScanning ?? false,
      advertising: peripheralManager?.isAdvertising ?? false,
      openLinks: Int64(asCentral.count + asPeripheral.count), encountersSinceLaunch: Int64(encounters),
      lastExchangeSecs: lastExchange.map { Int64($0.timeIntervalSince1970) },
      ownEnvelopes: Int64(stats?.ownEnvelopes ?? 0), carriedEnvelopes: Int64(stats?.carriedEnvelopes ?? 0),
      carriedBytes: Int64(clamping: stats?.carriedBytes ?? 0))
  }

  private func radioState() -> RadioState {
    switch CBManager.authorization {
    case .denied, .restricted: return .unauthorized
    case .notDetermined: if central == nil { return .unknown }
    default: break
    }
    switch central?.state ?? .unknown {
    case .poweredOn: return .poweredOn
    case .poweredOff: return .poweredOff
    case .unauthorized: return .unauthorized
    case .unsupported: return .unsupported
    default: return .unknown
    }
  }

  /// Call on `queue` only.
  private func publish() {
    let s = snapshot(try? nodeProvider()?.stats())
    DispatchQueue.main.async { self.onStatus(s) }
  }

  private func startRadio() {
    guard enabled else { return }
    if let central, central.state == .poweredOn, !central.isScanning {
      central.scanForPeripherals(
        withServices: [vlServiceUUID], options: [CBCentralManagerScanOptionAllowDuplicatesKey: true])
    }
    if let pm = peripheralManager, pm.state == .poweredOn {
      if !serviceAdded {
        let c = CBMutableCharacteristic(
          type: vlLinkUUID, properties: [.writeWithoutResponse, .notify], value: nil, permissions: [.writeable])
        let service = CBMutableService(type: vlServiceUUID, primary: true)
        service.characteristics = [c]
        linkCharacteristic = c
        serviceAdded = true
        pm.add(service)
      } else if !pm.isAdvertising {
        pm.startAdvertising([CBAdvertisementDataServiceUUIDsKey: [vlServiceUUID]])
      }
    }
  }

  private func stopRadio() {
    central?.stopScan()
    peripheralManager?.stopAdvertising()
    for (_, p) in connecting { central?.cancelPeripheralConnection(p) }
    connecting.removeAll()
    for (id, s) in asCentral {
      if let p = s.peripheral { central?.cancelPeripheralConnection(p) }
      finish(id, s, asCentral: true)
    }
    for (id, s) in asPeripheral { finish(id, s, asCentral: false) }
    timer?.cancel()
    timer = nil
  }

  private func startTimer() {
    guard timer == nil else { return }
    let t = DispatchSource.makeTimerSource(queue: queue)
    t.schedule(deadline: .now() + 1, repeating: 1)
    t.setEventHandler { [weak self] in self?.tick() }
    t.resume()
    timer = t
  }

  /// Ends quiet or overlong sessions and runs periodic maintenance.
  private func tick() {
    let nowDate = Date()
    let node = nodeProvider()
    for (id, s) in asCentral {
      let closed = !(node?.linkIsOpen(linkId: s.linkId) ?? false)
      let quiet = s.outbox.isEmpty && nowDate.timeIntervalSince(s.lastActivity) > Self.idleSeconds
      if (closed && s.outbox.isEmpty) || quiet || nowDate.timeIntervalSince(s.started) > Self.sessionLimit {
        if let p = s.peripheral { central?.cancelPeripheralConnection(p) }
        finish(id, s, asCentral: true)
      }
    }
    for (id, s) in asPeripheral where nowDate.timeIntervalSince(s.lastActivity) > Self.sessionLimit {
      finish(id, s, asCentral: false)
    }
    if nowDate.timeIntervalSince(lastMaintain) > Self.maintainEvery, let node {
      lastMaintain = nowDate
      try? node.maintain(nowSecs: now)
    }
  }

  // MARK: Byte pump

  private func receive(_ s: Session, _ bytes: Data) {
    guard let node = nodeProvider() else { return }
    s.lastActivity = Date()
    do {
      let reply = try node.linkReceive(linkId: s.linkId, bytes: bytes, nowSecs: now)
      s.exchanged = true
      s.outbox.append(reply)
      pump(s)
      scheduleChange()
    } catch {
      // Protocol or crypto failure: Rust already closed the link; drop the connection.
      s.outbox.removeAll()
      if let p = s.peripheral { central?.cancelPeripheralConnection(p) }
    }
  }

  private func pump(_ s: Session) {
    if let p = s.peripheral, let c = s.characteristic {
      let max = p.maximumWriteValueLength(for: .withoutResponse)
      while !s.outbox.isEmpty, p.canSendWriteWithoutResponse {
        let chunk = s.outbox.prefix(max)
        p.writeValue(Data(chunk), for: c, type: .withoutResponse)
        s.outbox.removeFirst(chunk.count)
        s.lastActivity = Date()
      }
    } else if let central = s.central, let pm = peripheralManager, let c = linkCharacteristic {
      while !s.outbox.isEmpty {
        let chunk = s.outbox.prefix(central.maximumUpdateValueLength)
        guard pm.updateValue(Data(chunk), for: c, onSubscribedCentrals: [central]) else { break }
        s.outbox.removeFirst(chunk.count)
        s.lastActivity = Date()
      }
    }
  }

  private func finish(_ id: UUID, _ s: Session, asCentral: Bool) {
    nodeProvider()?.linkClose(linkId: s.linkId)
    if asCentral {
      self.asCentral[id] = nil
      cooldownUntil[id] = Date().addingTimeInterval(Self.cooldown)
    } else {
      asPeripheral[id] = nil
    }
    if s.exchanged {
      encounters += 1
      lastExchange = Date()
      notifyIfBackgrounded()
    }
    scheduleChange()
  }

  /// Coalesces change events to at most one every half second.
  private func scheduleChange() {
    guard !changePending else { return }
    changePending = true
    queue.asyncAfter(deadline: .now() + 0.5) {
      self.changePending = false
      self.notifyDataChanged()
      self.publish()
    }
  }

  /// A generic local notification when mail arrived while the app was not in
  /// front. Never includes the sender or the text (Doc 16 §11 default).
  private func notifyIfBackgrounded() {
    guard let summaries = try? nodeProvider()?.conversationSummaries() else { return }
    let unread = summaries.reduce(0) { $0 + Int($1.unread) }
    defer { lastUnread = unread }
    guard let before = lastUnread, unread > before else { return }
    DispatchQueue.main.async {
      guard UIApplication.shared.applicationState != .active else { return }
      let content = UNMutableNotificationContent()
      content.title = "VYRO LINK"
      content.body = "You have a new message."
      content.sound = .default
      UNUserNotificationCenter.current().add(
        UNNotificationRequest(identifier: "vl.new-message", content: content, trigger: nil))
    }
  }
}

// MARK: Central role

extension BleRelay: CBCentralManagerDelegate, CBPeripheralDelegate {
  func centralManagerDidUpdateState(_ central: CBCentralManager) {
    if central.state == .poweredOn { startRadio() } else {
      for (id, s) in asCentral { finish(id, s, asCentral: true) }
      connecting.removeAll()
    }
    publish()
  }

  func centralManager(_ central: CBCentralManager, willRestoreState dict: [String: Any]) {
    // Sessions do not survive a relaunch; start clean.
    for p in dict[CBCentralManagerRestoredStatePeripheralsKey] as? [CBPeripheral] ?? [] {
      central.cancelPeripheralConnection(p)
    }
  }

  func centralManager(
    _ central: CBCentralManager, didDiscover peripheral: CBPeripheral, advertisementData: [String: Any],
    rssi RSSI: NSNumber
  ) {
    let id = peripheral.identifier
    seenAt[id] = Date()
    guard enabled, nodeProvider() != nil, asCentral[id] == nil, connecting[id] == nil,
      asCentral.count + connecting.count < Self.maxCentralLinks,
      (cooldownUntil[id] ?? .distantPast) < Date()
    else { return }
    connecting[id] = peripheral
    central.connect(peripheral)
  }

  func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
    peripheral.delegate = self
    peripheral.discoverServices([vlServiceUUID])
  }

  func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
    connecting[peripheral.identifier] = nil
    cooldownUntil[peripheral.identifier] = Date().addingTimeInterval(10)
  }

  func centralManager(
    _ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?
  ) {
    let id = peripheral.identifier
    connecting[id] = nil
    if let s = asCentral[id] { finish(id, s, asCentral: true) }
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
    guard error == nil, let service = peripheral.services?.first(where: { $0.uuid == vlServiceUUID }) else {
      central?.cancelPeripheralConnection(peripheral)
      return
    }
    peripheral.discoverCharacteristics([vlLinkUUID], for: service)
  }

  func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
    guard error == nil, let c = service.characteristics?.first(where: { $0.uuid == vlLinkUUID }) else {
      central?.cancelPeripheralConnection(peripheral)
      return
    }
    peripheral.setNotifyValue(true, for: c)
  }

  func peripheral(
    _ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic, error: Error?
  ) {
    let id = peripheral.identifier
    guard error == nil, characteristic.isNotifying, let node = nodeProvider(),
      let start = try? node.linkOpen(initiator: true)
    else {
      central?.cancelPeripheralConnection(peripheral)
      return
    }
    connecting[id] = nil
    let s = Session(linkId: start.linkId)
    s.peripheral = peripheral
    s.characteristic = characteristic
    s.outbox.append(start.bytesToSend)
    asCentral[id] = s
    pump(s)
    publish()
  }

  func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
    guard error == nil, let s = asCentral[peripheral.identifier], let value = characteristic.value else { return }
    receive(s, value)
  }

  func peripheralIsReady(toSendWriteWithoutResponse peripheral: CBPeripheral) {
    if let s = asCentral[peripheral.identifier] { pump(s) }
  }
}

// MARK: Peripheral role

extension BleRelay: CBPeripheralManagerDelegate {
  func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
    if peripheral.state == .poweredOn { startRadio() } else {
      for (id, s) in asPeripheral { finish(id, s, asCentral: false) }
      if peripheral.state == .resetting || peripheral.state == .poweredOff { serviceAdded = false }
    }
    publish()
  }

  func peripheralManager(_ peripheral: CBPeripheralManager, willRestoreState dict: [String: Any]) {
    let services = dict[CBPeripheralManagerRestoredStateServicesKey] as? [CBMutableService] ?? []
    if let c = services.first(where: { $0.uuid == vlServiceUUID })?.characteristics?
      .first(where: { $0.uuid == vlLinkUUID }) as? CBMutableCharacteristic
    {
      linkCharacteristic = c
      serviceAdded = true
    }
  }

  func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
    guard error == nil else {
      serviceAdded = false
      return
    }
    if enabled { peripheral.startAdvertising([CBAdvertisementDataServiceUUIDsKey: [vlServiceUUID]]) }
  }

  func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) { publish() }

  private func responderSession(for central: CBCentral) -> Session? {
    if let s = asPeripheral[central.identifier] { return s }
    seenAt[central.identifier] = Date()
    guard enabled, let start = try? nodeProvider()?.linkOpen(initiator: false) else { return nil }
    let s = Session(linkId: start.linkId)
    s.central = central
    s.outbox.append(start.bytesToSend)
    asPeripheral[central.identifier] = s
    publish()
    return s
  }

  func peripheralManager(
    _ peripheral: CBPeripheralManager, central: CBCentral, didSubscribeTo characteristic: CBCharacteristic
  ) {
    if let s = responderSession(for: central) { pump(s) }
  }

  func peripheralManager(
    _ peripheral: CBPeripheralManager, central: CBCentral, didUnsubscribeFrom characteristic: CBCharacteristic
  ) {
    if let s = asPeripheral[central.identifier] { finish(central.identifier, s, asCentral: false) }
  }

  func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
    for r in requests where r.characteristic.uuid == vlLinkUUID {
      guard let value = r.value, let s = responderSession(for: r.central) else { continue }
      receive(s, value)
    }
  }

  func peripheralManagerIsReady(toUpdateSubscribers peripheral: CBPeripheralManager) {
    for s in asPeripheral.values { pump(s) }
  }
}
