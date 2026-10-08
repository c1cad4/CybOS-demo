import CoreBluetooth
import Foundation

final class Advertiser: NSObject, CBPeripheralManagerDelegate {
    private var manager: CBPeripheralManager!
    private let name: String

    init(nodeID: String) {
        let compact = nodeID.replacingOccurrences(of: "-", with: "")
        let suffix = String(compact.prefix(8)).uppercased()
        self.name = "cybOS-" + suffix
        super.init()
        manager = CBPeripheralManager(delegate: self, queue: DispatchQueue.main)
    }

    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        switch peripheral.state {
        case .poweredOn:
            peripheral.startAdvertising([
                CBAdvertisementDataLocalNameKey: name
            ])
        case .poweredOff, .unauthorized, .unsupported:
            fputs("BLE unavailable: \(peripheral.state.rawValue)\n", stderr)
            exit(2)
        case .resetting, .unknown:
            break
        @unknown default:
            break
        }
    }

    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        if let error {
            fputs("BLE advertising failed: \(error.localizedDescription)\n", stderr)
            exit(3)
        }
        fputs("BLE advertising: \(name)\n", stdout)
        fflush(stdout)
    }
}

guard CommandLine.arguments.count >= 2 else {
    fputs("usage: cybOS-ble-advertiser NODE_ID\n", stderr)
    exit(64)
}

_ = Advertiser(nodeID: CommandLine.arguments[1])
RunLoop.main.run()
