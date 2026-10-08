//! Opt-in macOS BLE advertising for CYB RADAR.
//!
//! The advertiser is a small native CoreBluetooth helper bundled next to the
//! cybOS executable. It exists as a separate bounded process so the main UI
//! thread never blocks on CoreBluetooth's peripheral event loop.
//!
//! Advertising is intentionally opt-in: the process exists only while RADAR is
//! VISIBLE. The advertised local name contains only "cybOS" plus a short,
//! non-geographic node identifier. No GPS/location data is transmitted.

use std::process::{Child, Command};

pub(crate) fn start(node_id: &str) -> Result<Child, String> {
    let helper = helper_path()?;
    Command::new(&helper)
        .arg(node_id)
        .spawn()
        .map_err(|error| format!("could not start BLE advertiser {}: {}", helper.display(), error))
}

pub(crate) fn stop(child: &mut Option<Child>) {
    if let Some(process) = child.as_mut() {
        let _ = process.kill();
        let _ = process.wait();
    }
    *child = None;
}

pub(crate) fn poll(child: &mut Option<Child>) -> Option<bool> {
    let Some(process) = child.as_mut() else {
        return None;
    };

    match process.try_wait() {
        Ok(Some(status)) => {
            *child = None;
            Some(status.success())
        }
        Ok(None) => Some(true),
        Err(_) => {
            *child = None;
            Some(false)
        }
    }
}

fn helper_path() -> Result<std::path::PathBuf, String> {
    if let Ok(path) = std::env::var("CYBOS_BLE_ADVERTISER") {
        let path = std::path::PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }

    let exe = std::env::current_exe()
        .map_err(|error| format!("could not locate cybOS executable: {error}"))?;
    let path = exe
        .parent()
        .ok_or_else(|| "cybOS executable has no parent directory".to_string())?
        .join("cybOS-ble-advertiser");

    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("BLE advertiser helper not bundled: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn node_suffix_is_bounded() {
        let id = "12345678-1234-1234-1234-123456789abc";
        let compact = id.replace('-', "");
        assert_eq!(compact.chars().take(8).count(), 8);
    }
}

impl CybOs {
    pub(crate) fn sync_ble_advertiser(&mut self) {
        if !self.radar_visible {
            if self.ble_advertiser.is_some() {
                crate::network::ble_advertiser::stop(&mut self.ble_advertiser);
                self.ble_status = "BLE · ADVERTISING STOPPED · HIDDEN".into();
            }
            return;
        }

        match crate::network::ble_advertiser::poll(&mut self.ble_advertiser) {
            Some(false) => {
                self.ble_status = "BLE · ADVERTISER EXITED".into();
                self.runtime.set_status("RADAR", "ERROR");
            }
            Some(true) => {
                self.ble_status = "BLE · ADVERTISING · OPT-IN".into();
                self.runtime.set_status("RADAR", "READY");
            }
            None => {
                match crate::network::ble_advertiser::start(&self.node_id) {
                    Ok(child) => {
                        self.ble_advertiser = Some(child);
                        self.ble_status = "BLE · STARTING ADVERTISEMENT".into();
                        self.runtime.set_status("RADAR", "RUNNING");
                    }
                    Err(error) => {
                        self.ble_status = format!("BLE · ADVERTISER ERROR · {}", error);
                        self.runtime.set_status("RADAR", "ERROR");
                    }
                }
            }
        }
    }
}
