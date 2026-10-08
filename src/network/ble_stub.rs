//! Non-macOS BLE stub.

use std::sync::mpsc::{self, Receiver};

#[derive(Clone, Debug)]
pub(crate) struct BlePeer {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) rssi: i16,
    pub(crate) last_seen: std::time::Instant,
}

pub(crate) fn start_scan() -> Receiver<Result<Vec<BlePeer>, String>> {
    let (tx, rx) = mpsc::channel();
    let _ = tx.send(Err("BLE proximity is available on macOS only".into()));
    rx
}

pub(crate) fn poll_scan(rx: &Receiver<Result<Vec<BlePeer>, String>>) -> Option<Result<Vec<BlePeer>, String>> {
    match rx.try_recv() {
        Ok(result) => Some(result),
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => Some(Err("BLE scanner worker disconnected".into())),
    }
}
