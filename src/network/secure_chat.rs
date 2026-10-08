//! Bounded encrypted direct transport for CYBChat.
use serde::{Deserialize, Serialize};
use snow::Builder;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc::{self, Receiver, Sender}, Arc};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PORT: u16 = 39394;
const TIMEOUT: Duration = Duration::from_secs(8);
const SESSION_BUDGET: Duration = Duration::from_secs(12);
const MAX_FRAME: usize = 16 * 1024;
const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

#[derive(Clone, Debug)]
pub(crate) enum SecureReply {
    Ack,
    Reject(String),
}

#[derive(Clone, Debug)]
pub(crate) enum SecureEvent {
    Received {
        message_id: String,
        node_id: String,
        message: String,
        fingerprint: String,
        public_key: Vec<u8>,
        reply: Sender<SecureReply>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum SecureSendStatus {
    Delivered { message_id: String, peer_id: String },
    Failed { message_id: String, peer_id: String, reason: String },
}

#[derive(Serialize, Deserialize)]
struct Hello { node_id: String }

#[derive(Serialize, Deserialize)]
struct Envelope {
    message_id: String,
    from: String,
    to: String,
    message: String,
}

pub(crate) struct Listener {
    pub(crate) events: Receiver<SecureEvent>,
    stop: Arc<AtomicBool>,
}

impl Listener {
    pub(crate) fn try_recv(&self) -> Result<SecureEvent, mpsc::TryRecvError> {
        self.events.try_recv()
    }

    pub(crate) fn empty() -> Self {
        let (_tx, events) = mpsc::channel();
        Self {
            events,
            stop: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub(crate) fn load_or_create_static_key(store: &crate::store::Store) -> Result<Vec<u8>, String> {
    if let Some(encoded) = store.get("noise_static_private_hex") {
        if encoded.len() == 64 && encoded.chars().all(|c| c.is_ascii_hexdigit()) {
            let mut key = Vec::with_capacity(32);
            for i in (0..encoded.len()).step_by(2) {
                key.push(u8::from_str_radix(&encoded[i..i + 2], 16).map_err(|e| e.to_string())?);
            }
            return Ok(key);
        }
    }

    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let keypair = Builder::new(params)
        .generate_keypair()
        .map_err(|e| format!("noise key generation: {e}"))?;
    let encoded = keypair.private.iter().map(|b| format!("{b:02x}")).collect::<String>();
    store.set("noise_static_private_hex", &encoded);
    Ok(keypair.private)
}

pub(crate) fn spawn_listener(node_id: String, private_key: Vec<u8>) -> Listener {
    spawn_listener_bind(node_id, private_key, "0.0.0.0", PORT)
}

#[cfg(test)]
fn spawn_listener_at(node_id: String, private_key: Vec<u8>, port: u16) -> Listener {
    spawn_listener_bind(node_id, private_key, "127.0.0.1", port)
}

fn spawn_listener_bind(
    node_id: String,
    private_key: Vec<u8>,
    host: &'static str,
    port: u16,
) -> Listener {
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);

    thread::spawn(move || {
        let listener = match TcpListener::bind((host, port)) {
            Ok(v) => v,
            Err(_) => return,
        };
        if listener.set_nonblocking(true).is_err() {
            return;
        }

        const MAX_ACTIVE: usize = 8;
        let active = Arc::new(AtomicUsize::new(0));

        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    if active.load(Ordering::Acquire) >= MAX_ACTIVE {
                        continue;
                    }
                    let _ = stream.set_read_timeout(Some(TIMEOUT));
                    let _ = stream.set_write_timeout(Some(TIMEOUT));
                    active.fetch_add(1, Ordering::AcqRel);
                    let tx = tx.clone();
                    let node_id = node_id.clone();
                    let key = private_key.clone();
                    let active = Arc::clone(&active);
                    thread::spawn(move || {
                        let deadline = Instant::now() + SESSION_BUDGET;
                        let _ = receive_one(&mut stream, &node_id, &key, deadline, &tx);
                        active.fetch_sub(1, Ordering::AcqRel);
                    });
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(_) => thread::sleep(Duration::from_millis(50)),
            }
        }
    });
    Listener { events: rx, stop }
}

pub(crate) fn send(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    private_key: &[u8],
    message: &str,
) -> SecureSendStatus {
    send_on_port(sender_id, peer_id, address, PORT, private_key, message)
}

fn send_on_port(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    port: u16,
    private_key: &[u8],
    message: &str,
) -> SecureSendStatus {
    let message_id = Uuid::new_v4().to_string();
    send_with_deadline(sender_id, peer_id, address, port, private_key, message, Instant::now() + SESSION_BUDGET, message_id)
}

fn send_with_deadline(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    port: u16,
    private_key: &[u8],
    message: &str,
    deadline: Instant,
    message_id: String,
) -> SecureSendStatus {
    let result = (|| -> Result<(), String> {
        if message.trim().is_empty() {
            return Err("empty message".into());
        }
        if message.as_bytes().len() > MAX_FRAME {
            return Err("message too large".into());
        }

        let mut stream = TcpStream::connect_timeout(
            &format!("{address}:{port}")
                .parse()
                .map_err(|e| format!("address: {e}"))?,
            remaining(deadline)?,
        )
        .map_err(|e| format!("connect: {e}"))?;
        apply_io_timeout(&mut stream, deadline)?;

        let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
        let mut hs = Builder::new(params)
            .local_private_key(private_key)
            .map_err(|e| format!("noise key: {e}"))?
            .build_initiator()
            .map_err(|e| format!("noise initiator: {e}"))?;

        let mut buf = vec![0_u8; 65535];
        let mut payload = vec![0_u8; 65535];

        apply_io_timeout(&mut stream, deadline)?;
        let n = hs.write_message(&[], &mut buf)
            .map_err(|e| format!("handshake 1: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        apply_io_timeout(&mut stream, deadline)?;
        let n = read_frame(&mut stream, &mut buf)?;
        hs.read_message(&buf[..n], &mut payload)
            .map_err(|e| format!("handshake 2: {e}"))?;

        let hello = serde_json::to_vec(&Hello { node_id: sender_id.into() })
            .map_err(|e| e.to_string())?;
        apply_io_timeout(&mut stream, deadline)?;
        let n = hs.write_message(&hello, &mut buf)
            .map_err(|e| format!("handshake 3: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        let mut transport = hs.into_transport_mode()
            .map_err(|e| format!("transport: {e}"))?;

        let envelope = serde_json::to_vec(&Envelope {
            message_id: message_id.clone(),
            from: sender_id.into(),
            to: peer_id.into(),
            message: message.into(),
        }).map_err(|e| e.to_string())?;

        apply_io_timeout(&mut stream, deadline)?;
        let n = transport.write_message(&envelope, &mut buf)
            .map_err(|e| format!("encrypt: {e}"))?;
        write_frame(&mut stream, &buf[..n])?;

        apply_io_timeout(&mut stream, deadline)?;
        let n = read_frame(&mut stream, &mut buf)?;
        let payload_len = transport.read_message(&buf[..n], &mut payload)
            .map_err(|e| format!("ack decrypt: {e}"))?;

        if payload[..payload_len] == *b"ACK" {
            return Ok(());
        }

        let reason = std::str::from_utf8(&payload[..payload_len])
            .ok()
            .and_then(|text| text.strip_prefix("REJECT:"))
            .unwrap_or("secure receiver rejected message");
        Err(reason.to_string())
    })();

    match result {
        Ok(()) => SecureSendStatus::Delivered { message_id, peer_id: peer_id.into() },
        Err(reason) => SecureSendStatus::Failed { message_id, peer_id: peer_id.into(), reason },
    }
}

fn receive_one(
    stream: &mut TcpStream,
    node_id: &str,
    private_key: &[u8],
    deadline: Instant,
    events: &Sender<SecureEvent>,
) -> Result<(), String> {
    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let mut hs = Builder::new(params)
        .local_private_key(private_key)
        .map_err(|e| format!("noise key: {e}"))?
        .build_responder()
        .map_err(|e| format!("noise responder: {e}"))?;

    let mut buf = vec![0_u8; 65535];
    let mut payload = vec![0_u8; 65535];

    apply_io_timeout(stream, deadline)?;
    let n = read_frame(stream, &mut buf)?;
    hs.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("handshake 1: {e}"))?;

    apply_io_timeout(stream, deadline)?;
    let n = hs.write_message(&[], &mut buf)
        .map_err(|e| format!("handshake 2: {e}"))?;
    write_frame(stream, &buf[..n])?;

    apply_io_timeout(stream, deadline)?;
    let n = read_frame(stream, &mut buf)?;
    let payload_len = hs.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("handshake 3: {e}"))?;
    let hello: Hello = serde_json::from_slice(&payload[..payload_len])
        .map_err(|e| format!("hello: {e}"))?;

    if hello.node_id == node_id {
        return Ok(());
    }

    let public_key = hs
        .get_remote_static()
        .ok_or_else(|| "remote static key missing after XX handshake".to_string())?
        .to_vec();
    let fingerprint = short_hash(&public_key);

    let mut transport = hs.into_transport_mode()
        .map_err(|e| format!("transport: {e}"))?;

    apply_io_timeout(stream, deadline)?;
    let n = read_frame(stream, &mut buf)?;
    let payload_len = transport.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("decrypt: {e}"))?;
    let envelope: Envelope = serde_json::from_slice(&payload[..payload_len])
        .map_err(|e| format!("envelope: {e}"))?;

    if envelope.to != node_id || envelope.from != hello.node_id {
        return Err("secure identity mismatch".into());
    }

    let (reply_tx, reply_rx) = mpsc::channel::<SecureReply>();
    events
        .send(SecureEvent::Received {
            message_id: envelope.message_id,
            node_id: envelope.from,
            message: envelope.message,
            fingerprint,
            public_key,
            reply: reply_tx,
        })
        .map_err(|_| "secure event receiver stopped".to_string())?;

    let reply = reply_rx
        .recv_timeout(remaining(deadline)?)
        .map_err(|_| "secure receiver decision timed out".to_string())?;

    let response = match reply {
        SecureReply::Ack => b"ACK".to_vec(),
        SecureReply::Reject(reason) => {
            let safe: String = reason.replace(['\r', '\n'], " ").chars().take(512).collect();
            format!("REJECT:{}", safe).into_bytes()
        }
    };

    apply_io_timeout(stream, deadline)?;
    let n = transport.write_message(&response, &mut buf)
        .map_err(|e| format!("ack encrypt: {e}"))?;
    write_frame(stream, &buf[..n])?;

    Ok(())
}

fn remaining(deadline: Instant) -> Result<Duration, String> {
    let value = deadline.saturating_duration_since(Instant::now());
    if value.is_zero() {
        Err("secure session deadline expired".into())
    } else {
        Ok(value)
    }
}

fn apply_io_timeout(stream: &TcpStream, deadline: Instant) -> Result<(), String> {
    let timeout = remaining(deadline)?;
    stream.set_read_timeout(Some(timeout)).map_err(|e| format!("read timeout: {e}"))?;
    stream.set_write_timeout(Some(timeout)).map_err(|e| format!("write timeout: {e}"))?;
    Ok(())
}

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<(), String> {
    if data.len() > MAX_FRAME + 4096 {
        return Err("frame too large".into());
    }
    let len = u32::try_from(data.len()).map_err(|_| "frame length overflow".to_string())?;
    stream.write_all(&len.to_be_bytes())
        .and_then(|_| stream.write_all(data))
        .map_err(|e| format!("write frame: {e}"))
}

fn read_frame(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<usize, String> {
    let mut len = [0_u8; 4];
    stream.read_exact(&mut len).map_err(|e| format!("read length: {e}"))?;
    let size = u32::from_be_bytes(len) as usize;
    if size > buffer.len() || size > MAX_FRAME + 4096 {
        return Err("frame exceeds receive limit".into());
    }
    stream.read_exact(&mut buffer[..size]).map_err(|e| format!("read frame: {e}"))?;
    Ok(size)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("invalid hex length".into());
    }
    (0..value.len()).step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| "invalid hex".to_string()))
        .collect()
}

fn short_hash(bytes: &[u8]) -> String {
    let mut state = 0xcbf29ce484222325_u64;
    for byte in bytes {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, encode_hex, short_hash, spawn_listener_at, send_on_port, SecureSendStatus, SecureEvent, SecureReply, PATTERN};
    use snow::{params::NoiseParams, Builder};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn hex_roundtrip() {
        let data = [0, 1, 2, 15, 16, 255];
        assert_eq!(decode_hex(&encode_hex(&data)).unwrap(), data);
    }

    #[test]
    fn fingerprint_is_stable_for_public_key() {
        let key = [7_u8; 32];
        assert_eq!(short_hash(&key), short_hash(&key));
    }

    #[test]
    fn secure_loopback_rejection_is_not_reported_as_delivery() {
        let params: NoiseParams = PATTERN.parse().expect("noise params");
        let sender = Builder::new(params.clone()).generate_keypair().expect("sender keypair");
        let receiver = Builder::new(params).generate_keypair().expect("receiver keypair");

        let port = 39401;
        let listener = spawn_listener_at("node-b-reject".to_string(), receiver.private.clone(), port);
        thread::sleep(Duration::from_millis(100));

        let sender_key = sender.private.clone();
        let sender_thread = thread::spawn(move || {
            send_on_port(
                "node-a",
                "node-b-reject",
                "127.0.0.1",
                port,
                &sender_key,
                "rejection test",
            )
        });

        let deadline = Instant::now() + Duration::from_secs(4);
        let message_id = loop {
            match listener.try_recv() {
                Ok(SecureEvent::Received { message_id, reply, .. }) => {
                    reply
                        .send(SecureReply::Reject("receiver rejected test".into()))
                        .expect("send rejection decision");
                    break message_id;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("secure listener did not receive message: {error:?}"),
                _ => panic!("secure listener timed out"),
            }
        };

        let status = sender_thread.join().expect("sender thread join");
        match status {
            SecureSendStatus::Failed { message_id: failed_id, peer_id, reason } => {
                assert_eq!(failed_id, message_id);
                assert_eq!(peer_id, "node-b-reject");
                assert!(reason.contains("receiver rejected"));
            }
            other => panic!("expected rejected delivery, got {other:?}"),
        }

        drop(listener);
    }

    #[test]
    fn secure_loopback_delivery_uses_noise_and_ack() {
        let params: NoiseParams = PATTERN.parse().expect("noise params");
        let sender = Builder::new(params.clone()).generate_keypair().expect("sender keypair");
        let receiver = Builder::new(params).generate_keypair().expect("receiver keypair");

        let port = 39402;
        let listener = spawn_listener_at("node-b".to_string(), receiver.private.clone(), port);
        thread::sleep(Duration::from_millis(100));

        let status = send_on_port("node-a", "node-b", "127.0.0.1", port, &sender.private, "loopback secure test");

        let message_id = match status {
            SecureSendStatus::Delivered { message_id, peer_id } => {
                assert_eq!(peer_id, "node-b");
                message_id
            }
            other => panic!("expected delivered secure message, got {other:?}"),
        };

        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match listener.try_recv() {
                Ok(SecureEvent::Received { message_id: received_id, node_id, message, reply, .. }) => {
                    assert_eq!(received_id, message_id);
                    assert_eq!(node_id, "node-a");
                    assert_eq!(message, "loopback secure test");
                    reply.send(SecureReply::Ack).expect("send ACK decision");
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("secure listener did not receive message: {error:?}"),
                _ => panic!("secure listener timed out"),
            }
        }

        drop(listener);
    }
}
