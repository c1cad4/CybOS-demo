//! Explicit user controls for an already-running offline CybHero laboratory node.
use serde_json::{Value, json};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

pub(crate) const GENESIS: &str = "e162f6367a0140999726bb9564b955cc5f03b33121f1531e48d545387e864345";

#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    pub height: u64,
    pub active: bool,
    pub speed: Option<u64>,
    pub threads: Option<u64>,
    address: String,
}

pub(crate) enum Operation {
    Check,
    Start {
        address: String,
        threads: usize,
        consent: bool,
    },
    Stop,
}

pub(crate) fn thread_limit() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(4)
}

fn validate_start(address: &str, threads: usize, consent: bool) -> Result<(), String> {
    if !consent {
        return Err("Нужно явное согласие на нагрузку CPU".into());
    }
    if threads == 0 || threads > thread_limit() {
        return Err("Недопустимое число CPU-потоков".into());
    }
    if !(94..=128).contains(&address.len())
        || !address
            .bytes()
            .all(|b| b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(&b))
    {
        return Err("Нужен публичный адрес тестового кошелька; checksum проверит узел".into());
    }
    Ok(())
}

struct Client {
    agent: ureq::Agent,
    base: String,
}
impl Client {
    fn new(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("Порт не может быть нулевым".into());
        }
        let agent = ureq::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(4)))
            .build()
            .into();
        Ok(Self {
            agent,
            base: format!("http://127.0.0.1:{port}"),
        })
    }
    fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let mut response = self
            .agent
            .post(format!("{}{path}", self.base))
            .send_json(body)
            .map_err(|e| format!("Локальный узел недоступен: {e}"))?;
        if response.status().as_u16() != 200 {
            return Err("Неожиданный HTTP-ответ узла".into());
        }
        let raw = response
            .body_mut()
            .with_config()
            .limit(256 * 1024)
            .read_to_string()
            .map_err(|e| format!("Не удалось прочитать ответ узла: {e}"))?;
        serde_json::from_str(&raw).map_err(|_| "Некорректный JSON узла".into())
    }
    fn rpc(&self, method: &str, params: Value) -> Result<Value, String> {
        let result = self.post(
            "/json_rpc",
            json!({"jsonrpc":"2.0","id":"cybos-lab","method":method,"params":params}),
        )?;
        if result["jsonrpc"] != "2.0"
            || result["id"] != "cybos-lab"
            || result.get("error").is_some()
            || !result["result"].is_object()
        {
            return Err("Узел отклонил RPC-запрос".into());
        }
        Ok(result["result"].clone())
    }
    fn inspect(&self) -> Result<Snapshot, String> {
        let info = self.rpc("get_info", json!({}))?;
        let header = self.rpc("get_block_header_by_height", json!({"height":0}))?;
        validate_identity(&info, &header)?;
        let status = self.post("/mining_status", json!({}))?;
        trusted(&status)?;
        let active = status["active"]
            .as_bool()
            .ok_or("Неизвестное состояние майнера")?;
        Ok(Snapshot {
            height: info["height"].as_u64().unwrap(),
            active,
            speed: if active {
                status["speed"].as_u64()
            } else {
                None
            },
            threads: if active {
                status["threads_count"].as_u64()
            } else {
                None
            },
            address: status["address"].as_str().unwrap_or("").into(),
        })
    }
}

fn trusted(value: &Value) -> Result<(), String> {
    if value["status"] != "OK" || value.get("untrusted").is_some_and(|v| v != false) {
        return Err("Узел не подтвердил локальный результат".into());
    }
    Ok(())
}

fn validate_identity(info: &Value, result: &Value) -> Result<(), String> {
    trusted(info)?;
    trusted(result)?;
    let h = &result["block_header"];
    if info["nettype"] != "fakechain"
        || info["offline"] != true
        || ["mainnet", "testnet", "stagenet"]
            .iter()
            .any(|k| info[*k] != false)
        || info["height"].as_u64().is_none_or(|n| n == 0)
        || h["hash"] != GENESIS
        || h["height"].as_u64() != Some(0)
        || h["nonce"].as_u64() != Some(426942690)
        || h["orphan_status"] != false
    {
        return Err("Разрешён только идентифицированный offline CybHero lab".into());
    }
    Ok(())
}

