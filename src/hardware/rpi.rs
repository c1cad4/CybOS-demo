#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpiStatus {
    pub supported: bool,
    pub detail: String,
}

#[cfg(all(feature = "hardware-rpi", target_os = "linux"))]
pub fn probe() -> RpiStatus {
    let detail = match rppal::gpio::Gpio::new() {
        Ok(_) => "Raspberry Pi GPIO interface available".to_string(),
        Err(error) => format!("Raspberry Pi GPIO unavailable: {error}"),
    };
    RpiStatus { supported: true, detail }
}

#[cfg(not(all(feature = "hardware-rpi", target_os = "linux")))]
pub fn probe() -> RpiStatus {
    RpiStatus {
        supported: false,
        detail: "Raspberry Pi adapter is disabled on this target".into(),
    }
}
