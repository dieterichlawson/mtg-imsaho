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
    // Named as a kick everywhere the forfeit is told, not as a stall.
    assert!(terminal.contains("seat 0 forfeited (the host kicked seat 0)"), "{terminal}");
    assert!(log.contains("WITHDRAWN (Seat 0 vs Seat 1)") && !log.contains("STALLED"), "{log}");
    assert!(terminal.contains("[1 game forfeited: the host kicked seat 0]") && !terminal.contains("a seat stalled"),
        "{terminal}");
    let v = c0.view_until("the view after the tournament", Duration::from_secs(30), |v| v["phase"] == "done");
    assert_eq!(v["matches"][0]["games"][0]["winner"], 1, "{}", v["matches"]);
    assert!(v["matches"][0]["url"].is_null(), "a kicked seat is not offered its game: {}", v["matches"]);
}
