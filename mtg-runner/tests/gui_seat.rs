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

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn connect(port: u16) -> WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match tungstenite::connect(format!("ws://127.0.0.1:{port}/ws")) {
            Ok((ws, _)) => return ws,
            Err(e) if Instant::now() < deadline => {
                let _ = e;
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => panic!("the gui seat never accepted a connection on {port}: {e}"),
        }
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

    let port = free_port();
    let child = Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", &format!("gui:{port}"), "--p2", "random", "--seed", "11", "-q"])
        .args(["--deck1", deck.to_str().unwrap(), "--deck2", deck.to_str().unwrap()])
        .env("MTG_GUI_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mtg-runner");
    let _child = KillOnDrop(child);

    let mut ws = connect(port);
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
