//! Narrow loopback HTTP bridge. Reads through rqbit's piece-aware, seekable stream.
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use std::{
    future::Future,
    io::SeekFrom,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncSeekExt, ReadBuf},
    sync::{OwnedSemaphorePermit, Semaphore},
};

#[derive(Clone)]
struct StreamState {
    torrent: Arc<librqbit::ManagedTorrent>,
    file: usize,
    token: String,
    host: String,
    mime: &'static str,
    permits: Arc<Semaphore>,
    cancel: tokio_util::sync::CancellationToken,
}
pub(crate) struct StreamLease {
    pub url: String,
    cancel: tokio_util::sync::CancellationToken,
}
impl Drop for StreamLease {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub(crate) async fn start(
    torrent: Arc<librqbit::ManagedTorrent>,
    file: usize,
    mime: &'static str,
) -> Result<StreamLease, String> {
    let metadata = torrent
        .metadata
        .load_full()
        .ok_or("Ожидаем метаданные торрента")?;
    if metadata.file_infos.get(file).is_none() {
        return Err("Файл не найден в торренте".into());
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let host = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .to_string();
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let url = format!("http://{host}/watch/{token}");
    let cancel = tokio_util::sync::CancellationToken::new();
    let state = StreamState {
        torrent,
        file,
        token,
        host,
        mime,
        permits: Arc::new(Semaphore::new(4)),
        cancel: cancel.clone(),
    };
    let router = Router::new()
        .route("/watch/{token}", get(stream))
        .with_state(state);
    let stopped = cancel.clone();
    tokio::spawn(async move {
        // Each playback URL expires when its lease is dropped. No general rqbit API is exposed.
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                stopped.cancelled().await;
            })
            .await;
    });
    Ok(StreamLease { url, cancel })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ByteRange {
    pub start: u64,
    pub end: u64,
    pub partial: bool,
}
/// Inclusive end. Implements one RFC 9110 byte range, including suffix/open ranges.
pub(crate) fn range(header: Option<&str>, len: u64) -> Result<ByteRange, ()> {
    let Some(header) = header else {
        return Ok(ByteRange {
            start: 0,
            end: len.saturating_sub(1),
            partial: false,
        });
    };
    let value = header.strip_prefix("bytes=").ok_or(())?;
    if len == 0 || value.contains(',') || value.len() > 80 {
        return Err(());
    }
    let (start, end) = value.split_once('-').ok_or(())?;
    if start.is_empty() {
        let count: u64 = end.parse().map_err(|_| ())?;
        if count == 0 {
            return Err(());
        }
        return Ok(ByteRange {
            start: len.saturating_sub(count),
            end: len - 1,
            partial: true,
        });
    }
    let start: u64 = start.parse().map_err(|_| ())?;
    let end = if end.is_empty() {
        len - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(len - 1)
    };
    if start >= len || start > end {
        return Err(());
    }
    Ok(ByteRange {
        start,
        end,
        partial: true,
    })
}
struct PermitReader<T> {
    inner: T,
    _permit: OwnedSemaphorePermit,
    cancelled: Pin<Box<tokio_util::sync::WaitForCancellationFutureOwned>>,
}
impl<T: AsyncRead + Unpin> AsyncRead for PermitReader<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.cancelled.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
async fn stream(
    State(state): State<StreamState>,
    axum::extract::Path(token): axum::extract::Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    if state.cancel.is_cancelled() {
        return StatusCode::GONE.into_response();
    }
    if token != state.token
        || headers.get("host").and_then(|h| h.to_str().ok()) != Some(&state.host)
        || headers.contains_key("origin")
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(permit) = state.permits.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    let Ok(mut file) = state.torrent.clone().stream(state.file).await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let len = file.len();
    let header = match headers.get("range") {
        Some(value) => match value.to_str() {
            Ok(value) => Some(value),
            Err(_) => return StatusCode::RANGE_NOT_SATISFIABLE.into_response(),
        },
        None => None,
    };
    let selected = match range(header, len) {
        Ok(selected) => selected,
        Err(_) => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("Content-Range", format!("bytes */{len}"))],
            )
                .into_response();
        }
    };
    let count = if len == 0 {
        0
    } else {
        selected.end - selected.start + 1
    };
    if file.seek(SeekFrom::Start(selected.start)).await.is_err() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        let reader = PermitReader {
            inner: file.take(count),
            _permit: permit,
            cancelled: Box::pin(state.cancel.clone().cancelled_owned()),
        };
        Body::from_stream(tokio_util::io::ReaderStream::with_capacity(
            reader,
            64 * 1024,
        ))
    };
    let mut response = body.into_response();
    *response.status_mut() = if selected.partial {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let out = response.headers_mut();
    out.insert("Accept-Ranges", "bytes".parse().unwrap());
    out.insert("Content-Type", state.mime.parse().unwrap());
    out.insert("Content-Length", count.to_string().parse().unwrap());
    out.insert("Cache-Control", "no-store".parse().unwrap());
    out.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    if selected.partial {
        out.insert(
            "Content-Range",
            format!("bytes {}-{}/{len}", selected.start, selected.end)
                .parse()
                .unwrap(),
        );
    }
    response
}
pub(crate) fn video_mime(path: &std::path::Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "mkv" => Some("video/x-matroska"),
        "webm" => Some("video/webm"),
        "avi" => Some("video/x-msvideo"),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rqbit_http_stream_serves_exact_ranges_and_rejects_web_origins() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let dir =
                std::env::temp_dir().join(format!("cyb-stream-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            let content: Vec<u8> = (0..160000).map(|i| (i % 251) as u8).collect();
            let path = dir.join("own-test-video.mp4");
            std::fs::write(&path, &content).unwrap();
            let session = librqbit::Session::new_with_opts(
                dir.clone(),
                librqbit::SessionOptions {
                    dht: None,
                    disable_trackers: true,
                    listen: None,
                    connect: None,
                    persistence: None,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let (_, torrent) = session
                .create_and_serve_torrent(&path, librqbit::CreateTorrentOptions::default())
                .await
                .unwrap();
            torrent.wait_until_initialized().await.unwrap();
            let lease = start(torrent, 0, "video/mp4").await.unwrap();
            let url = lease.url.clone();
            let expected = content[70000..70100].to_vec();
            tokio::task::spawn_blocking(move || {
                let mut reply = ureq::get(&url)
                    .header("Range", "bytes=70000-70099")
                    .call()
                    .unwrap();
                assert_eq!(reply.status().as_u16(), 206);
                assert_eq!(
                    reply.headers().get("Content-Range").unwrap(),
                    "bytes 70000-70099/160000"
                );
                assert_eq!(reply.body_mut().read_to_vec().unwrap(), expected);
                let mut suffix = ureq::get(&url).header("Range", "bytes=-8").call().unwrap();
                assert_eq!(suffix.body_mut().read_to_vec().unwrap(), content[159992..]);
                let head = ureq::head(&url).call().unwrap();
                assert_eq!(head.headers().get("Content-Length").unwrap(), "160000");
                assert!(matches!(
                    ureq::get(&url)
                        .header("Origin", "https://example.org")
                        .call(),
                    Err(ureq::Error::StatusCode(403))
                ));
                let unauthorized = url.rsplit_once('/').unwrap().0.to_string() + "/wrong-token";
                assert!(matches!(
                    ureq::get(&unauthorized).call(),
                    Err(ureq::Error::StatusCode(403))
                ));
                assert!(matches!(
                    ureq::get(&url).header("Range", "bytes=160000-").call(),
                    Err(ureq::Error::StatusCode(416))
                ));
            })
            .await
            .unwrap();
            drop(lease);
            session.stop().await;
            std::fs::remove_dir_all(dir).unwrap();
        });
    }
    #[test]
    fn cancellation_wakes_a_reader_waiting_for_missing_data() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let cancel = tokio_util::sync::CancellationToken::new();
            let permits = Arc::new(Semaphore::new(1));
            let (reader, _writer) = tokio::io::duplex(64);
            let mut reader = PermitReader {
                inner: reader,
                _permit: permits.clone().acquire_owned().await.unwrap(),
                cancelled: Box::pin(cancel.clone().cancelled_owned()),
            };
            let job = tokio::spawn(async move {
                let mut buffer = [0; 8];
                reader.read(&mut buffer).await.unwrap()
            });
            tokio::task::yield_now().await;
            cancel.cancel();
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_millis(250), job)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
            assert_eq!(permits.available_permits(), 1);
        });
    }
    #[test]
    fn range_handles_seek_suffix_and_unsatisfiable_requests() {
        assert_eq!(
            range(Some("bytes=20-39"), 100).unwrap(),
            ByteRange {
                start: 20,
                end: 39,
                partial: true
            }
        );
        assert_eq!(range(Some("bytes=90-"), 100).unwrap().end, 99);
        assert_eq!(range(Some("bytes=-10"), 100).unwrap().start, 90);
        assert_eq!(range(Some("bytes=-200"), 100).unwrap().start, 0);
        assert_eq!(range(Some("bytes=0-999"), 100).unwrap().end, 99);
        for bad in [
            "bytes=100-",
            "bytes=9-1",
            "bytes=-0",
            "bytes=0-1,5-9",
            "units=0-1",
            "bytes=",
            "bytes=0-18446744073709551616",
        ] {
            assert!(range(Some(bad), 100).is_err(), "{bad}");
        }
        assert!(range(Some("bytes=0-"), 0).is_err());
        assert!(!range(None, 0).unwrap().partial);
    }
}

pub(crate) fn launch_player(url: &str, native: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if native {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let dir = exe.parent().ok_or("Не найден каталог приложения")?;
            let packaged = dir
                .parent()
                .ok_or("Не найден пакет приложения")?
                .join("Resources/CybPlayer.app");
            let development = dir.join("CybPlayer.app");
            let bundle = if packaged.is_dir() {
                packaged
            } else {
                development
            };
            if !bundle.is_dir() {
                return Err("Плеер не собран: запусти scripts/build_player_macos.sh".into());
            }
            std::process::Command::new("/usr/bin/open")
                .args(["-n", "-a"])
                .arg(bundle)
                .arg("--args")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            if !std::path::Path::new("/Applications/VLC.app").is_dir() {
                return Err("Для MKV/WebM/AVI установи VLC. CybPlayer использует системные кодеки macOS для MP4/MOV".into());
            }
            std::process::Command::new("/usr/bin/open")
                .args(["-n", "-a", "VLC", "--args"])
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = native;
        std::process::Command::new("vlc")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|_| "Установи VLC для потокового просмотра".into())
    }
}
