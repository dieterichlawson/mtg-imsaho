//! A kick reaches a match already under way (#821).
//!
//! `kick` said "its games are forfeit", but the match was only checked for
//! a seat the table plays for before it started: a running one went on
//! waiting on the kicked person's game page for ever, so the round never
//! ended and the next was never paired.
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::Server;

#[test]
fn a_kick_mid_match_forfeits_the_match_and_the_table_finishes() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("kick-mid-match", "human,ai", &["--pick-seconds", "1", "--build-seconds", "2"]);
    // Seat 0 joins and does nothing: the timers pick and build for it, and
    // its match starts with nobody at its game page.
    let mut c0 = server.client(0);
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        if let Some(v) = c0.next_view(Duration::from_millis(200)) {
            if v["matches"].as_array().is_some_and(|m| m.iter().any(|m| m["status"] == "playing")) {
                break;
            }
        }
        assert!(Instant::now() < deadline, "the match never started:\n{}", server.stderr());
    }
    // The game is waiting on seat 0's page.
    std::thread::sleep(Duration::from_secs(1));
    assert!(!server.log_text().contains("MATCH Round 1"), "the match is under way");
    server.type_line("kick 0");
    let log = server.wait_for_log("FINAL STANDINGS", Duration::from_secs(60));
    assert!(log.contains("MATCH Round 1 — Seat 0 vs Seat 1: 0-1 (Seat 1 wins)"),
        "the kicked seat forfeits its match:\n{log}");
    let terminal = server.stderr();
    assert!(terminal.contains("withdrawn from the game (the host kicked seat 0)"), "{terminal}");
    // Where the game waits is said with the table's name for the seat, not
    // the game's `Seat0` (#850).
    assert!(terminal.contains("seat 0: no browser at http://") && !terminal.contains("Seat0:"), "{terminal}");
    // Named as a kick everywhere the forfeit is told, not as a stall.
    assert!(terminal.contains("seat 0 forfeited (the host kicked seat 0)"), "{terminal}");
    assert!(log.contains("Seat 0 was withdrawn: the host kicked seat 0."), "the result says why:\n{log}");
    assert!(log.contains("WITHDRAWN (Seat 0 vs Seat 1)") && !log.contains("STALLED"), "{log}");
    assert!(terminal.contains("[1 game forfeited: the host kicked seat 0]") && !terminal.contains("a seat stalled"),
        "{terminal}");
    let v = c0.view_until("the view after the tournament", Duration::from_secs(30), |v| v["phase"] == "done");
    assert_eq!(v["matches"][0]["games"][0]["winner"], 1, "{}", v["matches"]);
    assert!(v["matches"][0]["url"].is_null(), "a kicked seat is not offered its game: {}", v["matches"]);
}

/// The seat whose game page holds the decision, read off both pages.
fn holder(pages: &mut [(usize, lobby_support::WsClient)]) -> Option<usize> {
    for (seat, page) in pages.iter_mut() {
        while let Some(msg) = page.next_message(Duration::from_millis(300)) {
            if msg["type"] == "decision" {
                return Some(*seat);
            }
        }
    }
    None
}

/// #854: a kick while the OTHER seat held the decision did nothing: the
/// kicked seat was not asked again until its opponent answered, so the
/// match stayed "playing", and a concede by the opponent meanwhile was
/// recorded as a game won by the seat the host had removed.
#[test]
fn a_kick_while_the_opponent_holds_the_decision_forfeits_the_game_now() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("kick-other-decides", "human,human",
        &["--pick-seconds", "1", "--build-seconds", "2", "--best-of", "3"]);
    let mut clients = [server.client(0), server.client(1)];
    let deadline = Instant::now() + Duration::from_secs(240);
    let mut urls = [None, None];
    while urls.iter().any(Option::is_none) {
        for (seat, c) in clients.iter_mut().enumerate() {
            if let Some(v) = c.next_view(Duration::from_millis(200)) {
                if let Some(url) = v["matches"].as_array().into_iter().flatten()
                    .filter(|m| m["status"] == "playing").find_map(|m| m["url"].as_str()) {
                    urls[seat] = Some(url.to_string());
                }
            }
        }
        assert!(Instant::now() < deadline, "the match never started:\n{}", server.stderr());
    }
    let mut pages: Vec<(usize, lobby_support::WsClient)> = urls.iter().enumerate().map(|(seat, url)| {
        let ws = url.as_deref().unwrap().replacen("http://", "ws://", 1) + "ws";
        let mut page = lobby_support::WsClient::connect(&ws);
        page.send(serde_json::json!({"type": "hello"}));
        (seat, page)
    }).collect();
    let deadline = Instant::now() + Duration::from_secs(30);
    let held = loop {
        if let Some(seat) = holder(&mut pages) {
            break seat;
        }
        assert!(Instant::now() < deadline, "no page was asked anything:\n{}", server.stderr());
    };
    let kicked = 1 - held;
    // Nobody answers at `held`'s page: before the fix the game waited on it.
    server.type_line(&format!("kick {kicked}"));
    let log = server.wait_for_log("FINAL STANDINGS", Duration::from_secs(60));
    let line = if kicked == 0 { "MATCH Round 1 — Seat 0 vs Seat 1: 0-2 (Seat 1 wins)" }
        else { "MATCH Round 1 — Seat 0 vs Seat 1: 2-0 (Seat 0 wins)" };
    assert!(log.contains(line), "the kicked seat forfeits every game, the one in progress too:\n{log}");
    assert!(log.contains(&format!("Seat {kicked} was withdrawn: the host kicked seat {kicked}.")), "{log}");
    let terminal = server.stderr();
    assert!(terminal.contains(&format!("[2 games forfeited: the host kicked seat {kicked}]")), "{terminal}");
    let v = clients[held].view_until("the view after the tournament", Duration::from_secs(30), |v| v["phase"] == "done");
    assert_eq!(v["matches"][0]["games"][0]["winner"], held, "{}", v["matches"]);
}
