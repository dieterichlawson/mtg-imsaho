//! #820: the hosted table pairs a round, and ends the tournament, on the
//! thread of whichever match finished last, so round 2's BYE and the FINAL
//! STANDINGS were filed under "seat 0 v 1", the thread of a round-1 match.
//! A reader filtering the draft log by thread missed them, or found them
//! inside a match. They are the tournament's records.
//!
//! Its own binary because the log is process-global.

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

const PACK: usize = 14;

#[test]
fn every_record_is_filed_under_whom_it_is_about() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let dir = std::env::temp_dir().join(format!("mtg-tournament-thread-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log_path = dir.join("draft.log");

    let mut set_data = SetData::load(&root.join("data/sets/isd.json")).unwrap();
    let registry = Arc::new(CardRegistry::with_all_cards());
    set_data.filter_implemented(&registry);
    let mut names = set_data.all_card_names();
    names.sort();
    let mut it = names.into_iter();
    let packs: Vec<Vec<BoosterPack>> = (0..3).map(|_| (0..3).map(|_| {
        let cards: Vec<String> = it.by_ref().take(PACK).collect();
        BoosterPack { commons: cards[..PACK - 2].to_vec(), uncommons: vec![],
            rare: cards[PACK - 2].clone(), dfc: cards[PACK - 1].clone(), foil: None }
    }).collect()).collect();
    let lines = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);
    let seats = vec![SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into()), SeatKind::Ai("cc".into())];
    let config = LobbyConfig {
        set_code: "isd".into(), set_name: set_data.set_name.clone(), seats,
        best_of: 1, seed: 1, guide_path: None, pick_seconds: None, build_seconds: None, out_dir: None,
    };
    let mut lobby = Lobby::new(config, &packs, &set_data, registry.clone(), &lines,
        DraftLogger::new(&log_path)).unwrap();

    // Draft and build, on the thread the server names "tournament".
    let to_play = std::thread::scope(|s| std::thread::Builder::new().name("tournament".into())
        .spawn_scoped(s, || {
            lobby.start();
            loop {
                let seats = lobby.seats_with_a_pack();
                if seats.is_empty() { break; }
                for seat in seats {
                    let v = serde_json::to_value(lobby.view(seat)).unwrap();
                    let Some(id) = v["pack"]["id"].as_u64() else { continue };
                    lobby.ai_pick(seat, id as usize, 0, "prompt", "{\"pick\":0}", false).unwrap();
                }
            }
            for seat in 0..3 {
                let v = serde_json::to_value(lobby.view(seat)).unwrap();
                let pool: Vec<String> = v["pool"].as_array().unwrap().iter()
                    .map(|c| c["name"].as_str().unwrap().to_string()).collect();
                lobby.ai_deck(seat, &mtg_draft_runner::deck::DeckBuildResult {
                    deck: fallback_deck(&pool, &registry), attempts: vec![], retries: 0, fallback: false,
                });
            }
            lobby.begin_tournament()
        }).unwrap().join().unwrap());

    // Each match ends on its own worker thread, as the server runs it.
    let mut to_play = to_play;
    while let Some((round, a, b)) = to_play.pop() {
        let next = std::thread::scope(|s| std::thread::Builder::new().name(format!("seat {a} v {b}"))
            .spawn_scoped(s, || {
                lobby.match_started(round, a, b, [None, None]);
                let game = GameOutcome { winner: Some(a), turns: 9, game_log: vec![], stalled_seat: None, abandoned: false };
                lobby.match_finished(round, a, b, MatchResult {
                    player_a: a, player_b: b, wins_a: 1, wins_b: 0, games: vec![game],
                })
            }).unwrap().join().unwrap());
        to_play.extend(next);
    }
    assert_eq!(lobby.phase(), Phase::Done);

    let logged = std::fs::read_to_string(&log_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let thread_of = |label: &str| -> Vec<String> {
        logged.lines().filter(|l| l.split('\t').nth(4).is_some_and(|f| f.starts_with(label)))
            .map(|l| l.split('\t').nth(2).unwrap_or("").to_string()).collect()
    };
    let byes = thread_of("BYE");
    assert_eq!(byes.len(), 2, "one bye a round:\n{logged}");
    assert!(byes.iter().all(|t| t == "tournament"), "a bye filed under a match: {byes:?}");
    assert_eq!(thread_of("FINAL STANDINGS"), vec!["tournament".to_string()]);
    // A match's own result stays the match's.
    assert!(thread_of("MATCH").iter().all(|t| t.starts_with("seat ")), "{:?}", thread_of("MATCH"));

    // The same, a phase earlier (the review of the fix): the draft's
    // sections are the table's, whichever pick opened them, and a seat's
    // picks, pool and deck are that seat's, whoever picked for it. Here
    // every pick ran on the "tournament" thread.
    let records: Vec<(String, String, String)> = logged.lines()
        .filter_map(|l| { let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| (f[2].to_string(), f[4].to_string(), f.get(5).unwrap_or(&"").to_string())) })
        .collect();
    for (thread, label, content) in &records {
        if label == "SECTION" && (content.contains("DRAFT") || content.contains("DECK BUILDING"))
            || label.starts_with("--- Pack")
        {
            assert_eq!(thread, "table", "{label} {content}");
        }
        // (The BOOSTER PACKS dump is the table's setup, written once as
        // it is made, as the runner writes it.)
        if let Some((seat, what)) = label.strip_prefix("[Seat ").and_then(|r| r.split_once("] ")) {
            if ["PICK", "POOL", "DECK", "WARN"].iter().any(|w| what.starts_with(w)) {
                assert_eq!(thread, &format!("seat {seat}"), "{label}");
            }
        }
    }
    assert!(records.iter().any(|(_, l, _)| l.starts_with("--- Pack 3")), "the packs are logged");
    assert!(records.iter().any(|(_, l, _)| l.starts_with("[Seat 2] PICK")), "the picks are logged");
}
