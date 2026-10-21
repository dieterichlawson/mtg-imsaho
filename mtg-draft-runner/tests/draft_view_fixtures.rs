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
use mtg_draft_runner::lobby::{Lobby, LobbyConfig, Phase, SeatKind, TableSave};
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
    lobby_timed(seats, pick_seconds, None)
}

fn lobby_timed(
    seats: Vec<SeatKind>, pick_seconds: Option<u64>, build_seconds: Option<u64>,
) -> (Lobby, Vec<Vec<BoosterPack>>) {
    lobby_clocked(seats, pick_seconds, build_seconds, None)
}

fn lobby_clocked(
    seats: Vec<SeatKind>, pick_seconds: Option<u64>, build_seconds: Option<u64>, sideboard_seconds: Option<u64>,
) -> (Lobby, Vec<Vec<BoosterPack>>) {
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
        build_seconds,
        sideboard_seconds,
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
        best_of: 1, seed: 1, guide_path: None, pick_seconds: None, build_seconds: None, sideboard_seconds: None,
        out_dir: None,
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
    // ... because round 1 is not over: the view says what they count (#818).
    assert_eq!(v["standings_after"], 0);
    assert_only(&v, &own_pool(&lobby, 0), &all, "playing");
    let v1 = serde_json::to_value(lobby.view(1)).unwrap();
    assert!(v1["matches"][0]["url"].is_null(), "seat 1's link is its own, not seat 0's");
    assert_eq!(v1["matches"][0]["opponent"], 0);
    fixtures.insert("playing", v);
    // Between games 1 and 2: seat 0 won game 1, so seat 1 (kicked, the
    // table's) chooses; seat 0 may sideboard, beside what seat 1 showed.
    let showed = jv(&lobby, 1)["pool"][0]["name"].as_str().unwrap().to_string();
    lobby.open_between(1, 0, 1, 2, 1, [vec![], vec![showed.clone()]], [None, None]);
    let v = jv(&lobby, 0);
    let b = &v["matches"][0]["between"];
    assert_eq!((b["game"].as_u64(), b["sideboarding"].as_bool(), b["you_choose"].as_bool()), (Some(2), Some(true), Some(false)));
    assert_eq!(b["opponent_ready"], true, "the table's seat has nothing to wait for");
    assert_eq!(b["opponent_showed"][0]["name"], showed.as_str());
    assert_eq!(b["deck"]["main"], v["deck"]["main"], "the deck played last game");
    let mut allowed = own_pool(&lobby, 0);
    allowed.push(mtg_draft::front_face(&showed).to_string());
    assert_only(&v, &allowed, &all, "playing_sideboard");
    assert!(jv(&lobby, 1)["matches"][0]["between"]["opponent_showed"].as_array().unwrap().is_empty(),
        "seat 0 has shown nothing");
    fixtures.insert("playing_sideboard", v);
    lobby.sideboard_ready(0).unwrap();
    let out = lobby.take_between(1, 0, 1).expect("both ready");
    assert!(out.decks.iter().all(Option::is_none) && out.chooser_plays.is_none());
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
    assert_eq!(v["standings_after"], 1);
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
    // Between two games of a match, sideboarding (MTR 2.4).
    doc.insert("playing_sideboard".into(), fixtures.remove("playing_sideboard").unwrap());
    // A seat on a bye, from an odd table: the page must read its status
    // word and its pairing with no opponent (#814).
    let (odd, _, _, bye) = odd_table_round_one();
    doc.insert("playing_bye".into(), jv(&odd, bye));
    doc.insert("refused".into(), serde_json::to_value(mtg_draft_runner::lobby::Refused {
        kind: "refused",
        reason: "'Griselbrand' is not in your drafted pool.".into(),
        echo: serde_json::json!({"type": "deck", "main": ["Griselbrand"], "lands": {}, "sideboard": []}),
    }).unwrap());
    let doc = Value::Object(doc);
    let path = repo_root().join("mtg-gui/tests/draft-view-fixtures.json");
    // Compared as text: a tiebreaker is a float, and a float read back
    // through serde_json is not always the bits that were written, so two
    // equal fixtures compared as values could never match.
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    let current = std::fs::read_to_string(&path).ok();
    if current.as_deref() != Some(text.as_str()) {
        std::fs::write(&path, &text).unwrap();
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

/// #810: when the build clock runs out, the deck the person sent is the
/// one played — as sent when it is legal, topped up with basics when it is
/// only short — and the table's own deck is for a seat that sent nothing.
#[test]
fn the_build_clock_keeps_the_deck_a_person_sent() {
    use std::time::{Duration, Instant};
    let lands = |pairs: &[(&str, u32)]| -> HashMap<String, u32> {
        pairs.iter().map(|(l, n)| ((*l).to_string(), *n)).collect()
    };
    for (sent, want) in [
        // A legal 40, never marked ready.
        (Some((23, lands(&[("Island", 9), ("Swamp", 8)]))), Some(lands(&[("Island", 9), ("Swamp", 8)]))),
        // 39: one more of the basic it plays most of.
        (Some((22, lands(&[("Island", 9), ("Swamp", 8)]))), Some(lands(&[("Island", 10), ("Swamp", 8)]))),
        // Nothing sent: the table's deck.
        (None, None),
        // Five spells and no lands is not a deck the seat built: 35 basics
        // would be, so the table builds, and says why it did not keep it.
        (Some((5, lands(&[]))), None),
    ] {
        let (mut lobby, _) = lobby_timed(vec![SeatKind::Human, SeatKind::Ai("cc".into())], None, Some(20));
        lobby.connected(0);
        lobby.start();
        draft_to_the_end(&mut lobby, None);
        let pool: Vec<String> = jv(&lobby, 0)["pool"].as_array().unwrap().iter()
            .map(|c| c["name"].as_str().unwrap().to_string()).collect();
        let main: Vec<String> = sent.as_ref().map(|(n, _)| pool[..*n].to_vec()).unwrap_or_default();
        if let Some((_, l)) = &sent {
            lobby.submit_deck(0, &main, l, &[]).unwrap();
        }
        assert!(jv(&lobby, 0)["build_deadline_ms"].as_u64().is_some(), "the clock is armed");
        assert!(lobby.tick(Instant::now() + Duration::from_secs(21)), "the clock runs out");
        let v = jv(&lobby, 0);
        assert_eq!(v["deck"]["ready"], true);
        let notice = v["notice"].as_str().unwrap();
        match want {
            Some(want) => {
                let got: HashMap<String, u32> = serde_json::from_value(v["deck"]["lands"].clone()).unwrap();
                assert_eq!(got, want, "the lands sent are kept: {v}");
                let got_main: Vec<String> = v["deck"]["main"].as_array().unwrap().iter()
                    .map(|c| c.as_str().unwrap().to_string()).collect();
                assert_eq!(got_main.len(), main.len(), "the spells sent are kept: {v}");
                assert!(notice.contains("as you last sent it"), "{notice}");
            }
            None => {
                assert!(notice.contains("The table built your deck"), "{notice}");
                assert_eq!(notice.contains("the deck you sent could not be played"), sent.is_some(), "{notice}");
            }
        }
    }
}

/// #811: a seat that has drafted all its cards is building — its view
/// says so, with its deck and its clock — while a slower seat still picks.
/// The view carried the table's phase, so the page and the client showed
/// "waiting for a pack" while the build clock ran.
#[test]
fn a_seat_that_finishes_first_is_shown_building_while_others_draft() {
    let (mut lobby, _) = lobby_timed(vec![SeatKind::Human, SeatKind::Ai("cc".into())], None, Some(60));
    lobby.connected(0);
    lobby.start();
    // The person picks whenever a pack is in front of them; the AI seat
    // only when the person has nothing to do, so it finishes last.
    while jv(&lobby, 0)["picks"].as_array().unwrap().len() < 42 {
        let v = jv(&lobby, 0);
        if let Some(id) = v["pack"]["id"].as_u64() {
            lobby.pick(0, id as usize, 0).unwrap();
        } else {
            let id = jv(&lobby, 1)["pack"]["id"].as_u64().expect("the AI seat holds the pack");
            lobby.ai_pick(1, id as usize, 0, "prompt", "{\"pick\":0}", false).unwrap();
        }
    }
    assert_eq!(lobby.phase(), Phase::Drafting, "the AI seat is still picking");
    assert!(jv(&lobby, 1)["pack"].is_object(), "the AI seat has its last pack");
    assert_eq!(jv(&lobby, 1)["phase"], "drafting");
    let v = jv(&lobby, 0);
    assert_eq!(v["phase"], "building", "the seat that is done is building: {}", v["seats"]);
    assert_eq!(v["seats"][0]["status"], "building");
    assert!(v["build_deadline_ms"].as_u64().is_some(), "its clock is running");
    assert!(v["pass_direction"].is_null(), "nothing is passed to a seat that is building");
    // What the building screen needs is there, and a deck is taken.
    let pool: Vec<String> = v["pool"].as_array().unwrap().iter()
        .map(|c| c["name"].as_str().unwrap().to_string()).collect();
    let lands: HashMap<String, u32> = [("Island".to_string(), 17)].into();
    lobby.submit_deck(0, &pool[..23], &lands, &[]).unwrap();
    assert_eq!(jv(&lobby, 0)["deck"]["valid"], true);
}

/// #812: a timer too long for an `Instant` panicked inside the person's
/// connection while it held the lobby, wedging the table. The flags refuse
/// one now; the lobby, which is a library, takes it as no deadline.
#[test]
fn a_timer_too_long_for_the_clock_is_no_deadline_not_a_panic() {
    let (mut lobby, _) = lobby_timed(vec![SeatKind::Human, SeatKind::Ai("cc".into())], Some(u64::MAX), Some(u64::MAX));
    lobby.connected(0);
    lobby.start();
    assert!(jv(&lobby, 0)["pack"]["deadline_ms"].is_null());
    draft_to_the_end(&mut lobby, None);
    let v = jv(&lobby, 0);
    assert_eq!(v["phase"], "building");
    assert!(v["build_deadline_ms"].is_null());
}

/// #814: at an odd table the seat given the bye is told so by its own
/// view: the round's pairings name it with no opponent, in the round its
/// matches are numbered in, and its status is "bye", not "waiting".
#[test]
fn an_odd_tables_bye_is_in_the_seats_own_view() {
    let (lobby, round, a, bye) = odd_table_round_one();
    let v = jv(&lobby, bye);
    let own: Vec<&Value> = v["pairings"].as_array().unwrap().iter()
        .filter(|p| p["a"] == bye && p["b"].is_null()).collect();
    assert_eq!(own.len(), 1, "{}", v["pairings"]);
    assert_eq!(own[0]["round"], round, "the bye's round is the matches' round");
    assert!(v["matches"].as_array().unwrap().is_empty());
    assert_eq!(v["seats"][bye]["status"], "bye");
    assert_eq!(jv(&lobby, a)["seats"][bye]["status"], "bye", "every seat sees it");
}

/// A three-seat table drafted, built and paired for round one: the lobby,
/// the round, one seat of the match and the seat with the bye.
fn odd_table_round_one() -> (Lobby, usize, usize, usize) {
    let (mut lobby, _) = lobby(vec![SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())]);
    lobby.connected(0);
    lobby.start();
    draft_to_the_end(&mut lobby, None);
    let registry = CardRegistry::with_all_cards();
    lobby.kick(0).unwrap();
    for seat in 1..3 {
        let pool: Vec<String> = jv(&lobby, seat)["pool"].as_array().unwrap().iter()
            .map(|c| c["name"].as_str().unwrap().to_string()).collect();
        let deck = fallback_deck(&pool, &registry);
        lobby.ai_deck(seat, &mtg_draft_runner::deck::DeckBuildResult {
            deck, attempts: vec![], retries: 0, fallback: false,
        });
    }
    let to_play = lobby.begin_tournament();
    assert_eq!(to_play.len(), 1, "three seats play one match: {to_play:?}");
    let (round, a, b) = to_play[0];
    let bye = (0..3).find(|s| *s != a && *s != b).unwrap();
    (lobby, round, a, bye)
}

/// #816, #826: the host's terminal names a person the way the page does,
/// "Lea (seat 0)", in the tournament's lines as in the draft's, with one
/// spelling of "seat", and one round is "1 round".
#[test]
fn the_hosts_lines_name_a_seat_as_the_page_does() {
    let (mut lobby, _) = lobby(vec![SeatKind::Human, SeatKind::Ai("cc".into())]);
    lobby.connected(0);
    lobby.set_name(0, "Lea");
    lobby.start();
    draft_to_the_end(&mut lobby, None);
    let registry = CardRegistry::with_all_cards();
    lobby.kick(0).unwrap();
    let pool: Vec<String> = jv(&lobby, 1)["pool"].as_array().unwrap().iter()
        .map(|c| c["name"].as_str().unwrap().to_string()).collect();
    lobby.ai_deck(1, &mtg_draft_runner::deck::DeckBuildResult {
        deck: fallback_deck(&pool, &registry), attempts: vec![], retries: 0, fallback: false,
    });
    assert_eq!(lobby.begin_tournament(), vec![(1, 0, 1)]);
    let game = GameOutcome { winner: Some(1), turns: 9, game_log: vec!["log".into()], stalled_seat: None, abandoned: false };
    lobby.game_finished(1, 0, 1, &game);
    lobby.match_finished(1, 0, 1, MatchResult {
        player_a: 0, player_b: 1, wins_a: 0, wins_b: 2, games: vec![game.clone(), game],
    });
    let events = lobby.take_events().join("\n");
    for line in ["Lea (seat 0) picked", "Lea (seat 0) kicked", "Lea (seat 0): the table built its deck",
                 "the tournament starts: 1 round, best-of-3", "round 1: Lea (seat 0) vs seat 1\n",
                 "round 1: Lea (seat 0) vs seat 1, game 1: seat 1 wins",
                 "round 1: Lea (seat 0) vs seat 1: 0-2 (winner: seat 1)", ". Lea (seat 0) — 0-1"] {
        assert!(events.contains(line), "{line:?} in:\n{events}");
    }
    assert!(!events.contains("Seat "), "one spelling of a seat:\n{events}");
}

/// #856: a client re-sends its name on every connect, and the host saw
/// "Cli (seat 0) is now called Cli" as the only sign the seat was back.
#[test]
fn a_reconnect_says_the_seat_is_back_and_its_own_name_is_no_rename() {
    let (mut lobby, _) = lobby(vec![SeatKind::Human, SeatKind::Ai("cc".into())]);
    lobby.connected(0);
    lobby.set_name(0, "Cli");
    lobby.start();
    let events = lobby.take_events().join("\n");
    assert!(events.contains("seat 0 is now called Cli"), "{events}");
    lobby.disconnected(0);
    lobby.connected(0);
    lobby.set_name(0, "Cli");
    let events = lobby.take_events().join("\n");
    assert_eq!(events, "Cli (seat 0) is back");
    // A second tab while the first is open is not a return.
    lobby.connected(0);
    assert!(lobby.take_events().is_empty());
    // Nor is a kicked seat's page connecting again: the table still plays
    // for it.
    lobby.kick(0).unwrap();
    lobby.disconnected(0);
    lobby.disconnected(0);
    lobby.take_events();
    lobby.connected(0);
    let events = lobby.take_events().join("\n");
    assert!(!events.contains("is back"), "{events}");
}

/// A seat kicked while building is told it was handed to the table: the
/// table's building of its deck wrote over that line before any view went
/// out, so the page's banner said only "The table built your deck" (#817's
/// review).
#[test]
fn a_seat_kicked_while_building_is_told_so() {
    let (mut lobby, _) = lobby(vec![SeatKind::Human, SeatKind::Ai("cc".into())]);
    lobby.connected(0);
    lobby.start();
    draft_to_the_end(&mut lobby, None);
    assert_eq!(jv(&lobby, 0)["phase"], "building");
    lobby.kick(0).unwrap();
    let notice = jv(&lobby, 0)["notice"].as_str().unwrap_or("").to_string();
    assert!(notice.contains("handed your seat to the table"), "{notice:?}");
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

/// #819: the 42nd pick was announced after the lines it causes — "seat 2
/// has drafted all 42 cards", "the draft is over" — on every pick path:
/// a person's, an AI seat's and the pick timer's. Each seat's picks come
/// before its "drafted all", and nothing is picked after the draft is over.
#[test]
fn the_last_pick_is_announced_before_what_it_causes() {
    for by_timer in [false, true] {
        let (mut lobby, _) = lobby(vec![SeatKind::Human, SeatKind::Ai("cc".into())]);
        lobby.connected(0);
        lobby.start();
        let mut events = Vec::new();
        while !lobby.seats_with_a_pack().is_empty() {
            if let Some(id) = jv(&lobby, 0)["pack"]["id"].as_u64() {
                if by_timer {
                    lobby.auto_pick(0, "the pick timer ran out").unwrap();
                } else {
                    lobby.pick(0, id as usize, 0).unwrap();
                }
            } else {
                let id = jv(&lobby, 1)["pack"]["id"].as_u64().expect("the AI seat holds the pack");
                lobby.ai_pick(1, id as usize, 0, "prompt", "{\"pick\":0}", false).unwrap();
            }
            events.extend(lobby.take_events());
        }
        let last = |pred: &dyn Fn(&str) -> bool| events.iter().rposition(|e| pred(e));
        let first = |pred: &dyn Fn(&str) -> bool| events.iter().position(|e| pred(e));
        let over = first(&|e| e.starts_with("the draft is over")).expect("the draft ends");
        for seat in ["seat 0", "seat 1"] {
            let picked = last(&|e| e.starts_with(seat) && e.contains("picked")).expect("the seat picked");
            let done = first(&|e| e.starts_with(&format!("{seat} has drafted all"))).expect("the seat is done");
            assert!(picked < done, "by_timer {by_timer}: {seat}'s last pick after its \"drafted all\":\n{}",
                events[picked.min(done)..].join("\n"));
            assert!(picked < over, "by_timer {by_timer}: {seat} picked after the draft was over:\n{}",
                events[over..].join("\n"));
        }
    }
}

/// #822 and #825: a seat's presence is one word every surface prints, and
/// a person in a match is wherever their game page is. The draft page's
/// socket said "here" for a player who had closed the game and "away" for
/// one playing it, and a kicked seat with its tab open read "here".
#[test]
fn a_seat_is_where_its_game_page_is_and_a_kicked_seat_says_so() {
    let (mut l, _) = lobby(vec![SeatKind::Human, SeatKind::Human, SeatKind::Ai("cc".into())]);
    let presence = |l: &Lobby, seat: usize| l.view(2).seats[seat].presence;
    assert_eq!(presence(&l, 0), "not yet");
    assert_eq!(presence(&l, 2), "here", "an AI seat is always here");
    l.connected(0);
    l.connected(1);
    assert_eq!(presence(&l, 0), "here");
    l.disconnected(0);
    assert_eq!(presence(&l, 0), "away");

    // In a match: the game page decides, whatever the draft page does.
    l.connected(0);
    assert!(l.set_game_pages(0, Some(0)));
    assert_eq!(presence(&l, 0), "game page closed", "a match with nobody at its game");
    assert!(l.set_game_pages(0, Some(1)));
    assert_eq!(presence(&l, 0), "in game");
    l.disconnected(0);
    assert_eq!(presence(&l, 0), "in game", "playing with the draft page closed is not away");
    l.take_events();
    assert!(l.set_game_pages(0, Some(0)));
    assert!(l.take_events().iter().any(|e| e == "seat 0's game page is closed; the game waits for them to open it again"),
        "the host is told");
    assert!(!l.set_game_pages(0, Some(0)), "no change, nothing to send");
    assert!(l.set_game_pages(0, Some(2)));
    assert!(l.take_events().iter().any(|e| e == "seat 0's game page is open again"));
    assert!(l.set_game_pages(0, None));
    assert_eq!(presence(&l, 0), "away", "the match is over and the draft page is closed");

    // A kick is the seat's state from then on, and the reason the table
    // gives for everything it does for the seat.
    l.kick(1).unwrap();
    assert_eq!(presence(&l, 1), "kicked", "with its draft page still open");
    assert_eq!(l.away_reason(1), "the host kicked the seat");
    assert_eq!(l.away_reason(0), "the seat is away");
    l.seat_gave_up(2, "no answer");
    assert_eq!(presence(&l, 2), "not answering");
    assert_eq!(l.away_reason(2), "the seat stopped answering");
    let forfeit = l.forfeit_result(1, 0);
    assert_eq!(forfeit.games[0].game_log, vec!["forfeit: the host kicked the seat".to_string()]);
}

/// The hosted table's snapshot: a server that stops, mid-draft or
/// mid-tournament, carries on where it was with `--resume` — the same
/// join keys, packs, pools, picks, names, AI notes, decks and standings,
/// and the matches left to play — where it used to start a new draft.
#[test]
fn a_snapshot_carries_the_table_on_where_it_stopped() {
    let registry = CardRegistry::with_all_cards();
    let seats = || vec![SeatKind::Human, SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())];
    let through_json = |save: TableSave| -> TableSave {
        serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap()
    };
    let same_draft = |a: &Lobby, b: &Lobby| {
        for seat in 0..POD {
            let (va, vb) = (jv(a, seat), jv(b, seat));
            for key in ["phase", "pack", "pool", "picks", "deck", "standings", "pairings"] {
                assert_eq!(va[key], vb[key], "seat {seat}'s {key} after the resume");
            }
            for (sa, sb) in va["seats"].as_array().unwrap().iter().zip(vb["seats"].as_array().unwrap()) {
                assert_eq!((&sa["name"], &sa["picks"], &sa["status"]), (&sb["name"], &sb["picks"], &sb["status"]));
            }
        }
    };

    // Mid-draft.
    let (mut a, _) = lobby(seats());
    a.connected(0);
    a.connected(1);
    a.set_name(0, "Ann");
    draft_to_the_end(&mut a, Some(20));
    a.set_ai_notes(2, Some("taking blue".into()));
    let (mut b, _) = lobby(seats());
    let to_play = b.restore(&through_json(a.snapshot())).unwrap();
    assert!(to_play.is_empty());
    same_draft(&a, &b);
    assert_eq!(b.human_keys(), a.human_keys(), "every join link still works");
    assert_eq!(b.ai_notes(2).as_deref(), Some("taking blue"));
    assert_eq!(b.phase(), Phase::Drafting);
    assert!(b.take_events().is_empty(), "the replay says nothing on the terminal");

    // Both carry on alike: to the end of the draft, the decks, round 1 and
    // half of round 2.
    for l in [&mut a, &mut b] {
        draft_to_the_end(l, None);
        for seat in 0..POD {
            let pool: Vec<String> = jv(l, seat)["pool"].as_array().unwrap().iter()
                .map(|c| c["name"].as_str().unwrap().to_string()).collect();
            let deck = fallback_deck(&pool, &registry);
            if seat < 2 {
                l.submit_deck(seat, &deck.maindeck, &deck.lands, &[]).unwrap();
                l.ready(seat).unwrap();
            } else {
                l.ai_deck(seat, &mtg_draft_runner::deck::DeckBuildResult { deck, attempts: vec![], retries: 0, fallback: false });
            }
        }
    }
    same_draft(&a, &b);
    let game = |winner: usize| GameOutcome { winner: Some(winner), turns: 9, game_log: vec![], stalled_seat: None, abandoned: false };
    let result = |p: usize, q: usize, wp: usize, wq: usize| MatchResult {
        player_a: p, player_b: q, wins_a: wp, wins_b: wq,
        games: (0..wp).map(|_| game(p)).chain((0..wq).map(|_| game(q))).collect(),
    };
    assert_eq!(a.begin_tournament(), vec![(1, 0, 1), (1, 2, 3)]);
    for (r, p, q) in [(1, 0, 1), (1, 2, 3)] {
        a.match_started(r, p, q, [None, None]);
    }
    a.match_finished(1, 0, 1, result(0, 1, 2, 1));
    let round2 = a.match_finished(1, 2, 3, result(2, 3, 0, 2));
    assert_eq!(round2, vec![(2, 0, 3), (2, 1, 2)]);
    a.match_started(2, 0, 3, [None, None]);
    a.match_started(2, 1, 2, [None, None]);
    a.match_finished(2, 0, 3, result(0, 3, 2, 0));

    // Mid-tournament: the finished matches are recorded, the one in
    // progress is to play again.
    let (mut c, _) = lobby(seats());
    let to_play = c.restore(&through_json(a.snapshot())).unwrap();
    assert_eq!(c.phase(), Phase::Playing);
    assert_eq!(to_play, vec![(2, 1, 2)], "the match that was in progress");
    assert_eq!(c.begin_tournament(), vec![(2, 1, 2)], "and what the tournament thread is handed");
    for seat in 0..POD {
        let (va, vc) = (jv(&a, seat), jv(&c, seat));
        assert_eq!(va["standings"], vc["standings"], "seat {seat}");
        assert_eq!(va["deck"], vc["deck"], "seat {seat}");
    }
    c.match_started(2, 1, 2, [None, None]);
    assert!(c.match_finished(2, 1, 2, result(1, 2, 1, 2)).is_empty());
    assert_eq!(c.phase(), Phase::Done, "the table finishes from where it stopped");

    // Another table's snapshot is refused, not half-applied.
    let mut other = a.snapshot();
    other.seed += 1;
    let (mut d, _) = lobby(seats());
    assert!(d.restore(&other).unwrap_err().contains("another table"));
}

/// Between games of a match (MTR 2.2, 2.4): the coin flip's winner chooses
/// to play or draw before game 1 and nobody sideboards; from game 2 the
/// last game's loser chooses and either seat may sideboard from its pool.
/// Only a deck the pool makes is taken, only the chooser chooses, a seat
/// that is ready is done, and the host's sideboard timer ends it. A deck
/// sideboarded out and back in is no change.
#[test]
fn between_games_a_person_chooses_and_sideboards_and_the_clock_ends_it() {
    let registry = CardRegistry::with_all_cards();
    let (mut l, _) = lobby_clocked(
        vec![SeatKind::Human, SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())],
        None, None, Some(60));
    l.connected(0);
    l.connected(1);
    draft_to_the_end(&mut l, None);
    for seat in 0..POD {
        let deck = fallback_deck(&l.pool(seat), &registry);
        if seat < 2 {
            l.submit_deck(seat, &deck.maindeck, &deck.lands, &[]).unwrap();
            l.ready(seat).unwrap();
        } else {
            l.ai_deck(seat, &mtg_draft_runner::deck::DeckBuildResult { deck, attempts: vec![], retries: 0, fallback: false });
        }
    }
    assert_eq!(l.begin_tournament(), vec![(1, 0, 1), (1, 2, 3)]);
    l.match_started(1, 0, 1, [None, None]);
    l.take_events();
    let between = |l: &Lobby, seat: usize| jv(l, seat)["matches"][0]["between"].clone();

    // Game 1: seat 0 won the flip. Seat 1 has nothing to do.
    l.open_between(1, 0, 1, 1, 0, [vec![], vec![]], [None, None]);
    let b0 = between(&l, 0);
    assert_eq!((b0["you_choose"].as_bool(), b0["sideboarding"].as_bool(), b0["ready"].as_bool()), (Some(true), Some(false), Some(false)));
    assert!(b0["deadline_ms"].as_u64().is_some_and(|ms| ms > 50_000), "the host's clock: {b0}");
    assert_eq!(between(&l, 1)["ready"], true, "the flip's loser waits on nothing");
    assert!(l.take_events().iter().any(|e| e.contains("before game 1: choosing play or draw")));
    let deck = l.match_deck(1, 0, 1, 0).unwrap();
    assert_eq!(l.sideboard(0, &deck.maindeck, &deck.lands).unwrap_err(), "no sideboarding before game 1");
    assert_eq!(l.play_draw(1, true).unwrap_err(), "the choice to play or draw is your opponent's");
    l.play_draw(0, false).unwrap();
    assert_eq!(between(&l, 0)["play_first"], false);
    assert!(between(&l, 1)["play_first"].is_null(), "the opponent is not told until the game starts");
    assert!(l.take_between(1, 0, 1).is_none(), "seat 0 is still deciding");
    l.sideboard_ready(0).unwrap();
    let out = l.take_between(1, 0, 1).unwrap();
    assert_eq!(out.chooser_plays, Some(false));
    assert!(out.decks.iter().all(Option::is_none));
    assert!(l.take_events().iter().any(|e| e == "round 1: game 1: seat 0 chose to draw first"));
    assert_eq!(l.play_draw(0, true).unwrap_err(), "you are not between games");

    // Game 2: seat 0 lost game 1 and chooses; both may sideboard.
    let game = |winner: usize| GameOutcome { winner: Some(winner), turns: 9, game_log: vec![], stalled_seat: None, abandoned: false };
    l.game_finished(1, 0, 1, &game(1));
    let played = l.match_deck(1, 0, 1, 0).unwrap();
    l.open_between(1, 0, 1, 2, 0, [vec![], vec!["Brimstone Volley".into()]], [None, None]);
    let b0 = between(&l, 0);
    assert_eq!((b0["sideboarding"].as_bool(), b0["you_choose"].as_bool()), (Some(true), Some(true)));
    assert_eq!(b0["opponent_showed"][0]["name"], "Brimstone Volley");
    assert_eq!(between(&l, 1)["ready"], false, "seat 1 may sideboard");
    let short = &played.maindeck[..10];
    assert!(l.sideboard(0, short, &HashMap::new()).unwrap_err().contains("40"), "a short deck is not taken");
    // One card out for one from the sideboard.
    let pool = l.pool(0);
    let mut spare = pool.clone();
    for c in &played.maindeck {
        if let Some(i) = spare.iter().position(|p| mtg_draft::front_face(p) == mtg_draft::front_face(c)) {
            spare.remove(i);
        }
    }
    let swap_in = spare.first().expect("a sideboard card").clone();
    let mut swapped = played.maindeck.clone();
    let out_card = swapped.remove(0);
    swapped.push(swap_in.clone());
    l.sideboard(0, &swapped, &played.lands).unwrap();
    assert!(between(&l, 0)["deck"]["main"].as_array().unwrap().iter().any(|c| c.as_str() == Some(swap_in.as_str())));
    l.sideboard_ready(0).unwrap();
    assert_eq!(l.sideboard(0, &played.maindeck, &played.lands).unwrap_err(), "you are ready for the next game");
    assert!(l.take_between(1, 0, 1).is_none(), "seat 1 is sideboarding");
    // Seat 1 takes a card out and puts it back: no change.
    let theirs = l.match_deck(1, 0, 1, 1).unwrap();
    l.sideboard(1, &theirs.maindeck[1..], &{ let mut lands = theirs.lands.clone(); *lands.entry("Plains".into()).or_insert(0) += 1; lands }).unwrap();
    l.sideboard(1, &theirs.maindeck, &theirs.lands).unwrap();
    l.sideboard_ready(1).unwrap();
    l.take_events();
    let out = l.take_between(1, 0, 1).unwrap();
    let mine = out.decks[0].as_ref().expect("seat 0 sideboarded");
    assert!(mine.maindeck.contains(&swap_in) && mine.maindeck.iter().filter(|c| **c == out_card).count()
        < played.maindeck.iter().filter(|c| **c == out_card).count());
    assert!(out.decks[1].is_none(), "out and back in is the same deck");
    assert_eq!(out.chooser_plays, None, "never chosen: the game plays the chooser first");
    let events = l.take_events();
    assert!(events.iter().any(|e| e.starts_with("round 1: seat 0 sideboarded for game 2")), "{events:?}");
    assert!(!events.iter().any(|e| e.starts_with("round 1: seat 1 sideboarded")), "{events:?}");
    assert_eq!(l.match_deck(1, 0, 1, 0).unwrap().maindeck, mine.maindeck, "game 3 starts from it");

    // Game 3: the clock runs out with seat 1 still deciding. Seat 0 went
    // back to its registered deck, which is a change from game 2's.
    l.game_finished(1, 0, 1, &game(0));
    l.open_between(1, 0, 1, 3, 1, [vec![], vec![]], [None, None]);
    l.sideboard(0, &played.maindeck, &played.lands).unwrap();
    assert!(!l.tick(std::time::Instant::now()), "not yet");
    assert!(l.tick(std::time::Instant::now() + std::time::Duration::from_secs(61)));
    assert!(l.take_events().iter().any(|e| e == "round 1: seat 0 vs seat 1: the sideboard timer ran out before game 3"));
    let out = l.take_between(1, 0, 1).unwrap();
    assert_eq!(out.decks[0].as_ref().unwrap().maindeck, played.maindeck);
    assert_eq!(out.chooser_plays, None);
}

