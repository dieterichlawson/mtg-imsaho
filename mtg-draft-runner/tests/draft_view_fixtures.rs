//! The lobby walked in-process, with no port: a seat's view never names
//! another seat's cards, and one view per phase is written to
//! `mtg-gui/tests/draft-view-fixtures.json` for the page's own test to
//! read — the draft's version of `mtg-player/tests/gui_protocol.rs`.
//!
//! The packs here are disjoint on purpose: 168 distinct Innistrad cards
//! dealt so that no two seats ever hold the same name, which is what
//! makes "no other seat's card appears in this view" a real check rather
//! than one a shared common could hide.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use mtg_draft::deckbuilding::fallback_deck;
use mtg_draft::pack::BoosterPack;
use mtg_draft::set_data::SetData;
use mtg_draft::tournament::{GameOutcome, MatchResult};
use mtg_draft_runner::card_lines::CardLines;
use mtg_draft_runner::draft_log::DraftLogger;
use mtg_draft_runner::lobby::{Lobby, LobbyConfig, Phase, SeatKind};
use mtg_engine::cards::CardRegistry;
use serde_json::Value;

const POD: usize = 4;
const PACK: usize = 14;

fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

/// A table whose packs share no card: `packs[seat][round]`.
fn disjoint_packs(set_data: &SetData) -> Vec<Vec<BoosterPack>> {
    let mut names: Vec<String> = set_data.all_card_names();
    names.sort();
    assert!(names.len() >= POD * 3 * PACK, "ISD has enough cards");
    let mut it = names.into_iter();
    (0..POD).map(|_| (0..3).map(|_| {
        let cards: Vec<String> = it.by_ref().take(PACK).collect();
        BoosterPack {
            commons: cards[..PACK - 2].to_vec(),
            uncommons: vec![],
            rare: cards[PACK - 2].clone(),
            dfc: cards[PACK - 1].clone(),
            foil: None,
        }
    }).collect()).collect()
}

fn lobby(seats: Vec<SeatKind>) -> (Lobby, Vec<Vec<BoosterPack>>) {
    lobby_with(seats, None)
}

fn lobby_with(seats: Vec<SeatKind>, pick_seconds: Option<u64>) -> (Lobby, Vec<Vec<BoosterPack>>) {
    let set_path = repo_root().join("data/sets/isd.json");
    let mut set_data = SetData::load(&set_path).unwrap();
    let registry = Arc::new(CardRegistry::with_all_cards());
    set_data.filter_implemented(&registry);
    let mut packs = disjoint_packs(&set_data);
    packs.truncate(seats.len());
    let lines = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);
    let config = LobbyConfig {
        set_code: "isd".into(),
        set_name: set_data.set_name.clone(),
        seats,
        best_of: 3,
        seed: 1,
        guide_path: None,
        pick_seconds,
        build_seconds: None,
        out_dir: None,
    };
    let lobby = Lobby::new(config, &packs, &set_data, registry, &lines, DraftLogger::silent()).unwrap();
    (lobby, packs)
}

#[test]
fn a_table_dealt_for_the_wrong_number_of_seats_is_refused() {
    let set_path = repo_root().join("data/sets/isd.json");
    let mut set_data = SetData::load(&set_path).unwrap();
    let registry = Arc::new(CardRegistry::with_all_cards());
    set_data.filter_implemented(&registry);
    let packs = disjoint_packs(&set_data);
    let lines = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);
    let config = LobbyConfig {
        set_code: "isd".into(), set_name: set_data.set_name.clone(),
        seats: vec![SeatKind::Human, SeatKind::Ai("cc".into())],
        best_of: 1, seed: 1, guide_path: None, pick_seconds: None, build_seconds: None, out_dir: None,
    };
    let err = Lobby::new(config, &packs, &set_data, registry, &lines, DraftLogger::silent()).err()
        .expect("four seats' packs do not seat two: a pass would go to a seat that is not at the table");
    assert!(err.contains("2 seats but packs for 4"), "{err}");
}

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings(x, out)),
        _ => {}
    }
}

/// Every card name in `view` is one of `allowed`.
fn assert_only(view: &Value, allowed: &[String], all: &[String], what: &str) {
    let mut seen = Vec::new();
    strings(view, &mut seen);
    for s in seen {
        let front = mtg_draft::front_face(&s).to_string();
        if all.contains(&front) {
            assert!(allowed.contains(&front), "{what}: seat {}'s view names {s:?}, another seat's card", view["seat"]);
        }
    }
}

