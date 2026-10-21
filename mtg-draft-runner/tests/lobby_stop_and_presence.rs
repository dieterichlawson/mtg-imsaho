//! How a hosted table ends, and who it says is where.
//!
//! - #822: closing a game page mid-match left the opponent's page saying
//!   "Opponent is thinking" for ever and both draft pages saying "here".
//! - #823: Ctrl-C ended the server with no word on the terminal or in the
//!   log, where `quit` writes why the table ended.
//! - #824: `quit` reset every socket instead of closing it, so each page
//!   read a deliberate stop as a dropped network and retried for ever.
//! - #825: a kick was never written to the log, and the forfeits it
//!   caused said "the seat is away".
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::{Server, WsClient};
use serde_json::Value;

/// The seat's game page socket, from the match link its draft view holds.
fn game_page(url: &str) -> WsClient {
    let ws = url.replacen("http://", "ws://", 1).trim_end_matches('/').to_string() + "/ws";
    let mut page = WsClient::connect(&ws);
    page.send(serde_json::json!({"type": "hello"}));
    page
}

/// The first message of `kind` a socket sends within `timeout` that `pred`
/// accepts.
fn message_until(ws: &mut WsClient, kind: &str, timeout: Duration, pred: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "no {kind} message came");
        if let Some(m) = ws.next_message(left) {
            if m["type"] == kind && pred(&m) {
                return m;
            }
        }
    }
}

fn presence_of(view: &Value, seat: usize) -> String {
    view["seats"][seat]["presence"].as_str().unwrap_or("").to_string()
}

#[test]
fn a_closed_game_page_is_said_on_both_sides_and_quit_closes_every_page() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("stop-presence", "human,human", &["--pick-seconds", "1", "--build-seconds", "2"]);
    let mut c0 = server.client(0);
    let mut c1 = server.client(1);
    let url = |v: &Value| v["matches"].as_array()
        .and_then(|m| m.iter().find(|m| m["status"] == "playing"))
        .and_then(|m| m["url"].as_str()).map(str::to_string);
    let v0 = c0.view_until("seat 0's match is on", Duration::from_secs(240), |v| url(v).is_some());
    let v1 = c1.view_until("seat 1's match is on", Duration::from_secs(30), |v| url(v).is_some());
    let mut g0 = game_page(&url(&v0).unwrap());
    let mut g1 = game_page(&url(&v1).unwrap());
    // Each page is told which game it is on (#855).
    let table = message_until(&mut g1, "table", Duration::from_secs(10), |m| m["match_line"].is_string());
    assert_eq!(table["match_line"], "Game 1 of a best-of-1", "{table}");

    c1.view_until("seat 0 in game", Duration::from_secs(10), |v| presence_of(v, 0) == "in game");
    drop(g0);
    let v = c1.view_until("seat 0's game page closed", Duration::from_secs(10), |v| presence_of(v, 0) == "game page closed");
    assert_eq!(presence_of(&v, 1), "in game", "{}", v["seats"]);
    let told = message_until(&mut g1, "table", Duration::from_secs(10), |m| m["opponent"].is_string());
    assert!(told["opponent"].as_str().unwrap().contains("seat 0's game page is closed"), "{told}");
    server.wait_for_stderr("seat 0's game page is closed; the game waits for them to open it again", Duration::from_secs(5));

    // Back: said on both sides again.
    g0 = game_page(&url(&v0).unwrap());
    c1.view_until("seat 0 in game again", Duration::from_secs(10), |v| presence_of(v, 0) == "in game");
    message_until(&mut g1, "table", Duration::from_secs(10), |m| m["opponent"].is_null());

    // The host quits: every page and client is told why, and closed.
    server.type_line("quit");
    let closed = message_until(&mut c0, "closed", Duration::from_secs(10), |_| true);
    assert_eq!(closed["reason"], "the host quit");
    message_until(&mut c1, "closed", Duration::from_secs(10), |_| true);
    message_until(&mut g0, "closed", Duration::from_secs(10), |m| m["reason"] == "the host quit");
    message_until(&mut g1, "closed", Duration::from_secs(10), |_| true);
    let status = server.child.wait().expect("the server exits");
    assert!(status.success(), "{status}");
    let log = server.log_text();
    assert!(log.contains("NOTE the host quit; the table was playing and is not finished"), "{log}");
}

