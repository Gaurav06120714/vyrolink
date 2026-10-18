import CoreLocation
import UIKit

/// One GPS fix for "Share location" (ADR-034). Location is asked for only
/// when the user taps "Share location"; Bluetooth never uses it.
@MainActor
final class LocationFetcher: NSObject, CLLocationManagerDelegate {
  static let shared = LocationFetcher()
  private let manager = CLLocationManager()
  private var authWaiters: [CheckedContinuation<Void, Never>] = []
  private var fixWaiters: [CheckedContinuation<LocationInfo?, Never>] = []

  func fix() async -> LocationInfo? {
    manager.delegate = self
    manager.desiredAccuracy = kCLLocationAccuracyBest
    if manager.authorizationStatus == .notDetermined {
      await withCheckedContinuation { c in
        authWaiters.append(c)
        manager.requestWhenInUseAuthorization()
      }
    }
    guard manager.authorizationStatus == .authorizedWhenInUse || manager.authorizationStatus == .authorizedAlways
    else { return nil }
    return await withCheckedContinuation { c in
      fixWaiters.append(c)
      manager.requestLocation()
    }
  }

  private func finish(_ fix: LocationInfo?) {
    let waiting = fixWaiters
    fixWaiters.removeAll()
    for c in waiting { c.resume(returning: fix) }
  }

  nonisolated func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
    let decided = manager.authorizationStatus != .notDetermined
    Task { @MainActor in
      guard decided else { return }
      let waiting = self.authWaiters
      self.authWaiters.removeAll()
      for c in waiting { c.resume() }
    }
  }

  nonisolated func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
    guard let l = locations.last else { return }
    let fix = LocationInfo(
      latE7: Int64((l.coordinate.latitude * 1e7).rounded()), lonE7: Int64((l.coordinate.longitude * 1e7).rounded()),
      accuracyM: Int64(min(max(l.horizontalAccuracy, 0), 65_535)),
      capturedMs: Int64(l.timestamp.timeIntervalSince1970 * 1000))
    Task { @MainActor in self.finish(fix) }
  }

  nonisolated func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
    Task { @MainActor in self.finish(nil) }
  }
}