/// Pick for whoever can, AI seats and humans alike, until the table is
/// drafted; humans pick last-card, AIs first-card, so the pools differ.
fn draft_to_the_end(lobby: &mut Lobby, stop_at_picks: Option<usize>) {
    loop {
        let seats = lobby.seats_with_a_pack();
        if seats.is_empty() {
            return;
        }
        for seat in seats {
            if stop_at_picks.is_some_and(|n| jv(&lobby, 0)["picks"].as_array().unwrap().len() >= n) && seat == 0 {
                continue;
            }
            let view = jv(&lobby, seat);
            let Some(id) = view["pack"]["id"].as_u64() else { continue };
            let n = view["pack"]["cards"].as_array().unwrap().len();
            match lobby.kind(seat) {
                Some(SeatKind::Ai(_)) => lobby.ai_pick(seat, id as usize, 0, "prompt", "{\"pick\":0}", false).unwrap(),
                _ => lobby.pick(seat, id as usize, n - 1).unwrap(),
            }
        }
        if stop_at_picks.is_some_and(|n| jv(&lobby, 0)["picks"].as_array().unwrap().len() >= n)
            && lobby.seats_with_a_pack().iter().all(|s| *s == 0)
        {
            return;
        }
    }
}

#[test]
fn a_seat_never_sees_another_seats_pack_or_pool_and_each_phase_has_a_fixture() {
    let (mut lobby, packs) = lobby(vec![SeatKind::Human, SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())]);
    let all: Vec<String> = packs.iter().flatten().flat_map(BoosterPack::all_cards)
        .map(|c| mtg_draft::front_face(&c).to_string()).collect();
    let registry = CardRegistry::with_all_cards();
    let mut fixtures: HashMap<&str, Value> = HashMap::new();
    let keys = lobby.human_keys();
    assert_eq!(keys.len(), 2);
    assert!(lobby.check_key(0, &keys[0].1).is_ok());
    assert!(lobby.check_key(0, &keys[1].1).is_err());
    assert!(lobby.check_key(2, "x").unwrap_err().contains("ai seat"));

    // Lobby: seat 0 is here, seat 1 is not.
    lobby.connected(0);
    assert_eq!(lobby.phase(), Phase::Lobby);
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["phase"], "lobby");
    assert!(v["pack"].is_null());
    assert_only(&v, &[], &all, "lobby");
    fixtures.insert("lobby", v);

    // Drafting: the last human arrives and the packs are dealt. Every
    // seat's view, at the start and partway through, names only its own.
    lobby.connected(1);
    assert_eq!(lobby.phase(), Phase::Drafting);
    let own = |seat: usize| -> Vec<String> {
        packs[seat].iter().flat_map(BoosterPack::all_cards).map(|c| mtg_draft::front_face(&c).to_string()).collect()
    };
    for seat in 0..POD {
        let v = serde_json::to_value(lobby.view(seat)).unwrap();
        assert_eq!(v["pack"]["cards"].as_array().unwrap().len(), PACK);
        assert_only(&v, &own(seat), &all, "first pick");
    }
    // Partway: seat 0 holds a pack seat 3 opened (round 1 passes left),
    // so the allowed set is seat 0's own picks plus the cards of the pack
    // in front of it, and nothing of anybody else's pool.
    draft_to_the_end(&mut lobby, Some(3));
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["phase"], "drafting");
    let mut allowed: Vec<String> = v["pack"]["cards"].as_array().unwrap().iter()
        .map(|c| c["name"].as_str().unwrap().to_string()).collect();
    allowed.extend(v["pool"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()));
    let v_json = v.clone();
    let mut seen = Vec::new();
    strings(&v_json, &mut seen);
    for s in &seen {
        if all.contains(s) {
            assert!(allowed.contains(s), "mid-draft, seat 0's view names {s:?}, not in its pack or pool");
        }
    }
    assert!(v["pack"]["waiting"].as_u64().unwrap() >= 1, "seat 0 fell behind, so packs queue: {}", v["pack"]);
    assert_eq!(v["seats"][3]["picks"].as_u64().unwrap() > 3, true, "{}", v["seats"]);
    fixtures.insert("drafting", v);

    // Building: a deck sent and refused, then accepted and ready.
    draft_to_the_end(&mut lobby, None);
    assert_eq!(lobby.phase(), Phase::Building);
    let pool: Vec<String> = jv(&lobby, 0)["pool"].as_array().unwrap().iter()
        .map(|c| c["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(pool.len(), 42);
    // A short deck is recorded as work in progress, not refused; a card
    // the seat did not draft is refused; `ready` on a short deck is refused.
    lobby.submit_deck(0, &pool[..5], &HashMap::new(), &[]).expect("a short deck is kept, not refused");
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["deck"]["valid"], false);
    assert!(v["deck"]["problem"].as_str().unwrap().contains("40"));
    let err = lobby.submit_deck(0, &["Griselbrand".to_string()], &HashMap::new(), &[]).unwrap_err();
    assert!(err.contains("not in your drafted pool"), "{err}");
    let err = lobby.ready(0).unwrap_err();
    assert!(err.contains("not legal"), "{err}");
    let fb = fallback_deck(&pool, &registry);
    lobby.submit_deck(0, &fb.maindeck, &fb.lands, &[]).unwrap();
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["deck"]["valid"], true);
    assert_eq!(v["deck"]["ready"], false);
    assert_only(&v, &own_pool(&lobby, 0), &all, "building");
    fixtures.insert("building", v);
    lobby.ready(0).unwrap();
    assert_eq!(jv(&lobby, 0)["seats"][0]["status"], "ready");
    // The others: seat 1 auto-built by a kick, the AIs through their
    // worker's result.
    lobby.kick(1).unwrap();
    for seat in 2..POD {
        let pool: Vec<String> = jv(&lobby, seat)["pool"].as_array().unwrap().iter()
            .map(|c| c["name"].as_str().unwrap().to_string()).collect();
        let deck = fallback_deck(&pool, &registry);
        lobby.ai_deck(seat, &mtg_draft_runner::deck::DeckBuildResult {
            deck, attempts: vec![], retries: 0, fallback: false,
        });
    }
    assert!(lobby.all_ready());

    // Playing: round 1 pairs 0 v 1 and 2 v 3; seat 0's page is up, a game
    // is in, then the match, and the round's end pairs round 2.
    let to_play = lobby.begin_tournament();
    assert_eq!(lobby.phase(), Phase::Playing);
    assert_eq!(to_play, vec![(1, 0, 1), (1, 2, 3)]);
    lobby.match_started(1, 0, 1, [Some("http://192.168.1.20:8801/".into()), None]);
    lobby.match_started(1, 2, 3, [None, None]);
    let game = |winner: usize| GameOutcome { winner: Some(winner), turns: 9, game_log: vec!["log".into()], stalled_seat: None, abandoned: false };
    lobby.game_finished(1, 0, 1, &game(0));
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["phase"], "playing");
    assert_eq!(v["matches"][0]["opponent"], 1);
    assert_eq!(v["matches"][0]["url"], "http://192.168.1.20:8801/");
    assert_eq!(v["matches"][0]["status"], "playing");
    assert_eq!(v["matches"][0]["games"][0]["winner"], 0);
    assert_eq!(v["pairings"].as_array().unwrap().len(), 2);
    assert_eq!(v["standings"][0]["points"], 0);
    assert_only(&v, &own_pool(&lobby, 0), &all, "playing");
    let v1 = serde_json::to_value(lobby.view(1)).unwrap();
    assert!(v1["matches"][0]["url"].is_null(), "seat 1's link is its own, not seat 0's");
    assert_eq!(v1["matches"][0]["opponent"], 0);
    fixtures.insert("playing", v);
    let result = |a: usize, b: usize, wa: usize, wb: usize| MatchResult {
        player_a: a, player_b: b, wins_a: wa, wins_b: wb,
        games: (0..wa).map(|_| game(a)).chain((0..wb).map(|_| game(b))).collect(),
    };
    assert!(lobby.match_finished(1, 0, 1, result(0, 1, 2, 0)).is_empty(), "the round is not over");
    let next = lobby.match_finished(1, 2, 3, result(2, 3, 2, 1));
    assert_eq!(next, vec![(2, 0, 2), (2, 1, 3)], "winners play winners");
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["standings"][0]["seat"], 0);
    assert_eq!(v["standings"][0]["wins"], 1);
    assert_eq!(v["matches"][0]["result"], "2-0");
    assert_eq!(v["matches"][1]["round"], 2);
    assert_eq!(v["matches"][1]["opponent"], 2);

    // Done.
    assert!(lobby.match_finished(2, 0, 2, result(0, 2, 2, 1)).is_empty());
    assert!(lobby.match_finished(2, 1, 3, result(1, 3, 0, 2)).is_empty());
    assert_eq!(lobby.phase(), Phase::Done);
    let v = serde_json::to_value(lobby.view(0)).unwrap();
    assert_eq!(v["phase"], "done");
    assert_eq!(v["standings"][0]["seat"], 0);
    assert_eq!(v["standings"][0]["points"], 6);
    assert_eq!(v["standings"][3]["seat"], 1);
    assert_only(&v, &own_pool(&lobby, 0), &all, "done");
    fixtures.insert("done", v);
    let events = lobby.take_events();
    assert!(events.iter().any(|e| e.contains("the tournament is over")), "{events:?}");

    // The fixture: one view per phase, plus a refusal, for the page's test.
    let mut doc = serde_json::Map::new();
    doc.insert("about".into(), Value::String(
        "Written by mtg-draft-runner/tests/draft_view_fixtures.rs: the view JSON mtg-draft-server sends \
seat 0 in each phase, and a refused message. Regenerate with `cargo test -p mtg-draft-runner --test \
draft_view_fixtures` and commit the result.".into()));
    for phase in ["lobby", "drafting", "building", "playing", "done"] {
        doc.insert(phase.into(), fixtures.remove(phase).unwrap());
    }
    doc.insert("refused".into(), serde_json::to_value(mtg_draft_runner::lobby::Refused {
        kind: "refused",
        reason: "'Griselbrand' is not in your drafted pool.".into(),
        echo: serde_json::json!({"type": "deck", "main": ["Griselbrand"], "lands": {}, "sideboard": []}),
    }).unwrap());
    let doc = Value::Object(doc);
    let path = repo_root().join("mtg-gui/tests/draft-view-fixtures.json");
    let current: Option<Value> = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok());
    if current.as_ref() != Some(&doc) {
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
        assert!(current.is_some(), "{} did not exist; it has been written — commit it", path.display());
        panic!("{} was stale and has been rewritten — commit it", path.display());
    }
}

