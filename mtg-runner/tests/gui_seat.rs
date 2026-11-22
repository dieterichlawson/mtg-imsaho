//! The gui seat, driven through its WebSocket the way the page drives it.
//!
//! The page's `s` ("stop at every priority") decides whether the page
//! answers a pass-only priority for the player. That only works if the
//! seat is *asked* about those priorities; the engine used to pass them
//! itself unless `--check-invariants` was given, so in the README's own
//! command lines `s` was a no-op and an opponent's spell the player could
//! not answer never appeared as a decision (#645).

use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tungstenite::{Message, WebSocket};

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The page's URL as the runner prints it, from its stderr. The seat is on
/// `gui:0`: a port found free and then handed to the runner could be taken
/// in between, and the test would drive whatever took it (#941's shape).
fn page_url(stderr: std::process::ChildStderr) -> String {
    use std::io::BufRead;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(url) = line.strip_prefix("--p1 gui: open ") {
                let _ = tx.send(url.trim().to_string());
            }
        }
    });
    rx.recv_timeout(Duration::from_secs(60)).expect("the runner named its page")
}

fn connect(url: &str) -> WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>> {
    let ws_url = format!("{}ws", url.replacen("http://", "ws://", 1));
    match tungstenite::connect(&ws_url) {
        Ok((ws, _)) => ws,
        Err(e) => panic!("the gui seat did not accept a connection at {ws_url}: {e}"),
    }
}

fn is_pass_only(actions: &[Value]) -> bool {
    !actions.is_empty() && actions.iter().all(|a| a == "PassPriority" || a == "Concede")
}

