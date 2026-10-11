//! WireGuard profile preparation. Export never activates a tunnel.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use snow::resolvers::CryptoResolver;
use std::{
    fs,
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub endpoint: String,
    pub server_key: String,
    pub address: String,
    pub network: String,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            server_key: String::new(),
            address: "10.77.0.2/32".into(),
            network: "10.77.0.0/24".into(),
        }
    }
}
pub(crate) struct Setup {
    pub profile: Profile,
    pub public_key: Option<String>,
    pub status: String,
    pub export_path: Option<PathBuf>,
    pub invitation_input: String,
    pub server_verified: bool,
    pub diagnostic_status: String,
    pub diagnostic_task: Option<std::sync::mpsc::Receiver<super::vpn_diagnostics::Health>>,
}
impl Setup {
    pub fn load(store: &crate::store::Store) -> Self {
        let (profile, status) = match store.get_checked("wireguard_profile") {
            Ok(Some(raw)) => match serde_json::from_str(&raw) {
                Ok(p) => (p, "Profile saved. Connection has not been checked.".into()),
                Err(_) => (
                    Profile::default(),
                    "Saved VPN settings could not be read.".into(),
                ),
            },
            Ok(None) => (
                Profile::default(),
                "Setup required: add a server before exporting a profile.".into(),
            ),
            Err(error) => (Profile::default(), error),
        };
        let mut setup = Self {
            profile,
            public_key: None,
            status,
            export_path: None,
            invitation_input: String::new(),
            server_verified: false,
            diagnostic_status: "Not checked. CLI diagnostics do not monitor App Store tunnels."
                .into(),
            diagnostic_task: None,
        };
        let path = identity_path(store);
        if fs::symlink_metadata(&path).is_ok() {
            match read_identity(&path).and_then(|key| public_key(&key)) {
                Ok(key) => setup.public_key = Some(key),
                Err(error) => setup.status = error,
            }
        }
        setup
    }
    pub fn start_diagnostics(&mut self) {
        if self.diagnostic_task.is_some() {
            return;
        }
        let Some(local_key) = self.public_key.clone() else {
            self.diagnostic_status = "Create or load this device's identity first.".into();
            return;
        };
        if decode_key(self.profile.server_key.trim()).is_err() {
            self.diagnostic_status = "Enter a valid server public key first.".into();
            return;
        }
        let server_key = self.profile.server_key.trim().to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        self.diagnostic_task = Some(rx);
        self.diagnostic_status = "Reading public WireGuard state…".into();
        std::thread::spawn(move || {
            let _ = tx.send(super::vpn_diagnostics::inspect(&local_key, &server_key));
        });
    }
    pub fn poll_diagnostics(&mut self) {
        let result = self.diagnostic_task.as_ref().map(|rx| rx.try_recv());
        match result {
            Some(Ok(health)) => {
                self.diagnostic_status = health.message();
                self.diagnostic_task = None;
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.diagnostic_status = "Diagnostic worker stopped.".into();
                self.diagnostic_task = None;
            }
            _ => {}
        }
    }
    pub fn settings_changed(&mut self) {
        self.server_verified = false;
        self.export_path = None;
        self.diagnostic_task = None;
        self.diagnostic_status = "Settings changed; refresh diagnostics.".into();
        self.status = "Settings changed. Verify the server key and export a new profile.".into();
    }
    pub fn import_invitation(&mut self) {
        match Profile::from_invitation(&self.invitation_input) {
            Ok(profile) => {
                self.profile = profile;
                self.settings_changed();
                self.export_path = None;
                self.invitation_input.clear();
                self.status =
                    "Public invitation loaded. Verify the server key before exporting.".into();
            }
            Err(error) => self.status = error,
        }
    }
    pub fn create_identity(&mut self, store: &crate::store::Store) {
        match ensure_identity(&identity_path(store)).and_then(|key| public_key(&key)) {
            Ok(key) => {
                self.public_key = Some(key);
                self.status = "Device identity ready. Share only the public key with your server administrator.".into();
            }
            Err(error) => self.status = error,
        }
    }
    pub fn export(&mut self, store: &crate::store::Store) {
        self.export_mode(store, None);
    }
    pub fn export_internet(&mut self, store: &crate::store::Store, dns: &str) {
        self.export_mode(store, Some(dns));
    }
    fn export_mode(&mut self, store: &crate::store::Store, dns: Option<&str>) {
        self.export_path = None;
        let result = (|| {
            if !self.server_verified {
                return Err("Verify the server public key before exporting.".into());
            }
            self.profile.validate()?;
            let key = ensure_identity(&identity_path(store))?;
            self.public_key = Some(public_key(&key)?);
            let config = match dns {
                Some(dns) => self.profile.render_internet(&key, dns)?,
                None => self.profile.render(&key)?,
            };
            store.set_checked(
                "wireguard_profile",
                &serde_json::to_string(&self.profile)
                    .map_err(|_| "Could not save VPN settings.")?,
            )?;
            let path = vpn_dir(store).join(format!(
                "cybos-{}.conf",
                &uuid::Uuid::new_v4().simple().to_string()[..8]
            ));
            write_secret_new(&path, config.as_bytes())?;
            Ok::<_, String>(path)
        })();
        match result {
            Ok(path) => {
                self.export_path = Some(path);
                self.status = "Profile exported. Import into WireGuard and activate there. Connection has not been verified.".into();
            }
            Err(error) => self.status = error,
        }
    }
}
fn vpn_dir(store: &crate::store::Store) -> PathBuf {
    store
        .path
        .parent()
        .unwrap_or(Path::new("."))
        .join("wireguard")
}
fn identity_path(store: &crate::store::Store) -> PathBuf {
    vpn_dir(store).join("device.key")
}
fn decode_key(encoded: &str) -> Result<[u8; 32], String> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "WireGuard key must be canonical base64.")?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "WireGuard keys must contain 32 bytes.")?;
    if key == [0; 32] || STANDARD.encode(key) != encoded {
        return Err("Invalid WireGuard key.".into());
    }
    Ok(key)
}
pub(crate) fn public_key(private: &[u8; 32]) -> Result<String, String> {
    let resolver = snow::resolvers::DefaultResolver::default();
    let mut dh = resolver
        .resolve_dh(&snow::params::DHChoice::Curve25519)
        .ok_or("Curve25519 unavailable.")?;
    dh.set(private);
    Ok(STANDARD.encode(dh.pubkey()))
}
fn private_cidr(text: &str) -> Result<(Ipv4Addr, u8), String> {
    let (ip, bits) = text
        .split_once('/')
        .ok_or("Use an IPv4 CIDR, for example 10.77.0.0/24.")?;
    let ip: Ipv4Addr = ip.parse().map_err(|_| "Invalid IPv4 address.")?;
    let bits: u8 = bits.parse().map_err(|_| "Invalid network prefix.")?;
    if !(8..=32).contains(&bits) {
        return Err("Use a private IPv4 subnet, not a default route.".into());
    }
    let mask = u32::MAX << (32 - bits);
    let first = Ipv4Addr::from(u32::from(ip) & mask);
    let last = Ipv4Addr::from(u32::from(first) | !mask);
    if !first.is_private() || !last.is_private() {
        return Err("Use a subnet within 10/8, 172.16/12 or 192.168/16.".into());
    }
    Ok((ip, bits))
}
impl Profile {
    fn from_invitation(raw: &str) -> Result<Self, String> {
        if raw.len() > 4096 {
            return Err("Invitation is too large.".into());
        }
        let mut value: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| "Invalid public invitation JSON.")?;
        let object = value
            .as_object_mut()
            .ok_or("Invitation must be an object.")?;
        if object.remove("version") != Some(serde_json::json!(1)) {
            return Err("Unsupported invitation version.".into());
        }
        let profile: Self = serde_json::from_value(value)
            .map_err(|_| "Invitation must contain public connection fields only.")?;
        profile.validate()?;
        Ok(profile)
    }
    pub fn validate(&self) -> Result<(), String> {
        let endpoint: SocketAddr = self
            .endpoint
            .trim()
            .parse()
            .map_err(|_| "Enter server IP and UDP port, for example 203.0.113.10:51820.")?;
        if endpoint.port() == 0
            || endpoint.ip().is_unspecified()
            || endpoint.ip().is_multicast()
            || endpoint.ip().to_string() == "255.255.255.255"
        {
            return Err("Invalid server endpoint.".into());
        }
        decode_key(self.server_key.trim())?;
        let (address, host_bits) = private_cidr(self.address.trim())?;
        let (network, bits) = private_cidr(self.network.trim())?;
        let mask = u32::MAX << (32 - bits);
        if host_bits != 32 {
            return Err("Device address must use /32.".into());
        }
        if u32::from(network) & !mask != 0 {
            return Err("Use the subnet's network address in AllowedIPs.".into());
        }
        if u32::from(address) & mask != u32::from(network) {
            return Err("Device address must belong to the selected subnet.".into());
        }
        if bits < 31 && (address == network || u32::from(address) == (u32::from(network) | !mask)) {
            return Err("Device address cannot be the subnet or broadcast address.".into());
        }
        Ok(())
    }
    pub fn render(&self, private: &[u8; 32]) -> Result<String, String> {
        self.validate()?;
        if public_key(private)? == self.server_key.trim() {
            return Err("Server and device must have different keys.".into());
        }
        Ok(format!(
            "[Interface]\nPrivateKey = {}\nAddress = {}\n\n[Peer]\nPublicKey = {}\nEndpoint = {}\nAllowedIPs = {}\nPersistentKeepalive = 25\n",
            STANDARD.encode(private),
            self.address.trim(),
            self.server_key.trim(),
            self.endpoint.trim(),
            self.network.trim()
        ))
    }
    /// Full-tunnel preparation, never activation or a claim that a kill switch is installed.
    pub fn render_internet(&self, private: &[u8; 32], dns: &str) -> Result<String, String> {
        let config = self.render(private)?;
        let dns: std::net::Ipv4Addr = dns.trim().parse().map_err(|_| "DNS must be one IPv4 address.")?;
        if dns.is_unspecified() || dns.is_loopback() || dns.is_multicast() || dns == Ipv4Addr::BROADCAST || dns.is_link_local() {
            return Err("Choose a reachable DNS server, not a local or multicast address.".into());
        }
        // IPv6 also routes into the tunnel. A server without IPv6 forwarding will drop it,
        // instead of leaving an IPv6 default route outside this exported profile.
        Ok(config.replace("\n\n[Peer]", &format!("\nDNS = {dns}\n\n[Peer]"))
            .replace(&format!("AllowedIPs = {}", self.network.trim()), "AllowedIPs = 0.0.0.0/0, ::/0"))
    }
}

