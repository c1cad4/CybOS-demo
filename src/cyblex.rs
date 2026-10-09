//! Embedded BitTorrent engine for the native CybLex archive layer.
//!
//! CybLex keeps the long-lived rqbit session off the egui thread. The UI
//! exchanges explicit commands and periodic snapshots with the worker.

use librqbit::{AddTorrent, AddTorrentOptions, CreateTorrentOptions, Session, SessionOptions, SessionPersistenceConfig};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    sync::Arc,
    thread,
    time::Duration,
};

const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(750);
const MAX_PENDING_COMMANDS: usize = 32;
const MAX_SOURCE_LEN: usize = 64 * 1024;
const MAX_PATH_LEN: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct CybLexTorrent {
    pub(crate) id: usize,
    pub(crate) name: String,
    pub(crate) info_hash: String,
    pub(crate) magnet_uri: String,
    pub(crate) output_folder: String,
    pub(crate) progress_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) uploaded_bytes: u64,
    pub(crate) finished: bool,
    pub(crate) paused: bool,
    pub(crate) state: String,
    pub(crate) error: Option<String>,
}

pub(crate) enum CybLexEvent {
    Snapshot(Vec<CybLexTorrent>),
    Status(String),
    Error(String),
}

enum CybLexCommand {
    AddSource { source: String, output_folder: String },
    SeedPath { path: String },
    Pause { id: usize },
    Resume { id: usize },
    Forget { id: usize, delete_files: bool },
    Shutdown,
}

pub(crate) struct CybLexRuntime {
    tx: SyncSender<CybLexCommand>,
    rx: Receiver<CybLexEvent>,
}

