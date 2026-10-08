//! Bounded encrypted direct transport for CYBChat.
//!
//! LAN discovery remains UDP. Actual direct messages use TCP + Noise XX.
//! The static Noise key is generated once and persisted in cybOS KV storage.
//! Handshake and message work run outside the egui thread and have hard
//! read/write deadlines.

use serde::{Deserialize, Serialize};
use snow::{Builder, HandshakeState, TransportState};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const PORT: u16 = 39394;
const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_FRAME: usize = 16 * 1024;
const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

#[derive(Clone, Debug)]
pub(crate) enum SecureEvent {
    Received {
        message_id: String,
        node_id: String,
        message: String,
        fingerprint: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum SecureSendStatus {
    Delivered { message_id: String, peer_id: String },
    Failed { message_id: String, peer_id: String, reason: String },
}

#[derive(Serialize, Deserialize)]
struct Hello {
    node_id: String,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    message_id: String,
    from: String,
    to: String,
    message: String,
}

pub(crate) fn load_or_create_static_key(store: &crate::store::Store) -> Result<Vec<u8>, String> {
    if let Some(encoded) = store.get("noise_static_private_hex") {
        return decode_hex(&encoded);
    }

    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let builder = Builder::new(params);
    let keypair = builder
        .generate_keypair()
        .map_err(|e| format!("noise key generation: {e}"))?;

    let encoded = encode_hex(&keypair.private);
    store.set("noise_static_private_hex", &encoded);
    Ok(keypair.private)
}

pub(crate) fn public_fingerprint(private_key: &[u8]) -> Result<String, String> {
    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let hs = Builder::new(params)
        .local_private_key(private_key)
        .map_err(|e| format!("noise key: {e}"))?
        .build_initiator()
        .map_err(|e| format!("noise state: {e}"))?;
    let public = hs
        .get_remote_static()
        .unwrap_or(&[]);
    if public.is_empty() {
        // For a XX pattern the local static public key is not exposed by the
        // handshake state. The fingerprint is therefore derived from the
        // private key as a stable display identifier, not used as crypto.
        return Ok(short_hash(private_key));
    }
    Ok(short_hash(public))
}

pub(crate) fn spawn_listener(node_id: String, private_key: Vec<u8>) -> Receiver<SecureEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let listener = match TcpListener::bind(("0.0.0.0", PORT)) {
            Ok(v) => v,
            Err(_) => return,
        };
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(TIMEOUT));
            let _ = stream.set_write_timeout(Some(TIMEOUT));
            let tx = tx.clone();
            let node_id = node_id.clone();
            let key = private_key.clone();
            thread::spawn(move || {
                if let Ok(Some(event)) = receive_one(&mut stream, &node_id, &key) {
                    let _ = tx.send(event);
                }
            });
        }
    });
    rx
}

