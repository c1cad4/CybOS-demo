//! Bounded encrypted direct transport for CYBChat.
use serde::{Deserialize, Serialize};
use snow::Builder;
use sha2::{Digest, Sha256};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc::{self, Receiver, Sender, SyncSender}, Arc};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PORT: u16 = 39394;
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
    NeedsTrust { peer_id: String, public_key: Vec<u8> },
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
    if let Some(encoded) = store.get_checked("noise_static_private_hex")? {
        return decode_key(&encoded).map_err(|_| "stored Noise identity is malformed; refusing to replace it".into());
    }

    let params = PATTERN.parse().map_err(|e| format!("noise params: {e}"))?;
    let keypair = Builder::new(params)
        .generate_keypair()
        .map_err(|e| format!("noise key generation: {e}"))?;
    let encoded = keypair.private.iter().map(|b| format!("{b:02x}")).collect::<String>();
    decode_key(&store.persist_noise_identity(&encoded)?)
}

pub(crate) fn spawn_listener(node_id: String, private_key: Vec<u8>) -> Result<Listener, String> {
    spawn_listener_bind(node_id, private_key, "0.0.0.0", PORT)
}

fn spawn_listener_bind(
    node_id: String,
    private_key: Vec<u8>,
    host: &'static str,
    port: u16,
) -> Result<Listener, String> {
    // Keep binding and socket creation separate so tests can reserve ephemeral ports.
    let socket = TcpListener::bind((host, port));
    spawn_listener_socket(node_id, private_key, socket)
}