impl Default for CybLexRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl CybLexRuntime {
    pub(crate) fn new() -> Self {
        // Bound pending UI commands so a busy UI cannot grow an unbounded queue.
        let (tx, command_rx) = mpsc::sync_channel(MAX_PENDING_COMMANDS);
        let (event_tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("cyblex-engine".into())
            .spawn(move || run_worker(command_rx, event_tx))
            .expect("failed to spawn CybLex worker");

        Self { tx, rx }
    }

    pub(crate) fn add_source(&self, source: String, output_folder: String) -> Result<(), String> {
        validate_source(&source)?;
        validate_path_text(&output_folder)?;
        self.send_command(CybLexCommand::AddSource {
            source: source.trim().to_string(),
            output_folder: expand_tilde(output_folder.trim()),
        })
    }

    pub(crate) fn seed_path(&self, path: String) -> Result<(), String> {
        validate_path_text(&path)?;
        let path = expand_tilde(path.trim());
        if !Path::new(&path).exists() {
            return Err(format!("Path does not exist: {path}"));
        }
        self.send_command(CybLexCommand::SeedPath { path })
    }

    pub(crate) fn pause(&self, id: usize) -> Result<(), String> {
        self.send_command(CybLexCommand::Pause { id })
    }

    pub(crate) fn resume(&self, id: usize) -> Result<(), String> {
        self.send_command(CybLexCommand::Resume { id })
    }

    pub(crate) fn forget(&self, id: usize, delete_files: bool) -> Result<(), String> {
        self.send_command(CybLexCommand::Forget { id, delete_files })
    }

    fn send_command(&self, command: CybLexCommand) -> Result<(), String> {
        send_bounded(&self.tx, command)
    }

    pub(crate) fn poll(&self) -> Vec<CybLexEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}

fn send_bounded<T>(tx: &SyncSender<T>, command: T) -> Result<(), String> {
    tx.try_send(command).map_err(|error| match error {
        TrySendError::Full(_) => {
            "CybLex command queue is full; wait for the current operation and retry".into()
        }
        TrySendError::Disconnected(_) => "CybLex worker is not running".into(),
    })
}

impl Drop for CybLexRuntime {
    fn drop(&mut self) {
        let _ = self.tx.send(CybLexCommand::Shutdown);
    }
}

fn run_worker(command_rx: Receiver<CybLexCommand>, event_tx: Sender<CybLexEvent>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = event_tx.send(CybLexEvent::Error(format!("Tokio runtime init failed: {error}")));
            return;
        }
    };

    let event_tx_inner = event_tx.clone();
    let result = runtime.block_on(async move {
        let download_dir = default_download_dir();
        let persistence_dir = default_persistence_dir();

        fs::create_dir_all(&download_dir)
            .map_err(|e| format!("create CybLex directory: {e}"))?;
        fs::create_dir_all(&persistence_dir)
            .map_err(|e| format!("create CybLex persistence directory: {e}"))?;

        let _ = event_tx_inner.send(CybLexEvent::Status(format!(
            "CYBLEX · IDLE · P2P OFF · {} · PERSISTENCE READY",
            download_dir.display()
        )));

        let mut session: Option<Arc<Session>> = None;
        let mut last_snapshot = tokio::time::Instant::now() - SNAPSHOT_INTERVAL;
        let mut running = true;

        while running {
            loop {
                match command_rx.try_recv() {
                    Ok(CybLexCommand::AddSource { source, output_folder }) => {
                        if session.is_none() {
                            session = Some(
                                Session::new_with_opts(
                                    download_dir.clone(),
                                    SessionOptions {
                                        persistence: Some(SessionPersistenceConfig::Json {
                                            folder: Some(persistence_dir.clone()),
                                        }),
                                        ..Default::default()
                                    },
                                )
                                    .await
                                    .map_err(|e| format!("CybLex session init: {e:#}"))?,
                            );
                            let _ = event_tx_inner.send(CybLexEvent::Status("CYBLEX · P2P SESSION ACTIVE".into()));
                        }

                        let active = session.as_ref().expect("session initialized");
                        let mut options = AddTorrentOptions::default();
                        options.output_folder = Some(output_folder);

                        match active.add_torrent(AddTorrent::from_url(source.as_str()), Some(options)).await {
                            Ok(response) => match response.into_handle() {
                                Some(handle) => {
                                    let _ = event_tx_inner.send(CybLexEvent::Status(format!(
                                        "CYBLEX · ADDED · {} · {}",
                                        handle.name().unwrap_or_else(|| "TORRENT".into()),
                                        handle.info_hash().as_string()
                                    )));
                                }
                                None => {
                                    let _ = event_tx_inner.send(CybLexEvent::Error(
                                        "CybLex source returned no managed torrent".into()
                                    ));
                                }
                            },
                            Err(error) => {
                                let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                    "CybLex add failed: {error:#}"
                                )));
                            }
                        }
                    }
                    Ok(CybLexCommand::SeedPath { path }) => {
                        if session.is_none() {
                            session = Some(
                                Session::new_with_opts(
                                    download_dir.clone(),
                                    SessionOptions {
                                        persistence: Some(SessionPersistenceConfig::Json {
                                            folder: Some(persistence_dir.clone()),
                                        }),
                                        ..Default::default()
                                    },
                                )
                                .await
                                .map_err(|e| format!("CybLex session init: {e:#}"))?,
                            );
                            let _ = event_tx_inner.send(CybLexEvent::Status("CYBLEX · P2P SESSION ACTIVE".into()));
                        }

                        let active = session.as_ref().expect("session initialized");
                        match active.create_and_serve_torrent(
                            Path::new(&path),
                            CreateTorrentOptions::default(),
                        ).await {
                            Ok((torrent, handle)) => {
                                let magnet = torrent.as_magnet().to_string();
                                let torrent_path = torrent_sidecar_path(Path::new(&path));

                                let sidecar = torrent.as_bytes()
                                    .map_err(|e| e.to_string())
                                    .and_then(|bytes| write_sidecar_new(&torrent_path, &bytes));

                                let detail = match sidecar {
                                    Ok(()) => format!(
                                        "CYBLEX · SEEDING · {} · {}",
                                        handle.name().unwrap_or_else(|| "CONTENT".into()),
                                        torrent_path.display()
                                    ),
                                    Err(error) => format!(
                                        "CYBLEX · SEEDING · {} · .torrent save failed: {}",
                                        handle.name().unwrap_or_else(|| "CONTENT".into()),
                                        error
                                    ),
                                };

                                let _ = event_tx_inner.send(CybLexEvent::Status(format!(
                                    "{} · MAGNET READY · {}",
                                    detail, magnet
                                )));
                            }
                            Err(error) => {
                                let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                    "CybLex seed failed: {error:#}"
                                )));
                            }
                        }
                    }
                    Ok(CybLexCommand::Pause { id }) => {
                        match session.as_ref().and_then(|s| s.get(id.into())) {
                            Some(handle) => {
                                if let Some(active) = session.as_ref() {
                                    if let Err(error) = active.pause(&handle).await {
                                        let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                            "CybLex pause failed: {error:#}"
                                        )));
                                    }
                                }
                            }
                            None => {
                                let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                    "CybLex torrent {id} not found"
                                )));
                            }
                        }
                    }
                    Ok(CybLexCommand::Resume { id }) => {
                        match session.as_ref().and_then(|s| s.get(id.into())) {
                            Some(handle) => {
                                if let Some(active) = session.as_ref() {
                                    if let Err(error) = active.unpause(&handle).await {
                                        let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                            "CybLex resume failed: {error:#}"
                                        )));
                                    }
                                }
                            }
                            None => {
                                let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                    "CybLex torrent {id} not found"
                                )));
                            }
                        }
                    }
                    Ok(CybLexCommand::Forget { id, delete_files }) => {
                        match session.as_ref() {
                            Some(active) => {
                                if let Err(error) = active.delete(id.into(), delete_files).await {
                                    let _ = event_tx_inner.send(CybLexEvent::Error(format!(
                                        "CybLex delete failed: {error:#}"
                                    )));
                                }
                            }
                            None => {
                                let _ = event_tx_inner.send(CybLexEvent::Error(
                                    "CybLex P2P session is not active".into()
                                ));
                            }
                        }
                    }
                    Ok(CybLexCommand::Shutdown) => {
                        running = false;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        running = false;
                        break;
                    }
                }
            }

            if !running {
                if let Some(active) = session.as_ref() {
                    let _ = active.stop().await;
                }
                break;
            }

            if last_snapshot.elapsed() >= SNAPSHOT_INTERVAL {
                let snapshot = match session.as_ref() {
                    Some(active) => active.with_torrents(|iter| {
                        iter.map(|(_, torrent)| {
                            let stats = torrent.stats();
                            let info_hash = torrent.info_hash().as_string();
                            CybLexTorrent {
                                id: torrent.id(),
                                name: torrent.name().unwrap_or_else(|| format!("torrent-{}", torrent.id())),
                                magnet_uri: format!("magnet:?xt=urn:btih:{info_hash}"),
                                info_hash,
                                output_folder: torrent.output_folder().display().to_string(),
                                progress_bytes: stats.progress_bytes,
                                total_bytes: stats.total_bytes,
                                uploaded_bytes: stats.uploaded_bytes,
                                finished: stats.finished,
                                paused: torrent.is_paused(),
                                state: format!("{:?}", stats.state),
                                error: stats.error,
                            }
                        }).collect::<Vec<_>>()
                    }),
                    None => Vec::new(),
                };

                let _ = event_tx_inner.send(CybLexEvent::Snapshot(snapshot));
                last_snapshot = tokio::time::Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok::<(), String>(())
    });

    if let Err(error) = result {
        let _ = event_tx.send(CybLexEvent::Error(error));
    }
}