/// The host starts without a person who has not arrived. Under a pick
/// timer the table picks for them when it runs out, not at once — the
/// first playtest's absent seat lost all 42 picks in four seconds to a
/// fast table, and the person who joined a minute late found the draft
/// over. Without a timer the table picks at once, as before; a kicked
/// seat is picked for at once either way.
#[test]
fn an_absent_seat_under_a_timer_is_picked_for_when_the_timer_runs_out() {
    use std::time::{Duration, Instant};
    let (mut lobby, _) = lobby_with(vec![SeatKind::Human, SeatKind::Ai("cc".into())], Some(30));
    lobby.start();
    assert_eq!(lobby.phase(), Phase::Drafting);
    let v = jv(&lobby, 0);
    assert_eq!(v["seats"][0]["auto"], true);
    assert_eq!(v["picks"].as_array().unwrap().len(), 0, "nothing is picked at once");
    let left = v["pack"]["deadline_ms"].as_u64().expect("the absent seat's deadline is armed");
    assert!(left > 25_000 && left <= 30_000, "{left}");
    assert!(!lobby.tick(Instant::now() + Duration::from_secs(29)), "not yet");
    assert_eq!(jv(&lobby, 0)["picks"].as_array().unwrap().len(), 0);
    assert!(lobby.tick(Instant::now() + Duration::from_secs(31)), "the timer picks");
    let v = jv(&lobby, 0);
    assert_eq!(v["picks"].as_array().unwrap().len(), 1);
    assert_eq!(v["picks"][0]["auto"], true);
    assert!(v["notice"].as_str().unwrap().contains("the seat is away"), "{}", v["notice"]);
    assert!(v["pack"].is_null(), "a two-seat table: nothing in front until the AI passes");
    // The AI seat's pack comes in the same way: not taken at once, on the clock.
    let ai_pack = jv(&lobby, 1)["pack"]["id"].as_u64().unwrap() as usize;
    lobby.ai_pick(1, ai_pack, 0, "p", "{\"pick\":0}", false).unwrap();
    let v = jv(&lobby, 0);
    assert_eq!(v["picks"].as_array().unwrap().len(), 1, "the pack passed in is not taken at once");
    let id = v["pack"]["id"].as_u64().expect("the AI's pack is in front") as usize;
    assert!(v["pack"]["deadline_ms"].as_u64().is_some_and(|ms| ms > 25_000), "{}", v["pack"]);
    // The person arrives: the seat is theirs, with the timer still running.
    lobby.connected(0);
    let v = jv(&lobby, 0);
    assert_eq!(v["seats"][0]["auto"], false);
    assert_eq!(v["pack"]["id"], id);
    assert!(v["pack"]["deadline_ms"].as_u64().unwrap() <= 30_000);
    // Kicked: at once, every pack in front of it.
    lobby.kick(0).unwrap();
    assert!(jv(&lobby, 0)["picks"].as_array().unwrap().len() >= 2);

    // No timer: the table picks for an absent seat at once.
    let (mut lobby, _) = lobby_with(vec![SeatKind::Human, SeatKind::Ai("cc".into())], None);
    lobby.start();
    assert!(jv(&lobby, 0)["picks"].as_array().unwrap().len() >= 1);
}

/// A seat's view as JSON.
fn jv(lobby: &Lobby, seat: usize) -> Value {
    serde_json::to_value(lobby.view(seat)).unwrap()
}

fn own_pool(lobby: &Lobby, seat: usize) -> Vec<String> {
    let mut names: Vec<String> = jv(&lobby, seat)["pool"].as_array().unwrap().iter()
        .map(|c| c["name"].as_str().unwrap().to_string()).collect();
    names.extend(["Plains", "Island", "Swamp", "Mountain", "Forest"].map(String::from));
    names
}
