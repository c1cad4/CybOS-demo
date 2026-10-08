#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMode {
    Normal,
    Conserve,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyPolicy {
    pub mode: PowerMode,
    pub camera_poll_ms: u64,
    pub telemetry_poll_ms: u64,
    pub ui_power_saver: bool,
}

impl EnergyPolicy {
    pub fn from_battery_percent(percent: f32) -> Self {
        if percent <= 10.0 {
            Self { mode: PowerMode::Critical, camera_poll_ms: 30_000, telemetry_poll_ms: 15_000, ui_power_saver: true }
        } else if percent <= 25.0 {
            Self { mode: PowerMode::Conserve, camera_poll_ms: 10_000, telemetry_poll_ms: 5_000, ui_power_saver: true }
        } else {
            Self { mode: PowerMode::Normal, camera_poll_ms: 2_000, telemetry_poll_ms: 1_000, ui_power_saver: false }
        }
    }
}

pub fn crc16_modbus(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in bytes {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xA001 } else { crc >> 1 };
        }
    }
    crc
}

pub fn append_crc(mut frame: Vec<u8>) -> Vec<u8> {
    let crc = crc16_modbus(&frame);
    frame.push((crc & 0xFF) as u8);
    frame.push((crc >> 8) as u8);
    frame
}

pub fn validate_response(frame: &[u8]) -> Result<&[u8], String> {
    if frame.len() < 4 { return Err("Modbus response is too short".into()); }
    let split = frame.len() - 2;
    let data = &frame[..split];
    let expected = crc16_modbus(data);
    let received = u16::from_le_bytes([frame[split], frame[split + 1]]);
    if expected != received { return Err("Modbus CRC mismatch".into()); }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_crc_vector() {
        let frame = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert_eq!(crc16_modbus(&frame), 0xCDC5);
        assert!(validate_response(&append_crc(frame.to_vec())).is_ok());
    }

    #[test]
    fn low_battery_enters_conserve_or_critical() {
        assert_eq!(EnergyPolicy::from_battery_percent(20.0).mode, PowerMode::Conserve);
        assert_eq!(EnergyPolicy::from_battery_percent(5.0).mode, PowerMode::Critical);
    }
}
