//! macOS CoreBluetooth proximity scanning for CYB RADAR.
//!
//! BLE reports RSSI, not GPS. cybOS exposes measured RSSI and does not invent
//! meter distances from an uncalibrated radio signal.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct BlePeer {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) rssi: i16,
    pub(crate) last_seen: Instant,
}

const SCAN_WINDOW: Duration = Duration::from_secs(5);
const PEER_TTL: Duration = Duration::from_secs(12);

pub(crate) fn start_scan() -> Receiver<Result<Vec<BlePeer>, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(scan_blocking());
    });
    rx
}

pub(crate) fn poll_scan(rx: &Receiver<Result<Vec<BlePeer>, String>>) -> Option<Result<Vec<BlePeer>, String>> {
    match rx.try_recv() {
        Ok(Ok(peers)) => Some(Ok(peers.into_iter().filter(|p| p.last_seen.elapsed() <= PEER_TTL).collect())),
        Ok(Err(error)) => Some(Err(error)),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(Err("BLE scanner worker disconnected".into())),
    }
}

fn scan_blocking() -> Result<Vec<BlePeer>, String> {
    use corebluetooth::{CBManagerState, CentralManager, CentralManagerDelegate, Peripheral, PeripheralDelegate};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Delegate {
        peers: Arc<Mutex<Vec<BlePeer>>>,
    }

    impl CentralManagerDelegate for Delegate {
        fn new_peripheral_delegate(&self) -> Box<dyn PeripheralDelegate> {
            Box::new(PeripheralDelegateImpl)
        }

        fn did_update_state(&self, _central: CentralManager) {}

        fn did_discover(
            &self,
            _central: CentralManager,
            peripheral: Peripheral,
            _advertisement_data: corebluetooth::advertisement_data::AdvertisementData,
            rssi: i16,
        ) {
            let Some(name) = peripheral.name() else { return; };
            if !name.to_ascii_lowercase().starts_with("cybos") { return; }

            let peer = BlePeer {
                device_id: peripheral.identifier().to_string(),
                name,
                rssi,
                last_seen: Instant::now(),
            };

            if let Ok(mut peers) = self.peers.lock() {
                if let Some(existing) = peers.iter_mut().find(|p| p.device_id == peer.device_id) {
                    *existing = peer;
                } else {
                    peers.push(peer);
                }
            }
        }
    }

    struct PeripheralDelegateImpl;
    impl PeripheralDelegate for PeripheralDelegateImpl {}

    let peers = Arc::new(Mutex::new(Vec::new()));
    let delegate = Delegate { peers: peers.clone() };

    let manager_state = std::sync::Arc::new(std::sync::Mutex::new(CBManagerState::Unknown));
    let state_ref = manager_state.clone();

    CentralManager::background(
        Default::default(),
        move |central| {
            *state_ref.lock().unwrap() = central.state();
            Box::new(delegate)
        },
        false,
        None,
        |central, _executor| {
            central.scan(None, true, None);
        },
    );

    thread::sleep(SCAN_WINDOW);

    let state = *manager_state.lock().unwrap();
    if state != CBManagerState::PoweredOn {
        return Err(format!("Bluetooth unavailable: {:?}", state));
    }

    peers.lock().map_err(|_| "BLE peer state poisoned".to_string()).map(|peers| peers.clone())
}