pub(crate) fn execute(port: u16, operation: Operation) -> Result<Snapshot, String> {
    if let Operation::Start {
        ref address,
        threads,
        consent,
    } = operation
    {
        validate_start(address, threads, consent)?;
    }
    let client = Client::new(port)?;
    let before = client.inspect()?; // Recheck identity immediately before every mutation.
    match operation {
        Operation::Check => Ok(before),
        Operation::Start {
            address, threads, ..
        } => {
            if before.active {
                return Err("Узел уже майнит; сначала остановите его".into());
            }
            let result = client.post(
                "/start_mining",
                json!({"miner_address":address,
                "threads_count":threads,"do_background_mining":false,"ignore_battery":false}),
            )?;
            trusted(&result)?;
            let after = client.inspect()?;
            if !after.active || after.address != address || after.threads != Some(threads as u64) {
                return Err(
                    "Запуск не подтверждён; проверьте узел и при необходимости нажмите Остановить"
                        .into(),
                );
            }
            Ok(after)
        }
        Operation::Stop => {
            if before.active {
                trusted(&client.post("/stop_mining", json!({}))?)?;
            }
            let after = client.inspect()?;
            if after.active {
                return Err("Остановка не подтверждена".into());
            }
            Ok(after)
        }
    }
}

pub(crate) struct Panel {
    pub report_path: String,
    pub report: Option<crate::mining_report::Report>,
    pub report_error: String,
    pub port: u16,
    pub address: String,
    pub threads: usize,
    pub consent: bool,
    pub status: String,
    pub snapshot: Option<Snapshot>,
    pub checked: Option<Instant>,
    task: Option<Receiver<Result<Snapshot, String>>>,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            report_path: String::new(),
            report: None,
            report_error: String::new(),
            port: 28981,
            address: String::new(),
            threads: 1,
            consent: false,
            status: "Статус подключения не проверен".into(),
            snapshot: None,
            checked: None,
            task: None,
        }
    }
}
impl Panel {
    pub fn busy(&self) -> bool {
        self.task.is_some()
    }
    pub fn fresh(&self) -> bool {
        self.checked
            .is_some_and(|t| t.elapsed() < Duration::from_secs(10))
    }
    pub fn invalidate(&mut self) {
        self.snapshot = None;
        self.checked = None;
        self.status = "Статус подключения не проверен".into();
    }
    pub fn request(&mut self, operation: Operation, ctx: &eframe::egui::Context) {
        if self.busy() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let port = self.port;
        let ctx = ctx.clone();
        self.task = Some(rx);
        self.status = "Ожидание ответа локального узла…".into();
        std::thread::spawn(move || {
            let _ = tx.send(execute(port, operation));
            ctx.request_repaint();
        });
    }
    pub fn poll(&mut self) {
        let result = self.task.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(value) => Some(value),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Проверка узла прервалась".into())),
        });
        if let Some(result) = result {
            self.task = None;
            match result {
                Ok(snapshot) => {
                    self.status = if snapshot.active {
                        "Тестовый майнер работает"
                    } else {
                        "Тестовый майнер остановлен"
                    }
                    .into();
                    self.snapshot = Some(snapshot);
                    self.checked = Some(Instant::now());
                }
                Err(error) => {
                    self.snapshot = None;
                    self.checked = None;
                    self.status = error;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn fake_server(replies: Vec<Value>) -> (u16, std::thread::JoinHandle<Vec<(String, Value)>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut buffer = [0u8; 2048];
                let split = loop {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    raw.extend_from_slice(&buffer[..n]);
                    if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let header = String::from_utf8(raw[..split].to_vec()).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                while raw.len() < split + length {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    raw.extend_from_slice(&buffer[..n]);
                }
                let path = header
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_string();
                requests.push((
                    path,
                    serde_json::from_slice(&raw[split..split + length]).unwrap(),
                ));
                let body = reply.to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",body.len(),body).unwrap();
            }
            requests
        });
        (port, task)
    }
    fn wrap(value: Value) -> Value {
        json!({"jsonrpc":"2.0","id":"cybos-lab","result":value})
    }
    fn info() -> Value {
        json!({"status":"OK","nettype":"fakechain","offline":true,"height":2,"mainnet":false,"testnet":false,"stagenet":false})
    }
    fn header() -> Value {
        json!({"status":"OK","block_header":{"hash":GENESIS,"height":0,"nonce":426942690u64,"orphan_status":false}})
    }
    fn status(active: bool) -> Value {
        json!({"status":"OK","active":active,"threads_count":1,"speed":123,"address":"1".repeat(97)})
    }

    #[test]
    fn http_start_requires_identity_and_confirms_measurements() {
        let (port, server) = fake_server(vec![
            wrap(info()),
            wrap(header()),
            status(false),
            json!({"status":"OK"}),
            wrap(info()),
            wrap(header()),
            status(true),
        ]);
        let result = execute(
            port,
            Operation::Start {
                address: "1".repeat(97),
                threads: 1,
                consent: true,
            },
        )
        .unwrap();
        assert!(result.active);
        assert_eq!(result.speed, Some(123));
        let requests = server.join().unwrap();
        assert_eq!(requests[3].0, "/start_mining");
        assert_eq!(requests[3].1["threads_count"], 1);
        assert_eq!(requests[3].1["do_background_mining"], false);
        assert_eq!(requests[3].1["ignore_battery"], false);
    }
    #[test]
    fn http_foreign_network_never_receives_start() {
        let mut bad = info();
        bad["mainnet"] = json!(true);
        let (port, server) = fake_server(vec![wrap(bad), wrap(header())]);
        assert!(
            execute(
                port,
                Operation::Start {
                    address: "1".repeat(97),
                    threads: 1,
                    consent: true
                }
            )
            .is_err()
        );
        assert!(
            server
                .join()
                .unwrap()
                .iter()
                .all(|(path, _)| path == "/json_rpc")
        );
    }
    #[test]
    fn http_stop_is_rechecked_and_confirmed() {
        let (port, server) = fake_server(vec![
            wrap(info()),
            wrap(header()),
            status(true),
            json!({"status":"OK"}),
            wrap(info()),
            wrap(header()),
            status(false),
        ]);
        assert!(!execute(port, Operation::Stop).unwrap().active);
        assert_eq!(server.join().unwrap()[3].0, "/stop_mining");
    }
    #[test]
    fn local_client_disables_proxies_and_redirects() {
        let client = Client::new(28981).unwrap();
        assert!(client.agent.config().proxy().is_none());
        assert_eq!(client.agent.config().max_redirects(), 0);
        assert!(Client::new(0).is_err());
    }
    #[test]
    fn no_implicit_power_use() {
        assert!(validate_start(&"1".repeat(97), 1, false).is_err());
        assert!(validate_start(&"1".repeat(97), 0, true).is_err());
        assert!(validate_start(&"1".repeat(97), thread_limit() + 1, true).is_err());
        assert!(validate_start("seed words", 1, true).is_err());
    }
    #[test]
    fn public_and_foreign_chain_rejected() {
        let info = json!({"status":"OK","nettype":"fakechain","offline":true,"height":1,"mainnet":false,"testnet":false,"stagenet":false});
        let header = json!({"status":"OK","block_header":{"hash":GENESIS,"height":0,"nonce":426942690u64,"orphan_status":false}});
        assert!(validate_identity(&info, &header).is_ok());
        for key in ["mainnet", "testnet", "stagenet"] {
            let mut bad = info.clone();
            bad[key] = json!(true);
            assert!(validate_identity(&bad, &header).is_err());
        }
        let mut bad = header.clone();
        bad["block_header"]["hash"] = json!("a".repeat(64));
        assert!(validate_identity(&info, &bad).is_err());
        let mut bad = info;
        bad["offline"] = json!(false);
        assert!(validate_identity(&bad, &header).is_err());
    }
}
