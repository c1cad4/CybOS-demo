#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialEndpoint {
    pub name: String,
    pub description: Option<String>,
}

#[cfg(feature = "hardware-serial")]
pub fn list_ports() -> Vec<SerialEndpoint> {
    serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| SerialEndpoint {
            name: port.port_name,
            description: Some(format!("{:?}", port.port_type)),
        })
        .collect()
}

#[cfg(not(feature = "hardware-serial"))]
pub fn list_ports() -> Vec<SerialEndpoint> {
    Vec::new()
}