#[cfg(test)]
mod internet_tests {
    use super::*;
    #[test]
    fn full_tunnel_contains_dns_and_both_routes_without_changing_private_profile() {
        let profile=Profile {endpoint:"203.0.113.10:51820".into(),server_key:public_key(&[7;32]).unwrap(),..Default::default()};
        let text=profile.render_internet(&[8;32],"9.9.9.9").unwrap();
        assert!(text.contains("AllowedIPs = 0.0.0.0/0, ::/0")); assert!(text.contains("DNS = 9.9.9.9"));
        assert!(profile.render(&[8;32]).unwrap().contains("AllowedIPs = 10.77.0.0/24"));
        for dns in ["127.0.0.1","0.0.0.0","224.0.0.1","9.9.9.9\nPostUp = evil","255.255.255.255"] {
            assert!(profile.render_internet(&[8;32],dns).is_err());
        }
    }
}
#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("Cannot create private WireGuard directory.".into()),
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "Cannot inspect WireGuard directory.")?;
    if !meta.is_dir() || meta.permissions().mode() & 0o077 != 0 {
        return Err("WireGuard directory must be a real directory with mode 0700.".into());
    }
    Ok(())
}
#[cfg(unix)]
fn write_secret_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    secure_directory(path.parent().ok_or("Missing private directory.")?)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "Private file already exists."
            } else {
                "Cannot create private file."
            }
        })?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot persist private file. Inspect it before retrying.".into())
}
#[cfg(unix)]
fn read_identity(path: &Path) -> Result<[u8; 32], String> {
    use std::os::unix::fs::PermissionsExt;
    secure_directory(path.parent().ok_or("Missing private directory.")?)?;
    let meta = fs::symlink_metadata(path).map_err(|_| "Cannot inspect device key.")?;
    if !meta.is_file() || meta.permissions().mode() & 0o077 != 0 || meta.len() > 64 {
        return Err("Device key must be a private regular file (mode 0600).".into());
    }
    let encoded = fs::read_to_string(path).map_err(|_| "Cannot read device key.")?;
    decode_key(encoded.trim())
}
#[cfg(not(unix))]
fn write_secret_new(_: &Path, _: &[u8]) -> Result<(), String> {
    Err("Private-key storage is currently supported on macOS and Linux only.".into())
}
#[cfg(not(unix))]
fn read_identity(_: &Path) -> Result<[u8; 32], String> {
    Err("Private-key storage is currently supported on macOS and Linux only.".into())
}
fn ensure_identity(path: &Path) -> Result<[u8; 32], String> {
    if fs::symlink_metadata(path).is_ok() {
        return read_identity(path);
    }
    let pair = snow::Builder::new(
        "Noise_XX_25519_ChaChaPoly_BLAKE2s"
            .parse()
            .map_err(|_| "Invalid key generation parameters.")?,
    )
    .generate_keypair()
    .map_err(|_| "Could not generate WireGuard key.")?;
    let mut private: [u8; 32] = pair
        .private
        .try_into()
        .map_err(|_| "Invalid generated key length.")?;
    private[0] &= 248;
    private[31] = (private[31] & 127) | 64;
    match write_secret_new(path, STANDARD.encode(private).as_bytes()) {
        Ok(()) => read_identity(path),
        Err(error) if error == "Private file already exists." => read_identity(path),
        Err(error) => Err(error),
    }
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn invitations_accept_only_versioned_public_valid_settings() {
        let mut invite = serde_json::to_value(profile()).unwrap();
        invite["version"] = 1.into();
        assert!(Profile::from_invitation(&invite.to_string()).is_ok());
        invite["private_key"] = "secret".into();
        assert!(Profile::from_invitation(&invite.to_string()).is_err());
        invite.as_object_mut().unwrap().remove("private_key");
        invite["version"] = 2.into();
        assert!(Profile::from_invitation(&invite.to_string()).is_err());
        assert!(Profile::from_invitation(&"x".repeat(4097)).is_err());
    }
    #[test]
    #[ignore = "requires an explicitly supplied official wg executable"]
    fn official_wg_derives_the_same_public_key() {
        use std::process::{Command, Stdio};
        let tool = std::env::var("CYBOS_WG").expect("Set CYBOS_WG to official wg executable");
        let private = [9; 32];
        let mut child = Command::new(tool)
            .arg("pubkey")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(STANDARD.encode(private).as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            public_key(&private).unwrap()
        );
    }
    fn profile() -> Profile {
        Profile {
            endpoint: "203.0.113.10:51820".into(),
            server_key: STANDARD.encode([7; 32]),
            ..Profile::default()
        }
    }
    #[test]
    fn profile_rejects_injection_default_routes_and_wrong_subnet() {
        let p = profile();
        assert!(p.validate().is_ok());
        for address in [
            "10.77.0.0/32",
            "10.77.0.255/32",
            "10.78.0.2/32",
            "10.77.0.2/24",
        ] {
            let mut bad = p.clone();
            bad.address = address.into();
            assert!(bad.validate().is_err());
        }
        for route in [
            "0.0.0.0/0",
            "192.168.0.0/8",
            "10.77.0.2/24",
            "10.77.0.0/24\nPostUp=evil",
        ] {
            let mut bad = p.clone();
            bad.network = route.into();
            assert!(bad.validate().is_err());
        }
        for endpoint in [
            "hostname:51820",
            "0.0.0.0:51820",
            "203.0.113.10:0",
            "203.0.113.10:51820\nPostUp=evil",
        ] {
            let mut bad = p.clone();
            bad.endpoint = endpoint.into();
            assert!(bad.validate().is_err());
        }
        let mut bad = p;
        bad.server_key = STANDARD.encode([0; 32]);
        assert!(bad.validate().is_err());
    }
    #[test]
    fn config_routes_device_subnet_without_hooks_or_dns() {
        let config = profile().render(&[9; 32]).unwrap();
        assert!(config.contains("AllowedIPs = 10.77.0.0/24\n"));
        assert!(config.contains("PersistentKeepalive = 25\n"));
        assert!(!config.contains("DNS"));
        assert!(!config.contains("PostUp"));
        let mut same = profile();
        same.server_key = public_key(&[9; 32]).unwrap();
        assert!(same.render(&[9; 32]).is_err());
    }
    #[test]
    fn export_requires_verification_and_keeps_private_keys_out_of_database() {
        let root = std::env::temp_dir().join(format!("cybos-wg-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        {
            let store = crate::store::Store::test_store(root.join("cybos.db").to_str().unwrap());
            let mut setup = Setup::load(&store);
            setup.profile = profile();
            setup.export(&store);
            assert!(setup.export_path.is_none());
            assert!(!identity_path(&store).exists());
            setup.server_verified = true;
            setup.export(&store);
            let path = setup.export_path.as_ref().expect("exported profile");
            let private = STANDARD.encode(read_identity(&identity_path(&store)).unwrap());
            let metadata = store.get_checked("wireguard_profile").unwrap().unwrap();
            assert!(!metadata.contains(&private));
            assert!(!metadata.contains("PrivateKey"));
            assert!(fs::read_to_string(path).unwrap().contains(&private));
            let loaded = Setup::load(&store);
            assert_eq!(loaded.public_key, setup.public_key);
            assert!(!loaded.server_verified);
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn identity_is_stable_private_and_corruption_never_rotates_it() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("cybos-wg-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("private/device.key");
        let first = ensure_identity(&path).unwrap();
        assert_eq!(ensure_identity(&path).unwrap(), first);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&path, "corrupt").unwrap();
        assert!(ensure_identity(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "corrupt");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn public_permissions_symlinks_and_overwrites_are_rejected() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = std::env::temp_dir().join(format!("cybos-wg-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("private/device.key");
        ensure_identity(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_identity(&path).is_err());
        let link = root.join("private/linked.key");
        symlink(&path, &link).unwrap();
        assert!(read_identity(&link).is_err());
        assert!(write_secret_new(&path, b"replacement").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