fn spawn_listener_socket(
    node_id: String,
    private_key: Vec<u8>,
    socket: std::io::Result<TcpListener>,
) -> Result<Listener, String> {
    if private_key.len() != 32 { return Err("invalid local Noise key".into()); }
    let listener = socket.map_err(|e| format!("secure listener bind: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| format!("secure listener setup: {e}"))?;
    let (tx, rx) = mpsc::sync_channel(8);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    thread::spawn(move || {
        const MAX_ACTIVE: usize = 8;
        let active = Arc::new(AtomicUsize::new(0));

        while !stop_thread.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    if active.load(Ordering::Acquire) >= MAX_ACTIVE {
                        continue;
                    }
                    // Accepted sockets inherit nonblocking mode on some platforms.
                    // read_exact/write_all and bounded timeouts require blocking I/O.
                    if stream.set_nonblocking(false).is_err() {
                        continue;
                    }
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
    Ok(Listener { events: rx, stop })
}

pub(crate) fn send(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    private_key: &[u8],
    expected_peer_key: Option<&[u8]>,
    message: &str,
) -> SecureSendStatus {
    send_on_port(sender_id, peer_id, address, PORT, private_key, expected_peer_key, message)
}

fn send_on_port(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    port: u16,
    private_key: &[u8],
    expected_peer_key: Option<&[u8]>,
    message: &str,
) -> SecureSendStatus {
    let message_id = Uuid::new_v4().to_string();
    send_with_deadline(sender_id, peer_id, address, port, private_key, expected_peer_key, message, Instant::now() + SESSION_BUDGET, message_id)
}

fn send_with_deadline(
    sender_id: &str,
    peer_id: &str,
    address: &str,
    port: u16,
    private_key: &[u8],
    expected_peer_key: Option<&[u8]>,
    message: &str,
    deadline: Instant,
    message_id: String,
) -> SecureSendStatus {
    let mut observed_key = None;
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
        write_frame(&mut stream, &buf[..n], deadline)?;

        apply_io_timeout(&mut stream, deadline)?;
        let n = read_frame(&mut stream, &mut buf, deadline)?;
        hs.read_message(&buf[..n], &mut payload)
            .map_err(|e| format!("handshake 2: {e}"))?;
        let remote_key = hs.get_remote_static().ok_or("remote key missing")?;
        match expected_peer_key {
            Some(expected) if expected == remote_key => {}
            Some(_) => return Err("Noise identity key changed; message not sent".into()),
            None => {
                observed_key = Some(remote_key.to_vec());
                return Err("peer approval required; message not sent".into());
            }
        }

        let hello = serde_json::to_vec(&Hello { node_id: sender_id.into() })
            .map_err(|e| e.to_string())?;
        apply_io_timeout(&mut stream, deadline)?;
        let n = hs.write_message(&hello, &mut buf)
            .map_err(|e| format!("handshake 3: {e}"))?;
        write_frame(&mut stream, &buf[..n], deadline)?;

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
        write_frame(&mut stream, &buf[..n], deadline)?;

        apply_io_timeout(&mut stream, deadline)?;
        let n = read_frame(&mut stream, &mut buf, deadline)?;
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

    if let Some(public_key) = observed_key {
        return SecureSendStatus::NeedsTrust { peer_id: peer_id.into(), public_key };
    }
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
    events: &SyncSender<SecureEvent>,
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
    let n = read_frame(stream, &mut buf, deadline)?;
    hs.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("handshake 1: {e}"))?;

    apply_io_timeout(stream, deadline)?;
    let n = hs.write_message(&[], &mut buf)
        .map_err(|e| format!("handshake 2: {e}"))?;
    write_frame(stream, &buf[..n], deadline)?;

    apply_io_timeout(stream, deadline)?;
    let n = read_frame(stream, &mut buf, deadline)?;
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
    let fingerprint = fingerprint(&public_key);

    let mut transport = hs.into_transport_mode()
        .map_err(|e| format!("transport: {e}"))?;

    apply_io_timeout(stream, deadline)?;
    let n = read_frame(stream, &mut buf, deadline)?;
    let payload_len = transport.read_message(&buf[..n], &mut payload)
        .map_err(|e| format!("decrypt: {e}"))?;
    let envelope: Envelope = serde_json::from_slice(&payload[..payload_len])
        .map_err(|e| format!("envelope: {e}"))?;

    super::peer_trust::validate_peer(&hello.node_id, &public_key)?;
    if envelope.message.trim().is_empty() || envelope.message.len() > MAX_FRAME {
        return Err("invalid secure message body".into());
    }
    if envelope.to != node_id || envelope.from != hello.node_id {
        return Err("secure identity mismatch".into());
    }

    let (reply_tx, reply_rx) = mpsc::channel::<SecureReply>();
    events
        .try_send(SecureEvent::Received {
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
    write_frame(stream, &buf[..n], deadline)?;

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

fn write_frame(stream: &mut TcpStream, data: &[u8], deadline: Instant) -> Result<(), String> {
    if data.len() > MAX_FRAME + 4096 {
        return Err("frame too large".into());
    }
    let len = u32::try_from(data.len()).map_err(|_| "frame length overflow".to_string())?;
    write_bounded(stream, &len.to_be_bytes(), deadline)?;
    write_bounded(stream, data, deadline)
}

fn read_frame(stream: &mut TcpStream, buffer: &mut [u8], deadline: Instant) -> Result<usize, String> {
    let mut len = [0_u8; 4];
    read_bounded(stream, &mut len, deadline)?;
    let size = u32::from_be_bytes(len) as usize;
    if size > buffer.len() || size > MAX_FRAME + 4096 {
        return Err("frame exceeds receive limit".into());
    }
    read_bounded(stream, &mut buffer[..size], deadline)?;
    Ok(size)
}

fn read_bounded(stream: &mut TcpStream, mut buffer: &mut [u8], deadline: Instant) -> Result<(), String> {
    while !buffer.is_empty() {
        apply_io_timeout(stream, deadline)?;
        match stream.read(buffer) {
            Ok(0) => return Err("secure stream closed".into()),
            Ok(n) => buffer = &mut buffer[n..],
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("secure read: {error}")),
        }
    }
    Ok(())
}

fn write_bounded(stream: &mut TcpStream, mut data: &[u8], deadline: Instant) -> Result<(), String> {
    while !data.is_empty() {
        apply_io_timeout(stream, deadline)?;
        match stream.write(data) {
            Ok(0) => return Err("secure stream stopped writing".into()),
            Ok(n) => data = &data[n..],
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("secure write: {error}")),
        }
    }
    Ok(())
}

pub(crate) fn local_fingerprint(private_key: &[u8]) -> Result<String, String> {
    use snow::resolvers::{CryptoResolver, DefaultResolver};
    if private_key.len() != 32 { return Err("invalid local identity".into()); }
    let mut dh = DefaultResolver.resolve_dh(&snow::params::DHChoice::Curve25519)
        .ok_or("Curve25519 resolver unavailable")?;
    dh.set(private_key);
    Ok(fingerprint(dh.pubkey()))
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn decode_key(value: &str) -> Result<Vec<u8>, String> {
    if value.len() != 64 { return Err("identity key must have 32 bytes".into()); }
    decode_hex(value)
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid hex".into());
    }
    value.as_bytes().chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).map_err(|_| "invalid hex".into()))
        .collect()
}

pub(crate) fn fingerprint(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, encode_hex, fingerprint, spawn_listener_socket, send_on_port, SecureSendStatus, SecureEvent, SecureReply, PATTERN};
    use snow::{params::NoiseParams, Builder};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn fingerprint_matches_sha256_vector_and_local_public_key() {
        assert_eq!(fingerprint(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let keypair = Builder::new(PATTERN.parse().unwrap()).generate_keypair().unwrap();
        assert_eq!(super::local_fingerprint(&keypair.private).unwrap(), fingerprint(&keypair.public));
        assert!(super::local_fingerprint(&[]).is_err());
    }

    #[test]
    fn identity_load_errors_and_corruption_never_rotate_the_key() {
        let store = crate::store::Store::test_store(":memory:");
        store.set_checked("noise_static_private_hex", &"€".repeat(22)).unwrap();
        assert!(super::load_or_create_static_key(&store).is_err());
        assert_eq!(store.get_checked("noise_static_private_hex").unwrap(), Some("€".repeat(22)));
        store.test_execute("DROP TABLE kv;");
        assert!(super::load_or_create_static_key(&store).is_err());
    }

    #[test]
    fn identity_generation_requires_durable_write_and_is_stable() {
        let store = crate::store::Store::test_store(":memory:");
        store.test_execute("CREATE TRIGGER deny_identity BEFORE INSERT ON kv BEGIN SELECT RAISE(ABORT, 'write denied'); END;");
        assert!(super::load_or_create_static_key(&store).is_err());
        store.test_execute("DROP TRIGGER deny_identity;");
        let key = super::load_or_create_static_key(&store).unwrap();
        assert_eq!(key, super::load_or_create_static_key(&store).unwrap());
    }

    #[test]
    fn occupied_port_and_invalid_key_are_reported_synchronously() {
        let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let error = super::spawn_listener_bind("node-a".into(), vec![7; 32], "127.0.0.1", port).err().unwrap();
        assert!(error.contains("bind"));
        assert!(super::spawn_listener_bind("node-a".into(), Vec::new(), "127.0.0.1", 0).is_err());
    }

    #[test]
    fn unknown_or_replaced_receiver_never_receives_message_content() {
        for expected in [None, Some(vec![0; 32])] {
            let sender = Builder::new(PATTERN.parse().unwrap()).generate_keypair().unwrap();
            let receiver = Builder::new(PATTERN.parse().unwrap()).generate_keypair().unwrap();
            let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let port = socket.local_addr().unwrap().port();
            let listener = spawn_listener_socket("node-b".into(), receiver.private, Ok(socket)).unwrap();
            let result = send_on_port("node-a", "node-b", "127.0.0.1", port, &sender.private, expected.as_deref(), "must not transmit");
            match (expected, result) {
                (None, SecureSendStatus::NeedsTrust { public_key, .. }) => assert_eq!(public_key, receiver.public),
                (Some(_), SecureSendStatus::Failed { reason, .. }) => assert!(reason.contains("key changed")),
                (_, status) => panic!("unexpected status {status:?}"),
            }
            assert!(matches!(listener.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn database_failure_rejects_delivery_then_retry_commits_once() {
        let sender = Builder::new(PATTERN.parse().unwrap()).generate_keypair().unwrap();
        let receiver = Builder::new(PATTERN.parse().unwrap()).generate_keypair().unwrap();
        let store = crate::store::Store::test_store(":memory:");
        store.approve_secure_peer("node-a", &sender.public).unwrap();
        store.test_execute("CREATE TRIGGER deny_chat BEFORE INSERT ON chat_messages BEGIN SELECT RAISE(ABORT, 'write failure'); END;");
        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        let listener = spawn_listener_socket("node-b".into(), receiver.private, Ok(socket)).unwrap();
        for attempt in 0..3 {
            if attempt == 1 { store.test_execute("DROP TRIGGER deny_chat;"); }
            let private = sender.private.clone();
            let public = receiver.public.clone();
            let task = thread::spawn(move || super::send_with_deadline(
                "node-a", "node-b", "127.0.0.1", port, &private, Some(&public),
                "durable loopback", Instant::now() + Duration::from_secs(4), "stable-id".into()));
            let SecureEvent::Received { message_id, node_id, message, public_key, reply, .. } = listener.events.recv_timeout(Duration::from_secs(3)).unwrap();
            assert!(super::super::peer_trust::check_peer(&store, &node_id, &public_key).unwrap());
            let decision = match store.accept_secure_message(&message_id, &node_id, &message) {
                Ok(true) => SecureReply::Ack,
                Ok(false) => SecureReply::Reject("replayed message id".into()),
                Err(error) => SecureReply::Reject(error),
            };
            reply.send(decision).unwrap();
            match (attempt, task.join().unwrap()) {
                (0, SecureSendStatus::Failed { reason, .. }) => {
                    assert!(reason.contains("persist secure message"));
                    assert!(store.chat_messages().is_empty());
                }
                (1, SecureSendStatus::Delivered { .. }) => assert_eq!(store.chat_messages().len(), 1),
                (2, SecureSendStatus::Failed { reason, .. }) => {
                    assert!(reason.contains("replayed"));
                    assert_eq!(store.chat_messages().len(), 1);
                }
                (_, other) => panic!("unexpected delivery state {other:?}"),
            }
        }
    }

    #[test]
    fn trickling_frame_cannot_extend_absolute_deadline() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let writer = thread::spawn(move || {
            let mut socket = std::net::TcpStream::connect(address).unwrap();
            for byte in [0, 0, 0, 4, 1, 2, 3, 4] {
                if socket.write_all(&[byte]).is_err() { break; }
                thread::sleep(Duration::from_millis(40));
            }
        });
        let (mut stream, _) = listener.accept().unwrap();
        let started = Instant::now();
        assert!(super::read_frame(&mut stream, &mut [0; 32], started + Duration::from_millis(100)).is_err());
        assert!(started.elapsed() < Duration::from_millis(500));
        drop(stream);
        writer.join().unwrap();
    }

    #[test]
    fn exact_length_non_ascii_hex_is_an_error_not_a_panic() {
        let malformed = format!("{}a", "€".repeat(21));
        assert_eq!(malformed.len(), 64);
        assert!(super::decode_key(&malformed).is_err());
    }

    #[test]
    fn hex_roundtrip() {
        let data = [0, 1, 2, 15, 16, 255];
        assert_eq!(decode_hex(&encode_hex(&data)).unwrap(), data);
    }

    #[test]
    fn fingerprint_is_stable_for_public_key() {
        let key = [7_u8; 32];
        assert_eq!(fingerprint(&key), fingerprint(&key));
    }

    #[test]
    fn secure_loopback_rejection_is_not_reported_as_delivery() {
        let params: NoiseParams = PATTERN.parse().expect("noise params");
        let sender = Builder::new(params.clone()).generate_keypair().expect("sender keypair");
        let receiver = Builder::new(params).generate_keypair().expect("receiver keypair");

        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("test listener");
        let port = socket.local_addr().unwrap().port();
        let listener = spawn_listener_socket("node-b-reject".to_string(), receiver.private.clone(), Ok(socket)).expect("listener");

        let sender_key = sender.private.clone();
        let receiver_public = receiver.public.clone();
        let sender_thread = thread::spawn(move || {
            send_on_port(
                "node-a",
                "node-b-reject",
                "127.0.0.1",
                port,
                &sender_key,
                Some(&receiver_public),
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

        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("test listener");
        let port = socket.local_addr().unwrap().port();
        let listener = spawn_listener_socket("node-b".to_string(), receiver.private.clone(), Ok(socket)).expect("listener");

        let sender_key = sender.private.clone();
        let receiver_public = receiver.public.clone();
        let sender_thread = thread::spawn(move || {
            send_on_port(
                "node-a",
                "node-b",
                "127.0.0.1",
                port,
                &sender_key,
                Some(&receiver_public),
                "loopback secure test",
            )
        });

        let deadline = Instant::now() + Duration::from_secs(4);
        let message_id = loop {
            match listener.try_recv() {
                Ok(SecureEvent::Received {
                    message_id,
                    node_id,
                    message,
                    reply,
                    ..
                }) => {
                    assert_eq!(node_id, "node-a");
                    assert_eq!(message, "loopback secure test");
                    reply.send(SecureReply::Ack).expect("send ACK decision");
                    break message_id;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("secure listener did not receive message: {error:?}"),
            }
        };

        let status = sender_thread.join().expect("sender thread join");
        match status {
            SecureSendStatus::Delivered {
                message_id: delivered_id,
                peer_id,
            } => {
                assert_eq!(peer_id, "node-b");
                assert_eq!(delivered_id, message_id);
            }
            other => panic!("expected delivered secure message, got {other:?}"),
        }

        drop(listener);
    }
}
