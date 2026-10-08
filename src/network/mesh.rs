#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshPacket {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub payload: Vec<u8>,
    pub ttl: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshRoute {
    pub hops: Vec<String>,
    pub remaining_ttl: u8,
}

impl MeshPacket {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("mesh packet id is empty".into());
        }
        if self.source.trim().is_empty() || self.destination.trim().is_empty() {
            return Err("mesh source/destination is empty".into());
        }
        if self.payload.len() > 4096 {
            return Err("mesh payload exceeds 4 KiB".into());
        }
        if self.ttl == 0 {
            return Err("mesh TTL expired".into());
        }
        Ok(())
    }

    pub fn forwarded(&self) -> Option<Self> {
        if self.ttl <= 1 { return None; }
        Some(Self { ttl: self.ttl - 1, ..self.clone() })
    }
}

#[cfg(test)]
mod tests {
    use super::MeshPacket;

    #[test]
    fn forwarding_decrements_ttl() {
        let packet = MeshPacket {
            id: "m1".into(),
            source: "a".into(),
            destination: "b".into(),
            payload: b"hello".to_vec(),
            ttl: 4,
        };
        assert_eq!(packet.forwarded().unwrap().ttl, 3);
    }
}
