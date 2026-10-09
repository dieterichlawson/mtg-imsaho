//! Two people and two stub AI seats draft a whole table through the
//! server: join, pick to the end, build, ready, and the first round's
//! AI-vs-AI match plays to a result while the humans' match waits for a
//! browser.
//!
//! One run, many assertions, because a draft is the expensive part: a
//! wrong key is refused, every view a seat gets names no other seat's
//! cards, the picks the view shows are the ones the log records, a client
//! that reconnects mid-draft is sent the same pending pack, an invalid
//! deck is refused with its reason, a human-vs-human match is two pages
//! on two ports, and the view API answers with the same view the socket
//! sends.
#![cfg(unix)]

mod lobby_support;

use std::time::Duration;

use lobby_support::{assert_no_leak, Server, WsClient};
use serde_json::{json, Value};

const T: Duration = Duration::from_secs(30);

fn all_set_cards() -> Vec<String> {
    let path = lobby_support::repo_root().join("data/sets/isd.json");
    let data = mtg_draft::set_data::SetData::load(&path).unwrap();
    let mut names: Vec<String> = data.all_card_names().iter()
        .map(|n| mtg_draft::front_face(n).to_string()).collect();
    names.extend(["Plains", "Island", "Swamp", "Mountain", "Forest"].map(String::from));
    names
}

/// Pick the last card of the pack: not what the stub picks (card 0), so a
/// human's picks and an AI's are told apart in the log.
fn pick_last(client: &mut WsClient, view: &Value, picked: &mut Vec<(u64, u64, String)>, last_pack: &mut Option<u64>) {
    let pack = &view["pack"];
    let Some(id) = pack["id"].as_u64() else { return };
    if *last_pack == Some(id) && picked.last().is_some_and(|(r, p, _)| (*r, *p) == (pack["round"].as_u64().unwrap(), pack["pick"].as_u64().unwrap())) {
        return;
    }
    let cards = pack["cards"].as_array().unwrap();
    let index = cards.len() - 1;
    picked.push((pack["round"].as_u64().unwrap(), pack["pick"].as_u64().unwrap(),
        cards[index]["name"].as_str().unwrap().to_string()));
    *last_pack = Some(id);
    client.send(json!({"type": "pick", "pack_id": id, "index": index}));
}