#[test]
fn a_gui_seat_is_asked_at_priorities_where_it_can_only_pass() {
    let dir = std::env::temp_dir().join(format!("mtg-gui-seat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Nothing to cast, so every priority after the land drop is pass-only.
    let deck = dir.join("deck.txt");
    std::fs::write(&deck, "60 Mountain\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "gui:0", "--p2", "random", "--seed", "11", "-q"])
        .args(["--deck1", deck.to_str().unwrap(), "--deck2", deck.to_str().unwrap()])
        .env("MTG_GUI_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mtg-runner");
    let stderr = child.stderr.take().expect("the runner's stderr");
    let _child = KillOnDrop(child);

    let mut ws = connect(&page_url(stderr));
    ws.send(Message::Text(json!({"type": "hello"}).to_string().into())).unwrap();

    let mut decisions = 0;
    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while decisions < 40 && Instant::now() < deadline {
        let Message::Text(text) = ws.read().expect("the seat stays connected") else { continue };
        let msg: Value = serde_json::from_str(&text).unwrap();
        if msg["type"] == "game_over" {
            break;
        }
        if msg["type"] != "decision" {
            continue;
        }
        decisions += 1;
        let actions = msg["legal"]["actions"].as_array().cloned().unwrap_or_default();
        if is_pass_only(&actions) {
            // The test's point: the seat was asked. Done.
            return;
        }
        seen.push(actions.iter().map(|a| match a {
            Value::String(s) => s.clone(),
            Value::Object(o) => o.keys().next().cloned().unwrap_or_default(),
            _ => a.to_string(),
        }).collect::<Vec<_>>().join("/"));
        let answer = actions.iter().find(|a| *a == "MulliganKeep")
            .or_else(|| actions.iter().find(|a| a.get("PlayLand").is_some()))
            .or_else(|| actions.iter().find(|a| *a == "PassPriority"))
            .cloned()
            .unwrap_or_else(|| panic!("a decision this test does not answer, before any \
                pass-only priority reached the seat: {}", msg["legal"]));
        ws.send(Message::Text(json!({"type": "action", "seq": msg["seq"], "action": answer})
            .to_string().into())).unwrap();
    }
    panic!("{decisions} decisions reached the gui seat and not one was a pass-only priority; \
        the engine passed them before the seat was asked, so the page's `s` has nothing to \
        stop at. Decisions seen: {seen:?}");
}

/// #1033: the runner sent `game_over` and exited, and the page, its socket
/// reset, read the exit as a dropped network and retried once a second for
/// ever. After the result it is now told nothing more is coming.
#[test]
fn a_gui_seats_page_is_told_the_runner_has_stopped_after_the_result() {
    let dir = std::env::temp_dir().join(format!("mtg-gui-seat-closed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = dir.join("deck.txt");
    std::fs::write(&deck, "60 Mountain\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "gui:0", "--p2", "random", "--seed", "11", "-q"])
        .args(["--deck1", deck.to_str().unwrap(), "--deck2", deck.to_str().unwrap()])
        .env("MTG_GUI_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mtg-runner");
    let stderr = child.stderr.take().expect("the runner's stderr");
    let _child = KillOnDrop(child);
    let mut ws = connect(&page_url(stderr));
    ws.send(Message::Text(json!({"type": "hello"}).to_string().into())).unwrap();
    let mut kinds = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let Ok(Message::Text(text)) = ws.read() else { break };
        let msg: Value = serde_json::from_str(&text).unwrap();
        let kind = msg["type"].as_str().unwrap_or_default().to_string();
        if kind == "decision" {
            ws.send(Message::Text(json!({"type": "action", "seq": msg["seq"], "action": "Concede"})
                .to_string().into())).unwrap();
        }
        if kind == "closed" {
            assert_eq!(msg["reason"], "the runner serving this page has stopped");
            assert_eq!(msg["over"], "game", "the page says the game is over, not a table: {msg}");
            assert!(kinds.iter().any(|k| k == "game_over"), "the result came first: {kinds:?}");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        kinds.push(kind);
    }
    panic!("the page was never told the runner stopped; it got {kinds:?} and then its socket dropped");
}

/// A runner on `--p1 gui:0 --p2 random`, its stderr's page URL, and the
/// guard that kills it.
fn gui_runner(tag: &str, extra: &[&str]) -> (KillOnDrop, String, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("mtg-gui-seat-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = dir.join("deck.txt");
    std::fs::write(&deck, "60 Mountain\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "gui:0", "--p2", "random", "--seed", "11", "-q"])
        .args(["--deck1", deck.to_str().unwrap(), "--deck2", deck.to_str().unwrap()])
        .args(extra)
        .env("MTG_GUI_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mtg-runner");
    let stderr = child.stderr.take().expect("the runner's stderr");
    let url = page_url(stderr);
    (KillOnDrop(child), url, dir)
}

/// Read until `closed`, answering each decision with `answer` (once
/// `on_decision` returns true it stops answering), and return the frame.
fn read_until_closed(
    ws: &mut WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    mut on_decision: impl FnMut(&Value) -> Option<Value>,
) -> Value {
    let mut kinds = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let Ok(Message::Text(text)) = ws.read() else { break };
        let msg: Value = serde_json::from_str(&text).unwrap();
        let kind = msg["type"].as_str().unwrap_or_default().to_string();
        if kind == "closed" {
            return msg;
        }
        if kind == "decision" {
            if let Some(answer) = on_decision(&msg) {
                ws.send(Message::Text(json!({"type": "action", "seq": msg["seq"], "action": answer})
                    .to_string().into())).unwrap();
            }
        }
        kinds.push(kind);
    }
    panic!("the page was never told the runner stopped; it got {kinds:?} and then its socket dropped");
}

/// #1033: `--stop-after` exited mid-game past the gui seat's page, which
/// read the reset socket as a dropped network and retried for ever. It is
/// told the runner stopped, and that the game is not over.
#[test]
fn a_gui_seats_page_is_told_when_stop_after_ends_the_run() {
    // Far enough in that the seat, which waits on its page, has been asked.
    let (_child, url, dir) = gui_runner("stop-after", &["--stop-after", "8"]);
    let mut ws = connect(&url);
    ws.send(Message::Text(json!({"type": "hello"}).to_string().into())).unwrap();
    let msg = read_until_closed(&mut ws, |msg| {
        let actions = msg["legal"]["actions"].as_array().cloned().unwrap_or_default();
        actions.iter().find(|a| *a == "MulliganKeep")
            .or_else(|| actions.iter().find(|a| a.get("PlayLand").is_some()))
            .or_else(|| actions.iter().find(|a| *a == "PassPriority"))
            .cloned()
    });
    assert_eq!(msg["over"], "stopped", "the page says the game stopped, not that it is over: {msg}");
    assert_eq!(msg["reason"], "the runner serving this page stopped: --stop-after 8", "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1033: with only gui and random seats no stop handler was installed, so
/// SIGTERM took the runner down by its default action and the page was
/// told nothing.
#[test]
fn a_gui_seats_page_is_told_when_a_stop_signal_ends_the_run() {
    let (child, url, dir) = gui_runner("sigterm", &[]);
    let mut ws = connect(&url);
    ws.send(Message::Text(json!({"type": "hello"}).to_string().into())).unwrap();
    let pid = child.0.id().to_string();
    let mut sent = false;
    let msg = read_until_closed(&mut ws, |_| {
        if !sent {
            sent = true;
            let status = Command::new("kill").args(["-TERM", &pid]).status().expect("run kill");
            assert!(status.success(), "kill -TERM {pid}");
        }
        None
    });
    assert_eq!(msg["over"], "stopped", "{msg}");
    assert_eq!(msg["reason"], "the runner serving this page stopped: stopped by SIGTERM", "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}
