//! What the `lobby_*` tests share: the stub `claude`, a server subprocess
//! on a free port, and a WebSocket client that reads views.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

/// A `claude -p` that answers every schema deterministically: picks card
/// 0, builds 23 of the pool over 9 Island 8 Swamp, and fills a game
/// schema the way `a_seed_writes_one_log.rs`'s stub does.
pub const STUB: &str = r##"#!/usr/bin/env python3
import sys, json, hashlib
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
msg = sys.stdin.read(); sid = ""; sc = None
for i, a in enumerate(argv):
    if a in ("--session-id", "--resume") and i + 1 < len(argv): sid = argv[i + 1]
    if a == "--json-schema" and i + 1 < len(argv): sc = argv[i + 1]
H = int(hashlib.sha256(((sc or "") + msg).encode()).hexdigest(), 16)
def fill(s, seed):
    if "enum" in s:
        e = s["enum"]; return e[seed % len(e)]
    t = s.get("type")
    if t == "string": return "t"
    if t == "boolean": return seed % 2 == 0
    if t in ("integer", "number"): return s.get("minimum", 0)
    if t == "array":
        p = list(s.get("items", {}).get("enum", [0]))
        lo, hi = s.get("minItems"), s.get("maxItems")
        if lo is None and hi is None: k = len(p)
        else:
            lo = lo or 0; hi = min(hi if hi is not None else len(p), len(p))
            k = lo + seed % (max(hi - lo, 0) + 1)
        return sorted(p, key=lambda v: hashlib.sha256((str(v) + str(seed)).encode()).hexdigest())[:k]
    if t == "object":
        pr = s.get("properties", {}); rq = s.get("required", list(pr))
        return {k: fill(pr[k], int(hashlib.sha256((k + str(seed)).encode()).hexdigest(), 16))
                for k in sorted(pr) if k in rq}
    return "t"
sch = json.loads(sc) if sc else {}
if "maindeck" in sch.get("properties", {}):
    n = sorted(sch["properties"]["maindeck"].get("properties", {}))
    o = {"maindeck": {c: 1 for c in n[:23]}, "lands": {"Island": 9, "Swamp": 8}}
elif "pick" in sch.get("properties", {}):
    o = {"thoughts": "t", "pick": 0}