pub(crate) fn send(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    private_key: &[u8],
    message: &str,
) -> SecureSendStatus {
    let message_id = Uuid::new_v4().to_string();
    let result = (|| -> Result<(), String> {
        if message.trim().is_empty() {
            return Err("empty message".into());
        }
        if message.as_bytes().len() > MAX_FRAME {
            return Err("message too large".into());
        }

        let mut stream = TcpStream::connect_timeout(
            &format!("{address}:{PORT}")
                .parse()
                .map_err(|e| format!("address: {e}"))?,
            TIMEOUT,
        )
        .map_err(|e| format!("connect: {e}"))?;
        stream
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;

        let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
        let mut hs = Builder::new(params)
            .local_private_key(private_key)
            .map_err(|e| format!("noise key: {e}"))?
            .build_initiator()
            .map_err(|e| format!("noise initiator: {e}"))?;

        let mut buf = vec![0_u8; 65535];
        let mut payload = vec![0_u8; 65535];

        let n = hs
            .write_message(&[], &mut buf)
            .map_err(|e| format!("handshake 1: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        let n = read_frame(&mut stream, &mut buf)?;
        hs.read_message(&buf[..n], &mut payload)
            .map_err(|e| format!("handshake 2: {e}"))?;

        let hello = serde_json::to_vec(&Hello {
            node_id: sender_id.into(),
        })
        .map_err(|e| e.to_string())?;
        let n = hs
            .write_message(&hello, &mut buf)
            .map_err(|e| format!("handshake 3: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        let mut transport = hs
            .into_transport_mode()
            .map_err(|e| format!("transport: {e}"))?;

        let envelope = serde_json::to_vec(&Envelope {
            message_id: message_id.clone(),
            from: sender_id.into(),
            to: peer_id.into(),
            message: message.into(),
        })
        .map_err(|e| e.to_string())?;

        let n = transport
            .write_message(&envelope, &mut buf)
            .map_err(|e| format!("encrypt: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        let n = read_frame(&mut stream, &mut buf)?;
        transport
            .read_message(&buf[..n], &mut payload)
            .map_err(|e| format!("ack decrypt: {e}"))?;

        if payload != b"ACK" {
            return Err("invalid secure ACK".into());
        }
        Ok(())
    })();

    match result {
        Ok(()) => SecureSendStatus::Delivered {
            message_id,
            peer_id: peer_id.into(),
        },
        Err(reason) => SecureSendStatus::Failed {
            message_id,
            peer_id: peer_id.into(),
            reason,
        },
    }
}

fn receive_one(
    stream: &mut TcpStream,
    node_id: &str,
    private_key: &[u8],
) -> Result<Option<SecureEvent>, String> {
    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let mut hs = Builder::new(params)
        .local_private_key(private_key)
        .map_err(|e| format!("noise key: {e}"))?
        .build_responder()
        .map_err(|e| format!("noise responder: {e}"))?;

    let mut buf = vec![0_u8; 65535];
    let mut payload = vec![0_u8; 65535];

    let n = read_frame(stream, &mut buf)?;
    hs.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("handshake 1: {e}"))?;

    let n = hs
        .write_message(&[], &mut buf)
        .map_err(|e| format!("handshake 2: {e}"))?;
    write_frame(stream, &buf[..n])?;

    let n = read_frame(stream, &mut buf)?;
    let payload_len = hs
        .read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("handshake 3: {e}"))?;
    let hello: Hello = serde_json::from_slice(&payload[..payload_len])
        .map_err(|e| format!("hello: {e}"))?;

    if hello.node_id == node_id {
        return Ok(None);
    }

    let fingerprint = hs
        .get_remote_static()
        .map(short_hash)
        .unwrap_or_else(|| "UNKNOWN".into());

    let mut transport = hs
        .into_transport_mode()
        .map_err(|e| format!("transport: {e}"))?;

    let n = read_frame(stream, &mut buf)?;
    let payload_len = transport
        .read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("decrypt: {e}"))?;
    let envelope: Envelope = serde_json::from_slice(&payload[..payload_len])
        .map_err(|e| format!("envelope: {e}"))?;

    if envelope.to != node_id || envelope.from != hello.node_id {
        return Err("secure identity mismatch".into());
    }

    let n = transport
        .write_message(b"ACK", &mut buf)
        .map_err(|e| format!("ack encrypt: {e}"))?;
    write_frame(stream, &buf[..n])?;

    Ok(Some(SecureEvent::Received {
        message_id: envelope.message_id,
        node_id: envelope.from,
        message: envelope.message,
        fingerprint,
    }))
}

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<(), String> {
    if data.len() > MAX_FRAME + 4096 {
        return Err("frame too large".into());
    }
    let len = u32::try_from(data.len()).map_err(|_| "frame length overflow".to_string())?;
    stream
        .write_all(&len.to_be_bytes())
        .and_then(|_| stream.write_all(data))
        .map_err(|e| format!("write frame: {e}"))
}

fn read_frame(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<usize, String> {
    let mut len = [0_u8; 4];
    stream
        .read_exact(&mut len)
        .map_err(|e| format!("read length: {e}"))?;
    let size = u32::from_be_bytes(len) as usize;
    if size > buffer.len() {
        return Err("frame exceeds receive buffer".into());
    }
    stream
        .read_exact(&mut buffer[..size])
        .map_err(|e| format!("read frame: {e}"))?;
    Ok(size)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("invalid hex length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| "invalid hex".to_string()))
        .collect()
}

fn short_hash(bytes: &[u8]) -> String {
    // Display-only fingerprint. The Noise handshake itself provides the
    // cryptographic authentication of the static key.
    let mut state = 0xcbf29ce484222325_u64;
    for byte in bytes {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, encode_hex};

    #[test]
    fn hex_roundtrip() {
        let data = [0, 1, 2, 15, 16, 255];
        assert_eq!(decode_hex(&encode_hex(&data)).unwrap(), data);
    }
}
