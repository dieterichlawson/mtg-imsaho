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