#[test]
fn two_humans_and_two_ai_seats_draft_build_and_the_ai_match_plays() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let all_cards = all_set_cards();
    let server = Server::start("two-humans", "human,human,ai,ai", &[]);
    assert_eq!(server.keys.len(), 2, "one key per human seat:\n{}", server.stderr());

    // A wrong key, and an AI seat, are refused by name.
    let mut wrong = WsClient::connect(&server.ws_url(0, "nope"));
    let refusal = wrong.next_message(T).expect("a refusal");
    assert_eq!(refusal["type"], "refused");
    assert!(refusal["reason"].as_str().unwrap().contains("wrong key for seat 0"), "{refusal}");
    let mut ai = WsClient::connect(&server.ws_url(2, "anything"));
    let refusal = ai.next_message(T).expect("a refusal");
    assert!(refusal["reason"].as_str().unwrap().contains("ai seat"), "{refusal}");

    // The first human sees the lobby; the second one's arrival starts it.
    let mut c0 = server.client(0);
    let lobby = c0.next_view(T).expect("a view on connect");
    assert_eq!(lobby["phase"], "lobby");
    assert_eq!(lobby["seat"], 0);
    assert_eq!(lobby["pod_size"], 4);
    assert_eq!(lobby["seats"][1]["joined"], false);
    assert_eq!(lobby["seats"][2]["kind"], "ai");
    assert!(lobby["pack"].is_null());
    c0.send(json!({"type": "name", "name": "Lawson"}));
    let mut c1 = server.client(1);
    let started = c0.view_until("the draft starts", T, |v| v["phase"] == "drafting");
    assert_eq!(started["pass_direction"], "left");
    assert_eq!(started["pack"]["round"], 1);
    assert_eq!(started["pack"]["pick"], 1);
    assert_eq!(started["pack"]["size"], 14);
    assert_eq!(started["pack"]["cards"].as_array().unwrap().len(), 14);
    assert_eq!(started["pack"]["cards"][0]["index"], 0);
    assert!(started["pack"]["cards"][0]["line"].as_str().unwrap().contains('|'), "{}", started["pack"]["cards"][0]);
    assert_eq!(started["seats"][0]["name"], "Lawson");

    // Draft to the end, both humans, checking every view for leaks.
    let mut picked0: Vec<(u64, u64, String)> = Vec::new();
    let mut picked1: Vec<(u64, u64, String)> = Vec::new();
    let (mut last0, mut last1) = (None, None);
    let mut reconnected = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let done = |c: &WsClient| c.last.as_ref().is_some_and(|v| v["seats"][v["seat"].as_u64().unwrap() as usize]["status"] != "picking" && v["pack"].is_null() && v["pool"].as_array().unwrap().len() == 42);
        if done(&c0) && done(&c1) {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "the draft did not finish:\n{}", server.stderr());
        if let Some(v) = c0.next_view(Duration::from_millis(100)) {
            assert_no_leak(&v, &all_cards);
            // Mid-draft, drop the socket and come back: the same pack is
            // waiting (#reconnect-safe, the game page's rule).
            if !reconnected && picked0.len() == 3 && !v["pack"].is_null() && v["pack"]["pick"] == 4 {
                reconnected = true;
                let pending = v["pack"].clone();
                c0 = server.client(0);
                let again = c0.next_view(T).expect("a view on reconnect");
                assert_eq!(again["pack"]["id"], pending["id"]);
                assert_eq!(again["pack"]["cards"], pending["cards"]);
                assert_eq!(again["picks"].as_array().unwrap().len(), 3);
                assert_eq!(again["seats"][0]["name"], "Lawson", "the name survives a reconnect");
                pick_last(&mut c0, &again, &mut picked0, &mut last0);
                continue;
            }
            pick_last(&mut c0, &v, &mut picked0, &mut last0);
        }
        if let Some(v) = c1.next_view(Duration::from_millis(100)) {
            assert_no_leak(&v, &all_cards);
            pick_last(&mut c1, &v, &mut picked1, &mut last1);
        }
    }
    assert!(reconnected, "the reconnect check ran");
    assert_eq!(picked0.len(), 42);
    assert_eq!(picked1.len(), 42);

    // What the view shows is what was picked, and what the log records.
    let v0 = c0.last.clone().unwrap();
    let shown: Vec<(u64, u64, String)> = v0["picks"].as_array().unwrap().iter()
        .map(|p| (p["round"].as_u64().unwrap(), p["pick"].as_u64().unwrap(), p["card"].as_str().unwrap().to_string()))
        .collect();
    assert_eq!(shown, picked0, "the view's picks are the picks made");
    assert!(v0["picks"].as_array().unwrap().iter().all(|p| p["auto"] == false));
    let log = server.log_text();
    for (round, pick, card) in &picked0 {
        let line = format!("[Seat 0] PICK Pack {round} Pick {pick} | Chose: {card} (");
        assert!(log.contains(&line), "the log records the pick: {line}");
    }
    assert!(log.contains("[Seat 0] PROMPT Pack 1 Pick 1\thuman"), "a human pick logs `human` where the prompt goes");
    assert!(log.contains("[Seat 2] PROMPT Pack 1 Pick 1") && log.contains("Pack 1 of 3, Pick 1 of 14. You are seat 2 of 4"),
        "an AI pick logs its prompt");
    assert!(log.contains("[Seat 0] POOL (42 cards)"));
    assert!(v0["seats"][2]["picks"] == 42 && v0["seats"][3]["picks"] == 42, "{}", v0["seats"]);
    // The AI seats finished first and built at once; their pools and decks
    // are under the DECK BUILDING header, not above it in the DRAFT section
    // (the first playtest found three of four decks there).
    let header = log.find("  DECK BUILDING\n").expect("a DECK BUILDING section");
    for needle in ["] POOL (", "] DECK ("] {
        let first = log.find(needle).unwrap_or_else(|| panic!("the log has a {needle} record"));
        assert!(first > header, "the first {needle:?} record is above the DECK BUILDING header");
    }

    // A short deck is work in progress: recorded with its problem, not
    // refused (the page sends the deck after every card moved).
    let pool: Vec<String> = v0["pool"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect();
    c0.send(json!({"type": "deck", "main": pool[..10], "lands": {}, "sideboard": pool[10..]}));
    let v = c0.view_until("the short deck is shown with its problem", T, |v| v["deck"]["valid"] == false);
    assert!(v["deck"]["problem"].as_str().unwrap().contains("need at least 40"), "{}", v["deck"]);
    assert_eq!(v["deck"]["main"].as_array().unwrap().len(), 10);
    assert_eq!(v["deck"]["sideboard"].as_array().unwrap().len(), 32, "the sideboard sent is the one recorded");
    assert!(c0.next_message(Duration::from_millis(300)).is_none_or(|m| m["type"] != "refused"), "a short deck is not a refusal");
    // Ready is where a short deck is refused.
    c0.send(json!({"type": "ready"}));
    let refusal = c0.next_refusal(T);
    assert!(refusal["reason"].as_str().unwrap().contains("not legal"), "{refusal}");
    c0.send(json!({"type": "deck", "main": ["Griselbrand"], "lands": {"Island": 39}, "sideboard": []}));
    let refusal = c0.next_refusal(T);
    assert!(refusal["reason"].as_str().unwrap().contains("not in your drafted pool"), "{refusal}");
    c0.send(json!({"type": "deck", "main": pool[..23], "lands": {"Island": 9, "Swamp": 8}, "sideboard": pool[23..]}));
    let v = c0.view_until("a legal deck is accepted", T, |v| v["deck"]["valid"] == true);
    assert_eq!(v["deck"]["main"].as_array().unwrap().len(), 23);
    assert_eq!(v["deck"]["sideboard"].as_array().unwrap().len(), 19);
    assert_eq!(v["deck"]["ready"], false);
    c0.send(json!({"type": "ready"}));
    let v = c0.view_until("ready", T, |v| v["deck"]["ready"] == true);
    assert_eq!(v["seats"][0]["status"], "ready");
    // The deck file the runner can play.
    let deck_file = std::fs::read_to_string(server.dir.join("decks/seat-0.txt")).expect("decks/seat-0.txt");
    assert!(deck_file.contains("9 Island") && deck_file.contains("8 Swamp"), "{deck_file}");
    assert_eq!(deck_file.lines().map(|l| l.split_once(' ').unwrap().0.parse::<u32>().unwrap()).sum::<u32>(), 40);

    // The second human builds through the API's view, so both paths are read.
    let (status, body) = server.http_get(&format!("/api/view?seat=1&key={}", server.key(1)));
    assert_eq!(status, 200, "{body}");
    let api: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(api["type"], "view");
    assert_eq!(api["seat"], 1);
    assert_no_leak(&api, &all_cards);
    let pool1: Vec<String> = api["pool"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(pool1.len(), 42);
    c1.send(json!({"type": "deck", "main": pool1[..23], "lands": {"Plains": 9, "Forest": 8}, "sideboard": []}));
    c1.send(json!({"type": "ready"}));

    // Playing: the humans are paired (0 v 1) on two pages, the AI pair plays.
    let v = c0.view_until("the tournament starts", Duration::from_secs(60), |v| v["phase"] == "playing"
        && !v["matches"].as_array().unwrap().is_empty() && !v["matches"][0]["url"].is_null());
    let m = &v["matches"][0];
    assert_eq!(m["round"], 1);
    assert_eq!(m["opponent"], 1);
    let url0 = m["url"].as_str().expect("a game link for the human");
    assert!(url0.starts_with("http://127.0.0.1:"), "{url0}");
    let v1 = c1.view_until("seat 1 is paired", T, |v| !v["matches"].as_array().unwrap().is_empty() && !v["matches"][0]["url"].is_null());
    let url1 = v1["matches"][0]["url"].as_str().unwrap();
    assert_ne!(url0, url1, "human-vs-human is two ports");
    // The page is really there.
    let port: u16 = url0.trim_start_matches("http://127.0.0.1:").trim_end_matches('/').parse().unwrap();
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the game page listens");
    use std::io::{Read, Write};
    write!(s, "GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
    let mut page = String::new();
    s.read_to_string(&mut page).unwrap();
    assert!(page.starts_with("HTTP/1.1 200"), "{}", page.lines().next().unwrap_or(""));

    let log = server.wait_for_log("MATCH Round 1 — Seat 2 vs Seat 3", Duration::from_secs(120));
    assert!(log.contains("GAME 1 (Seat 2 vs Seat 3)"));
    let v = c0.view_until("the AI match is done in the pairings", T, |v| v["pairings"].as_array().unwrap().iter()
        .any(|p| p["a"] == 2 && p["b"] == 3 && p["status"] == "done"));
    let ai_match = v["pairings"].as_array().unwrap().iter().find(|p| p["a"] == 2).unwrap();
    assert!(ai_match["result"].as_str().unwrap().contains('-'), "{ai_match}");
    assert_eq!(v["matches"][0]["status"], "playing", "the humans' match waits for a browser");
    assert!(std::fs::read_dir(server.dir.join("games")).unwrap().count() >= 1, "games/ holds the AI game's log");

    // The static files: only the page, dist/ and assets/.
    let (status, _) = server.http_get("/dist/main.js");
    assert_eq!(status, 200);
    let (status, _) = server.http_get("/../Cargo.toml");
    assert_eq!(status, 404);
    let (status, _) = server.http_get("/src/main.ts");
    assert_eq!(status, 404);
    let (status, body) = server.http_get("/api/view?seat=0&key=wrong");
    assert_eq!(status, 403);
    assert!(body.contains("wrong key"));

    let terminal = server.stderr();
    for line in ["seat 0 joined", "the draft has started", "Lawson picked", "seat 2 picked",
                 "Lawson is ready", "round 1: seat 0 vs seat 1", "round 1: seat 2 vs seat 3",
                 "seat 0 plays at http://", "round 1: Seat 2 vs Seat 3:"] {
        assert!(terminal.contains(line), "the host's terminal says {line:?}:\n{terminal}");
    }
}