/// A seat's draft review: every pack it was handed, as it was, and the
/// card it took — 42 picks, each card in its own pack, the packs shrinking
/// by one a pick — and the same after a `--resume`, which replays the
/// picks rather than making them.
#[test]
fn the_review_is_every_pack_a_seat_was_handed() {
    let seats = || vec![SeatKind::Human, SeatKind::Human, SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())];
    let (mut a, _) = lobby(seats());
    a.connected(0);
    a.connected(1);
    draft_to_the_end(&mut a, None);
    let review = serde_json::to_value(a.review(0)).unwrap();
    assert_eq!(review["type"], "review");
    let picks = review["picks"].as_array().unwrap();
    assert_eq!(picks.len(), 3 * PACK, "every pick of the seat's draft");
    for (i, p) in picks.iter().enumerate() {
        let names: Vec<&str> = p["pack"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
        assert!(names.contains(&p["card"].as_str().unwrap()), "pick {i}: {p}");
        assert_eq!(names.len(), PACK - i % PACK, "pick {i}: the pack shrinks a card a pick");
        assert_eq!((p["round"].as_u64(), p["pick"].as_u64()), (Some((i / PACK + 1) as u64), Some((i % PACK + 1) as u64)));
    }
    let pool = own_pool(&a, 0);
    assert!(picks.iter().all(|p| pool.contains(&mtg_draft::front_face(p["card"].as_str().unwrap()).to_string())));
    let (mut b, _) = lobby(seats());
    b.restore(&serde_json::from_str(&serde_json::to_string(&a.snapshot()).unwrap()).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(b.review(0)).unwrap(), review, "a resumed table reviews the same draft");
}
