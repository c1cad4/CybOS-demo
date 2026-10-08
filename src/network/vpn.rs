#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VpnBackend {
    SystemWireGuard,
    WireGuardRsLinux,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnProfile {
    pub interface: String,
    pub endpoint: Option<String>,
    pub address: String,
    pub allowed_ips: Vec<String>,
    pub backend: VpnBackend,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VpnState {
    Disabled,
    Ready,
    Connecting,
    Connected,
    Error,
}

impl VpnProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.interface.trim().is_empty() { return Err("VPN interface is empty".into()); }
        if self.interface.len() > 64 { return Err("VPN interface is too long".into()); }
        if self.address.trim().is_empty() { return Err("VPN address is empty".into()); }
        if self.allowed_ips.is_empty() { return Err("VPN allowed-ips list is empty".into()); }
        if self.allowed_ips.len() > 256 { return Err("VPN allowed-ips list is too large".into()); }
        Ok(())
    }
}
