//! A human seat nobody is sitting at: the host starts without it and the
//! table picks for it under `--pick-seconds`, then stops the moment the
//! person joins; `kick` hands a seat to the table for good, and its games
//! are forfeit rather than waited on.
#![cfg(unix)]

mod lobby_support;

use std::time::Duration;

use lobby_support::Server;
use serde_json::json;

const T: Duration = Duration::from_secs(30);

#[test]
fn an_absent_human_is_picked_for_until_it_joins() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let mut server = Server::start("absent", "human,ai", &["--pick-seconds", "1"]);
    // Nobody joins; the host presses Enter.
    server.type_line("");
    let log = server.wait_for_log("[Seat 0] PICK Pack 1 Pick 2 |", Duration::from_secs(20));
    assert!(log.contains("[Seat 0] PROMPT Pack 1 Pick 1\tauto-pick: the seat is away"),
        "the auto-pick is recorded as the table's, not the seat's:\n{log}");
    let terminal = server.stderr();
    assert!(terminal.contains("seat 0 has not joined; the table picks for it until it does"), "{terminal}");

    // The person arrives: the next pack is theirs, with the timer running.
    let mut c0 = server.client(0);
    let v = c0.view_until("the seat is handed back", T, |v| v["seats"][0]["auto"] == false);
    assert!(v["picks"].as_array().unwrap().iter().all(|p| p["auto"] == true), "every pick so far was the table's");
    assert!(v["picks"].as_array().unwrap().len() >= 2);
    assert_eq!(v["pick_seconds"], 1);
    let v = c0.view_until("a pack with a deadline", T, |v| !v["pack"].is_null() && !v["pack"]["deadline_ms"].is_null());
    assert!(v["pack"]["deadline_ms"].as_u64().unwrap() <= 1000);
    // Do nothing: the timer picks, and the seat is told.
    let v = c0.view_until("the timer's pick is noticed", T, |v| v["notice"].as_str().is_some_and(|n| n.contains("timer ran out")));
    assert!(v["picks"].as_array().unwrap().last().unwrap()["auto"] == true);
    let picks_before = v["picks"].as_array().unwrap().len();
    // Then pick in time, which is the seat's own.
    let v = c0.view_until("a pack to pick from", T, |v| !v["pack"].is_null());
    let id = v["pack"]["id"].as_u64().unwrap();
    c0.send(json!({"type": "pick", "pack_id": id, "index": 1}));
    let v = c0.view_until("the own pick lands", T, |v| v["picks"].as_array().unwrap().len() > picks_before);
    assert_eq!(v["picks"].as_array().unwrap().last().unwrap()["auto"], false);

    // A pick for a pack that is no longer in front is refused, not
    // applied to the next one.
    c0.send(json!({"type": "pick", "pack_id": id, "index": 0}));
    let refusal = c0.next_refusal(T);
    let reason = refusal["reason"].as_str().unwrap();
    assert!(reason.contains("not the pack in front of you") || reason.contains("no pack in front of you"), "{refusal}");
    assert_eq!(refusal["echo"]["pack_id"], id);

    // Kicked: the table finishes the draft and builds, and the seat is told
    // its games are forfeit — the match against the stub is 0-1 unplayed.
    server.type_line("kick 0");
    let v = c0.view_until("kicked", T, |v| v["seats"][0]["auto"] == true);
    assert!(v["notice"].as_str().is_some_and(|n| n.contains("handed your seat")), "{}", v["notice"]);
    let log = server.wait_for_log("FINAL STANDINGS", Duration::from_secs(90));
    assert!(log.contains("[Seat 0] WARN the table built this seat's deck"), "{log}");
    assert!(log.contains("MATCH Round 1 — Seat 0 vs Seat 1: 0-1 (Seat 1 wins)"), "the kicked seat forfeits:\n{log}");
    assert!(log.contains("[1 game forfeited]"), "the standings say the loss was a forfeit:\n{log}");
    let v = c0.view_until("done", T, |v| v["phase"] == "done");
    assert_eq!(v["standings"][0]["seat"], 1);
    assert_eq!(v["matches"][0]["result"], "0-1");
    assert!(v["matches"][0]["url"].is_null(), "no page was opened for a forfeit");
    // Kicking again, or kicking the AI, is refused on the terminal.
    server.type_line("kick 1");
    server.type_line("kick 9");
    std::thread::sleep(Duration::from_millis(300));
    let terminal = server.stderr();
    assert!(terminal.contains("kick: seat 1 is an ai seat"), "{terminal}");
    assert!(terminal.contains("kick: there is no seat 9"), "{terminal}");
    assert!(terminal.contains("the tournament is over. Final standings:"), "{terminal}");
}
