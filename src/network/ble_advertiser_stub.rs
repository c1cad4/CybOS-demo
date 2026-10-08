//! Non-macOS BLE advertiser stub.

use std::process::Child;

pub(crate) fn start(_node_id: &str) -> Result<Child, String> {
    Err("BLE advertising is available on macOS only".into())
}

pub(crate) fn stop(child: &mut Option<Child>) {
    *child = None;
}

pub(crate) fn poll(_child: &mut Option<Child>) -> Option<bool> {
    None
}