pub(crate) fn validate_source(source: &str) -> Result<(), String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("Enter a magnet link or .torrent URL".into());
    }
    if source.len() > MAX_SOURCE_LEN {
        return Err("Torrent source is too large".into());
    }

    let lower = source.to_ascii_lowercase();
    if lower.starts_with("magnet:?")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
    {
        Ok(())
    } else {
        Err("CybLex accepts magnet:, http:// or https:// torrent sources".into())
    }
}

fn validate_path_text(path: &str) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Path cannot be empty".into());
    }
    if path.len() > MAX_PATH_LEN {
        return Err("Path is too large".into());
    }
    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| ".".into());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return std::env::var("HOME")
            .map(|home| format!("{home}/{rest}"))
            .unwrap_or_else(|_| path.into());
    }
    path.into()
}

/// Never overwrite an existing sidecar or an unrelated user file.
fn write_sidecar_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("refusing to overwrite existing torrent sidecar: {}", path.display())
            } else {
                format!("cannot create torrent sidecar {}: {error}", path.display())
            }
        })?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write torrent sidecar {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync torrent sidecar {}: {error}", path.display()))?;
    Ok(())
}

fn default_download_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("Downloads")
        .join("CybLex")
}

fn default_persistence_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("Library")
        .join("Application Support")
        .join("cybOS")
        .join("CybLex")
        .join("rqbit")
}

fn torrent_sidecar_path(path: &Path) -> PathBuf {
    let base = path.file_name().and_then(|n| n.to_str()).filter(|s| !s.is_empty()).unwrap_or("cyblex-content");
    let clean = base.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect::<String>();
    path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{clean}.torrent"))
}

#[cfg(test)]
mod tests {
    use super::{send_bounded, validate_source, validate_path_text, write_sidecar_new};
    use std::{fs, sync::mpsc};

    #[test]
    fn command_queue_reports_backpressure_without_blocking() {
        let (tx, _rx) = mpsc::sync_channel(1);
        tx.try_send("first").expect("first command fits");
        let error = send_bounded(&tx, "second").unwrap_err();
        assert!(error.contains("queue is full"));
    }

    #[test]
    fn command_queue_reports_stopped_worker() {
        let (tx, rx) = mpsc::sync_channel::<()>(1);
        drop(rx);
        let error = send_bounded(&tx, ()).unwrap_err();
        assert!(error.contains("worker is not running"));
    }

    #[test]
    fn rejects_empty_and_unsupported_sources() {
        assert!(validate_source(" ").is_err());
        assert!(validate_source("file:///etc/passwd").is_err());
        assert!(validate_source("ftp://example.org/a.torrent").is_err());
    }

    #[test]
    fn accepts_magnet_and_https_torrent_sources() {
        assert!(validate_source("magnet:?xt=urn:btih:abc").is_ok());
        assert!(validate_source("https://example.org/file.torrent").is_ok());
    }

    #[test]
    fn rejects_empty_or_oversized_paths() {
        assert!(validate_path_text("").is_err());
        assert!(validate_path_text(&"x".repeat(4097)).is_err());
        assert!(validate_path_text("~/Downloads/CybLex").is_ok());
    }

    #[test]
    fn sidecar_writer_never_overwrites_existing_file() {
        let dir = std::env::temp_dir().join(format!("cyblex-sidecar-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("content.torrent");
        fs::write(&path, b"existing").unwrap();

        let error = write_sidecar_new(&path, b"replacement").unwrap_err();
        assert!(error.contains("refusing to overwrite"));
        assert_eq!(fs::read(&path).unwrap(), b"existing");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sidecar_writer_creates_file_atomically_without_overwriting() {
        let dir = std::env::temp_dir().join(format!("cyblex-sidecar-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("content.torrent");

        write_sidecar_new(&path, b"torrent-data").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"torrent-data");

        fs::remove_dir_all(&dir).unwrap();
    }
}
