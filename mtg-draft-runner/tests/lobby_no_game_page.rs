//! A seat whose game page cannot take a port forfeits its match, and the
//! record says so (#759).
//!
//! The table announced "the match is forfeit" and then recorded a 0-0 draw,
//! a point each: `forfeit_result` only forfeits a seat the table plays for,
//! and a joined human is not one. The reason went to the terminal only.
#![cfg(unix)]

mod lobby_support;

use std::net::TcpListener;
use std::time::Duration;

use lobby_support::Server;

#[test]
fn a_seat_without_a_game_page_forfeits_and_the_log_says_why() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    // Hold the only game port.
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let game = held.local_addr().unwrap().port().to_string();
    let range = format!("{game}-{game}");
    let server = Server::start("no-game-page", "human,ai",
        &["--pick-seconds", "1", "--build-seconds", "2", "--game-ports", &range]);
    // Seat 0 joins and does nothing: the timers pick and build for it.
    let mut c0 = server.client(0);
    let deadline = std::time::Instant::now() + Duration::from_secs(240);
    while !server.log_text().contains("MATCH Round 1") {
        let _ = c0.next_message(Duration::from_millis(200));
        assert!(std::time::Instant::now() < deadline, "the match was never recorded:\n{}", server.stderr());
    }
    let log = server.wait_for_log("FINAL STANDINGS", Duration::from_secs(30));
    assert!(log.contains("MATCH Round 1 — Seat 0 vs Seat 1: 0-1 (Seat 1 wins)"),
        "the seat with no page forfeits to its opponent, not a draw:\n{log}");
    assert!(log.contains("no game page for seat 0"), "the log says why:\n{log}");
    let terminal = server.stderr();
    assert!(terminal.contains("seat 0 forfeits the match"), "{terminal}");
    assert!(!terminal.contains("(drawn)"), "the terminal does not call it drawn:\n{terminal}");
    assert!(terminal.contains("[1 game forfeited: no game page for seat 0"),
        "the score line gives the reason, not \"a seat stalled\":\n{terminal}");
    drop(held);
}
