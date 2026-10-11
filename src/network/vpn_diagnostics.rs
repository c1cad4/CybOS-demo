//! Read-only CLI diagnostics. Never use `wg show dump` (it includes private keys).
use std::{
    io,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Health {
    MissingTools,
    Unavailable,
    NoMatchingInterface,
    NoHandshake(String),
    Handshake {
        interface: String,
        age_seconds: u64,
        recent: bool,
    },
    InvalidClock,
}
impl Health {
    pub fn message(&self) -> String {
        match self {
            Self::MissingTools => "WireGuard CLI not installed. App Store tunnels cannot be inspected by this diagnostic.".into(),
            Self::Unavailable => "Cannot read WireGuard state. Check CLI permissions or use the WireGuard app's diagnostics.".into(),
            Self::NoMatchingInterface => "No CLI tunnel with this device's public key was found.".into(),
            Self::NoHandshake(interface) => format!("{interface}: no handshake with the configured server has been observed."),
            Self::Handshake { interface, age_seconds, recent } => format!("{interface}: {} handshake, {age_seconds}s ago. This does not verify routing or application delivery.", if *recent { "recent" } else { "old" }),
            Self::InvalidClock => "Handshake timestamp is invalid or system clocks disagree.".into(),
        }
    }
}
fn read_public(tool: &str, field: &str) -> Result<String, Health> {
    let mut child = Command::new(tool)
        .args(["show", "all", field])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Health::MissingTools
            } else {
                Health::Unavailable
            }
        })?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(Health::Unavailable);
                }
                let output = child.wait_with_output().map_err(|_| Health::Unavailable)?;
                if output.stdout.len() > 65536 {
                    return Err(Health::Unavailable);
                }
                return String::from_utf8(output.stdout).map_err(|_| Health::Unavailable);
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Health::Unavailable);
            }
        }
    }
}
pub(crate) fn inspect(local_key: &str, server_key: &str) -> Health {
    let keys = match read_public("wg", "public-key") {
        Ok(text) => text,
        Err(health) => return health,
    };
    let handshakes = match read_public("wg", "latest-handshakes") {
        Ok(text) => text,
        Err(health) => return health,
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    parse(&keys, &handshakes, local_key, server_key, now)
}
fn parse(keys: &str, handshakes: &str, local_key: &str, server_key: &str, now: u64) -> Health {
    let interfaces: Vec<_> = keys
        .lines()
        .filter_map(|line| {
            let row: Vec<_> = line.split_whitespace().collect();
            if row.len() != 2
                || row[1] != local_key
                || row[0].len() > 15
                || !row[0]
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            {
                return None;
            }
            Some(row[0])
        })
        .collect();
    if interfaces.is_empty() {
        return Health::NoMatchingInterface;
    }
    let mut newest = None;
    for line in handshakes.lines() {
        let row: Vec<_> = line.split_whitespace().collect();
        if row.len() != 3 || !interfaces.contains(&row[0]) || row[1] != server_key {
            continue;
        }
        let Ok(time) = row[2].parse::<u64>() else {
            return Health::InvalidClock;
        };
        if time > now {
            return Health::InvalidClock;
        }
        if newest.is_none_or(|(_, old)| time > old) {
            newest = Some((row[0], time));
        }
    }
    match newest {
        Some((interface, time)) if time > 0 => Health::Handshake {
            interface: interface.into(),
            age_seconds: now - time,
            recent: now - time <= 180,
        },
        _ => Health::NoHandshake(interfaces[0].into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handshake_requires_matching_device_and_server_and_recent_timestamp() {
        let keys = "wg0 device\nwg1 other-device\n";
        assert_eq!(
            parse(
                keys,
                "wg1 server 990\nwg0 other-server 990\n",
                "device",
                "server",
                1000
            ),
            Health::NoHandshake("wg0".into())
        );
        assert_eq!(
            parse(keys, "wg0 server 990\n", "device", "server", 1000),
            Health::Handshake {
                interface: "wg0".into(),
                age_seconds: 10,
                recent: true
            }
        );
        assert_eq!(
            parse(keys, "wg0 server 1\n", "device", "server", 1000),
            Health::Handshake {
                interface: "wg0".into(),
                age_seconds: 999,
                recent: false
            }
        );
        assert_eq!(
            parse(keys, "wg0 server 0\n", "device", "server", 1000),
            Health::NoHandshake("wg0".into())
        );
    }
    #[test]
    fn missing_interface_bad_times_and_multiple_interfaces_are_handled() {
        assert_eq!(
            parse(
                "wg0 other-device\n",
                "wg0 server 999\n",
                "device",
                "server",
                1000
            ),
            Health::NoMatchingInterface
        );
        assert_eq!(
            parse(
                "wg0 device\n",
                "wg0 server 1001\n",
                "device",
                "server",
                1000
            ),
            Health::InvalidClock
        );
        assert_eq!(
            parse(
                "wg0 device\n",
                "wg0 server corrupt\n",
                "device",
                "server",
                1000
            ),
            Health::InvalidClock
        );
        assert_eq!(
            parse(
                "wg0 device\nwg1 device\n",
                "wg0 server 1\nwg1 server 999\n",
                "device",
                "server",
                1000
            ),
            Health::Handshake {
                interface: "wg1".into(),
                age_seconds: 1,
                recent: true
            }
        );
    }
    #[cfg(unix)]
    #[test]
    fn stalled_tool_is_killed_within_the_diagnostic_budget() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("cybos-wg-stall-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, "#!/bin/sh\nexec sleep 10\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let start = Instant::now();
        assert_eq!(
            read_public(path.to_str().unwrap(), "public-key"),
            Err(Health::Unavailable)
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn missing_tool_is_an_explicit_diagnostic() {
        assert_eq!(
            read_public("/cybos-nonexistent/wg", "public-key"),
            Err(Health::MissingTools)
        );
    }
}