#[test]
fn a_ctrl_c_ends_the_table_the_way_quit_does() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("stop-sigint", "human,ai", &["--pick-seconds", "30"]);
    let mut c0 = server.client(0);
    c0.view_until("the draft has begun", Duration::from_secs(30), |v| v["phase"] == "drafting");
    let pid = server.child.id().to_string();
    let killed = std::process::Command::new("kill").args(["-INT", &pid]).status().expect("kill runs");
    assert!(killed.success());
    let closed = message_until(&mut c0, "closed", Duration::from_secs(10), |_| true);
    assert_eq!(closed["reason"], "the server was stopped by Ctrl-C (SIGINT)", "{closed}");
    let status = server.child.wait().expect("the server exits");
    assert_eq!(status.code(), Some(130), "the exit status of a Ctrl-C: {status}");
    // Said on the terminal too (the usage summary follows when the seats
    // had spent anything; this one's only call was the one taken down).
    server.wait_for_stderr("the server was stopped by Ctrl-C (SIGINT); the table is closed", Duration::from_secs(5));
    let log = server.log_text();
    assert!(log.contains("NOTE the server was stopped by Ctrl-C (SIGINT); the table was drafting and is not finished"),
        "{log}");
}

#[test]
fn a_kick_is_on_the_record_and_its_forfeits_say_so() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("kick-record", "human,ai", &["--pick-seconds", "30"]);
    let mut c0 = server.client(0);
    c0.view_until("the draft has begun", Duration::from_secs(30), |v| v["phase"] == "drafting");
    server.type_line("kick 0");
    let log = server.wait_for_log("FINAL STANDINGS", Duration::from_secs(240));
    assert!(log.contains("[Seat 0] KICKED by the host while the table was drafting"), "{log}");
    assert!(log.contains("forfeit: the host kicked the seat") && !log.contains("the seat is away"),
        "the forfeits give the kick as the reason:\n{log}");
    let v = c0.view_until("the table is done", Duration::from_secs(30), |v| v["phase"] == "done");
    assert_eq!(presence_of(&v, 0), "kicked", "{}", v["seats"]);
}

/// The server writes a snapshot beside its log after every change, and
/// `--resume` carries the table on from it: the same join link opens the
/// same seat, with the picks it had made, and the table finishes. A server
/// that died used to take the draft with it (`--resume` was accepted and
/// ignored).
#[test]
fn a_stopped_table_carries_on_from_its_snapshot() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut first = Server::start("resume-first", "human,ai", &["--pick-seconds", "1", "--build-seconds", "2"]);
    first.wait_for_stderr("snapshot: ", Duration::from_secs(5));
    let key = first.key(0).to_string();
    let mut c0 = first.client(0);
    c0.view_until("seat 0 has picked a few", Duration::from_secs(60),
        |v| v["picks"].as_array().is_some_and(|p| p.len() >= 5));
    let snapshot = first.dir.join("table.save.json");
    let kill = std::process::Command::new("kill").args(["-INT", &first.child.id().to_string()]).status().unwrap();
    assert!(kill.success());
    first.child.wait().unwrap();
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&snapshot).expect("a snapshot was written")).unwrap();
    let picks = saved["picks"].as_array().unwrap().iter().filter(|p| p["seat"] == 0).count();
    assert!(picks >= 5, "{saved}");
    let kept = first.dir.with_extension("save.json");
    std::fs::copy(&snapshot, &kept).unwrap();

    let mut second = Server::start("resume-second", "human,ai", &["--resume", kept.to_str().unwrap(),
        "--pick-seconds", "1", "--build-seconds", "2"]);
    let terminal = second.wait_for_stderr("the table carries on: everybody rejoins", Duration::from_secs(10));
    assert!(terminal.contains(&format!("key={key}")), "the same join link:\n{terminal}");
    let mut again = WsClient::connect(&second.ws_url(0, &key));
    let v = again.view_until("the resumed seat's view", Duration::from_secs(10), |v| v["phase"] == "drafting");
    assert!(v["picks"].as_array().unwrap().len() >= picks, "the picks it had made: {}", v["picks"]);
    // It drafts and builds on to the tournament; nobody opens seat 0's
    // game page, so the host hands it to the table to finish.
    second.wait_for_log("TOURNAMENT", Duration::from_secs(240));
    second.type_line("kick 0");
    let log = second.wait_for_log("FINAL STANDINGS", Duration::from_secs(60));
    assert!(log.contains("NOTE the table carries on from the snapshot"), "{log}");
    assert_eq!(log.matches("DRAFT POOLS").count() + log.matches("BOOSTER PACKS").count(), 0,
        "the replay wrote nothing the first log already had:\n{log}");
    let _ = std::fs::remove_file(&kept);
}