else:
    o = fill(sch, H)
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": sid, "result": json.dumps(o), "structured_output": o,
                  "usage": {"input_tokens": 10, "output_tokens": 2,
                            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"##;

pub fn python3_available() -> bool {
    Command::new("python3").arg("--version")
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status().is_ok_and(|s| s.success())
}

/// A fresh scratch directory for one test, with the stub in it.
pub fn scratch(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("mtg-lobby-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("seat.py");
    std::fs::write(&bin, STUB).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (dir, bin)
}

pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// The server subprocess: killed when dropped.
pub struct Server {
    pub child: Child,
    pub port: u16,
    pub keys: Vec<(usize, String)>,
    pub log: PathBuf,
    pub dir: PathBuf,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Start a server with `extra` flags past `--seats`, `--port`,
    /// `--log`, `--seed` and `--best-of 1`; wait for its join lines.
    pub fn start(name: &str, seats: &str, extra: &[&str]) -> Self {
        let (dir, bin) = scratch(name);
        let port = free_port();
        let game_lo = free_port();
        let log = dir.join("draft.log");
        let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-draft-server"))
            .args(["--seats", seats, "--port", &port.to_string(), "--best-of", "1", "--seed", "11"])
            .args(["--game-ports", &format!("{game_lo}-{}", game_lo.saturating_add(40))])
            .args(["--log", log.to_str().unwrap()])
            .args(extra)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .env("CLAUDE_CODE_BIN", &bin)
            .env("MTG_GAME_RETRY_BUDGET_SECS", "5")
            .env("MTG_DRAFT_RETRY_BUDGET_SECS", "5")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the server starts");
        let stderr = Arc::new(Mutex::new(String::new()));
        let pipe = child.stderr.take().unwrap();
        let sink = Arc::clone(&stderr);
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                sink.lock().unwrap().push_str(&line);
                sink.lock().unwrap().push('\n');
            }
        });
        let mut server = Self { child, port, keys: Vec::new(), log, dir, stderr };
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let text = server.stderr();
            if text.contains("the draft starts when") || text.contains("no human seats") {
                server.keys = text.lines()
                    .filter(|l| l.starts_with("seat ") && l.contains("key="))
                    .filter_map(|l| {
                        let seat: usize = l.split_whitespace().nth(1)?.parse().ok()?;
                        let key = l.split("key=").nth(1)?.split([' ', ')']).next()?.to_string();
                        Some((seat, key))
                    })
                    .collect();
                return server;
            }
            if text.contains("Error:") {
                panic!("the server refused to start:\n{text}");
            }
            assert!(Instant::now() < deadline, "the server never printed its join lines:\n{text}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn stderr(&self) -> String {
        self.stderr.lock().unwrap().clone()
    }

    pub fn key(&self, seat: usize) -> &str {
        &self.keys.iter().find(|(s, _)| *s == seat).expect("a key for the seat").1
    }

    pub fn ws_url(&self, seat: usize, key: &str) -> String {
        format!("ws://127.0.0.1:{}/ws?seat={seat}&key={key}", self.port)
    }

    pub fn client(&self, seat: usize) -> WsClient {
        WsClient::connect(&self.ws_url(seat, self.key(seat)))
    }

    /// A line on the host's keyboard.
    pub fn type_line(&mut self, line: &str) {
        let stdin = self.child.stdin.as_mut().expect("the server's stdin");
        writeln!(stdin, "{line}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn log_text(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// Wait until the log holds `needle`.
    pub fn wait_for_log(&self, needle: &str, timeout: Duration) -> String {
        let deadline = Instant::now() + timeout;
        loop {
            let text = self.log_text();
            if text.contains(needle) {
                return text;
            }
            assert!(Instant::now() < deadline,
                "the log never said {needle:?}\nstderr:\n{}\nlog tail:\n{}",
                self.stderr(), text.chars().rev().take(3000).collect::<String>().chars().rev().collect::<String>());
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn http_get(&self, path: &str) -> (u16, String) {
        use std::io::Read;
        let mut s = std::net::TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
        let mut text = String::new();
        s.read_to_string(&mut text).unwrap();
        let status: u16 = text.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One seat's WebSocket.
pub struct WsClient {
    ws: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    pub last: Option<Value>,
}

impl WsClient {
    pub fn connect(url: &str) -> Self {
        let (ws, _) = tungstenite::connect(url).expect("the socket opens");
        if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() {
            s.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        }
        Self { ws, last: None }
    }

    pub fn send(&mut self, msg: Value) {
        self.ws.send(tungstenite::Message::Text(msg.to_string().into())).expect("the message sends");
    }

    /// The next message of any kind, or `None` after `timeout`.
    pub fn next_message(&mut self, timeout: Duration) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.ws.read() {
                Ok(tungstenite::Message::Text(t)) => {
                    let v: Value = serde_json::from_str(&t).expect("json");
                    if v["type"] == "view" {
                        self.last = Some(v.clone());
                    }
                    return Some(v);
                }
                Ok(tungstenite::Message::Close(_)) => return None,
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(_) => return None,
            }
            if Instant::now() >= deadline {
                return None;
            }
        }
    }

    /// The next view, skipping other messages.
    pub fn next_view(&mut self, timeout: Duration) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match self.next_message(deadline.saturating_duration_since(Instant::now())) {
                Some(v) if v["type"] == "view" => return Some(v),
                Some(_) => {}
                None => return None,
            }
        }
        None
    }

    /// Views until one satisfies `pred`; panics after `timeout`.
    pub fn view_until(&mut self, what: &str, timeout: Duration, mut pred: impl FnMut(&Value) -> bool) -> Value {
        let deadline = Instant::now() + timeout;
        if let Some(v) = &self.last {
            if pred(v) {
                return v.clone();
            }
        }
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "never saw a view where {what}; last view:\n{}",
                self.last.as_ref().map_or("none".to_string(), |v| serde_json::to_string_pretty(v).unwrap()));
            if let Some(v) = self.next_view(left) {
                if pred(&v) {
                    return v;
                }
            }
        }
    }

    /// The next `refused` message, or a panic after `timeout`.
    pub fn next_refusal(&mut self, timeout: Duration) -> Value {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "no refusal came");
            if let Some(v) = self.next_message(left) {
                if v["type"] == "refused" {
                    return v;
                }
            }
        }
    }
}

/// Every string anywhere in a JSON value.
pub fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// The card names a view shows this seat: its pack, its pool, its picks
/// and its deck — the only places a card name belongs.
pub fn own_card_names(view: &Value) -> Vec<String> {
    let mut names = Vec::new();
    for c in view["pack"]["cards"].as_array().into_iter().flatten() {
        names.push(c["name"].as_str().unwrap().to_string());
    }
    for c in view["pool"].as_array().into_iter().flatten() {
        names.push(c["name"].as_str().unwrap().to_string());
    }
    for p in view["picks"].as_array().into_iter().flatten() {
        names.push(p["card"].as_str().unwrap().to_string());
    }
    for k in ["main", "sideboard"] {
        for c in view["deck"][k].as_array().into_iter().flatten() {
            names.push(c.as_str().unwrap().to_string());
        }
    }
    names
}

/// Assert a view carries no card name outside the viewing seat's own
/// lists: with `all_cards` the names of every card in the draft, every
/// one that appears must be the seat's.
pub fn assert_no_leak(view: &Value, all_cards: &[String]) {
    let own = own_card_names(view);
    let mut seen = Vec::new();
    for key in ["seats", "matches", "pairings", "standings", "notice"] {
        strings(&view[key], &mut seen);
    }
    for s in seen {
        assert!(!all_cards.contains(&s) || own.contains(&s),
            "the view for seat {} names {s:?} outside its own pack/pool/deck", view["seat"]);
    }
    // And the pack is this seat's: nothing else in the view is a pack.
    assert!(view.get("packs").is_none() && view.get("pools").is_none());
}

pub fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}
