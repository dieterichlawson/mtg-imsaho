//! A resumed table says what its AI seats were asked under (#948, #949).
//!
//! The batch runner's snapshot recorded each seat's guide (#579); the
//! hosted table's recorded none, nor `MTG_LLM_HISTORY` or the answer
//! format, and `mtg-draft-server --resume` read `--guide` afresh: a table
//! started with a guide and resumed without one drafted the rest under
//! another system prompt, and nothing said so.
#![cfg(unix)]

mod lobby_support;

use std::time::{Duration, Instant};

use lobby_support::Server;
use mtg_draft_runner::lobby::{SeatKind, StillAsked, TablePrompt};

#[test]
fn a_table_resumed_without_its_guide_says_so() {
    if !lobby_support::python3_available() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let files = std::env::temp_dir().join(format!("mtg-lobby-guide-resume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&files);
    std::fs::create_dir_all(&files).unwrap();
    let guide = files.join("guide.md");
    std::fs::write(&guide, "Take red cards.\n").unwrap();
    let (guide, save) = (guide.to_str().unwrap().to_string(), files.join("table.save.json"));
    let save = save.to_str().unwrap().to_string();

    // The host's terminal seat waits; the AI seat takes its first pick.
    let first = Server::start("guide-resume-a", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--guide", &guide, "--save", &save]);
    let deadline = Instant::now() + Duration::from_secs(60);
    let saved: serde_json::Value = loop {
        let text = std::fs::read_to_string(&save).unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v["picks"].as_array().is_some_and(|p| !p.is_empty()) {
                break v;
            }
        }
        assert!(Instant::now() < deadline, "the AI seat's pick never reached the snapshot:\n{}", first.stderr());
        std::thread::sleep(Duration::from_millis(100));
    };
    drop(first);
    assert_eq!(saved["prompt"]["guide_path"].as_str(), Some(guide.as_str()), "the guide is recorded: {saved}");
    assert!(saved["prompt"]["guide_hash"].as_str().is_some_and(|h| h.len() == 16), "and its contents: {saved}");

    let resumed = files.join("resumed.save.json");
    let second = Server::start("guide-resume-b", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--resume", &save, "--save", resumed.to_str().unwrap()]);
    let said = format!("made under guide {guide}, and the rest of this draft will be made under no guide");
    let stderr = second.wait_for_stderr("WARN --resume", Duration::from_secs(30));
    assert!(stderr.contains(&said) && stderr.contains("its system prompt is not the live one"),
        "the terminal names both, in mtg-draft-replay's words:\n{stderr}");
    let log = second.wait_for_log("WARN --resume", Duration::from_secs(30));
    assert!(log.contains(&said), "and the log, which outlives it:\n{log}");
    drop(second);

    // A snapshot from before the record says what it cannot know.
    let mut old = saved.clone();
    old.as_object_mut().unwrap().remove("prompt");
    let old_path = files.join("old.save.json");
    std::fs::write(&old_path, old.to_string()).unwrap();
    let third = Server::start("guide-resume-c", "cli,ai:cc",
        &["--quiet", "--pick-seconds", "600", "--resume", old_path.to_str().unwrap(),
          "--save", files.join("old-resumed.save.json").to_str().unwrap()]);
    let stderr = third.wait_for_stderr("predates the guide and settings record", Duration::from_secs(30));
    assert!(!stderr.contains("WARN --resume"), "{stderr}");
    drop(third);
    let _ = std::fs::remove_dir_all(&files);
}

/// The settings besides the guide, said in `mtg-draft-replay`'s words; the
/// game settings only once a game was played; and a table with no AI seat
/// has nothing to say about any of them.
#[test]
fn a_table_prompt_names_each_setting_that_moved() {
    let ai = [SeatKind::Human, SeatKind::Ai("cc".into())];
    let live = TablePrompt::live(&ai, None, None, "set-a");
    let mut was = live.clone();
    was.settings.history = Some(live.settings.history.unwrap() + 2);
    was.settings.text_answers = live.settings.text_answers.map(|t| !t);
    was.settings.pass_until = live.settings.pass_until.map(|p| !p);
    was.settings.reference = Some("set-b".into());
    let said = was.unlike(&live, false, StillAsked::Draft);
    assert_eq!(said.len(), 3, "the draft's three, not the games' before a game was played: {said:?}");
    assert!(said[0].contains("MTG_LLM_HISTORY="), "{said:?}");
    assert!(said[1].contains("MTG_CLAUDE_CODE_DRAFT_ANSWERS="), "{said:?}");
    assert!(said[2].contains("not the one it was asked about"), "{said:?}");
    let said = was.unlike(&live, true, StillAsked::Draft);
    assert_eq!(said.len(), 4, "{said:?}");
    assert!(said[3].contains("MTG_LLM_PASS_UNTIL="), "{said:?}");
    assert!(live.unlike(&live, true, StillAsked::Draft).is_empty(), "the same settings say nothing");

    let people = [SeatKind::Human, SeatKind::Human];
    let mut was = TablePrompt::live(&people, Some("g.md"), Some("0123456789abcdef"), "set-b");
    was.settings.history = Some(5);
    let said = was.unlike(&TablePrompt::live(&people, None, None, "set-a"), true, StillAsked::Draft);
    assert!(said.is_empty(), "no AI seat is asked anything: {said:?}");
}

/// #1014: a resume after every pick and deck was made, under a guide edited
/// since, warned "the rest of this draft will be made under" the new guide
/// and "this draft is a mixture of the two", when nothing but sideboarding
/// (best-of-3) or nothing at all (best-of-1) was still to be asked. The
/// warning says what is still asked; with nothing, there is no warning.
#[test]
fn a_changed_guide_after_the_draft_is_not_called_a_mixed_draft() {
    let ai = [SeatKind::Human, SeatKind::Ai("cc".into())];
    let was = TablePrompt::live(&ai, Some("g.md"), Some("0123456789abcdef"), "set-a");
    let live = TablePrompt::live(&ai, Some("g.md"), Some("fedcba9876543210"), "set-a");

    assert_eq!(StillAsked::after(false, true, 3), StillAsked::Draft);
    let said = was.unlike(&live, false, StillAsked::Draft);
    assert!(said.len() == 1 && said[0].contains("mixture") && said[0].contains("g.md as it was"), "{said:?}");

    assert_eq!(StillAsked::after(true, true, 3), StillAsked::Sideboarding);
    let said = was.unlike(&live, false, StillAsked::Sideboarding);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("picks, decks and any finished matches were made under guide g.md as it was")
            && said[0].contains("only the games still to play and the sideboarding between them are asked"), "{said:?}");
    assert!(!said[0].contains("mixture") && !said[0].contains("rest of this draft"), "{said:?}");

    assert_eq!(StillAsked::after(true, false, 1), StillAsked::Nothing);
    assert_eq!(StillAsked::after(true, false, 3), StillAsked::Nothing);
    assert!(was.unlike(&live, false, StillAsked::Nothing).is_empty(), "nothing is asked under the new guide");

    // Nor under another draft setting (#1014's review): "the rest of this
    // draft" is no draft at all.
    let mut was = live.clone();
    was.settings.history = Some(live.settings.history.unwrap() + 2);
    assert_eq!(was.unlike(&live, false, StillAsked::Sideboarding).len(), 1);
    assert!(was.unlike(&live, false, StillAsked::Nothing).is_empty());
    // The history is in a game seat's system prompt too (#1015's review).
    let said = was.unlike(&live, false, StillAsked::Games);
    assert!(said.len() == 1 && said[0].contains("the games still to play are asked under"), "{said:?}");
    // A game setting, once every match is played, reaches nothing.
    let mut was = live.clone();
    was.settings.pass_until = live.settings.pass_until.map(|on| !on);
    assert_eq!(was.unlike(&live, true, StillAsked::Games).len(), 1);
    assert!(was.unlike(&live, true, StillAsked::Nothing).is_empty(), "no game is left to play under it");
}

/// #1015: a snapshot with every deck and a best-of-1 match still to play
/// was taken as "nothing more is asked", and the note said the new guide
/// "is not used" while every game was played under it: a guided seat's
/// game system prompt carries its guide. The games are what is still
/// asked, and the change is said.
#[test]
fn a_changed_guide_before_the_matches_reaches_the_games() {
    let ai = [SeatKind::Human, SeatKind::Ai("cc".into())];
    let was = TablePrompt::live(&ai, Some("g.md"), Some("0123456789abcdef"), "set-a");
    let live = TablePrompt::live(&ai, Some("g.md"), Some("fedcba9876543210"), "set-a");

    assert_eq!(StillAsked::after(true, true, 1), StillAsked::Games);
    let said = was.unlike(&live, false, StillAsked::Games);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("only the games still to play are asked, and under guide g.md as it is now"), "{said:?}");
    assert!(!said[0].contains("mixture") && !said[0].contains("sideboarding"), "{said:?}");

    // Which is what the records say: no match played is a match to play,
    // and a finished two-seat best-of-1 is none.
    let result = |a: usize, b: usize| mtg_draft::tournament::MatchResult {
        player_a: a, player_b: b, wins_a: 1, wins_b: 0,
        games: vec![serde_json::from_value(serde_json::json!({ "winner": a, "turns": 5 })).unwrap()],
    };
    assert!(mtg_draft_runner::game::matches_remain(&[], 2, 1));
    let mut t = mtg_draft::tournament::Tournament::new(2, mtg_draft::tournament::TournamentConfig { best_of: 1 });
    let mut records = Vec::new();
    let mut round = 1;
    while !t.is_complete() {
        let pairings = t.generate_pairings();
        let results: Vec<_> = pairings.iter().filter(|&&(_, b)| b != mtg_draft::tournament::BYE)
            .map(|&(a, b)| result(a, b)).collect();
        for r in &results {
            records.push((round, r.clone()));
        }
        t.record_round(pairings, results);
        round += 1;
    }
    let all: Vec<(usize, &mtg_draft::tournament::MatchResult)> = records.iter().map(|(r, m)| (*r, m)).collect();
    assert!(!mtg_draft_runner::game::matches_remain(&all, 2, 1), "every match is played");
    assert!(mtg_draft_runner::game::matches_remain(&all[..all.len() - 1], 2, 1), "the last match is still to play");
}

/// #1015's review: what is still to play is a seat's own. A snapshot is
/// written after each match, so the last round can be half played: the
/// seats whose match is recorded, and a seat with the last round's bye,
/// have nothing left, and their guide is no mixture.
#[test]
fn a_seat_whose_last_match_is_played_has_nothing_left() {
    use mtg_draft::tournament::{MatchResult, Tournament, TournamentConfig, BYE};
    use mtg_draft_runner::game::seats_with_matches_left;
    // Every match of every round but the last, and the first match of the
    // last round when `half`.
    let records = |players: usize, half: bool| {
        let mut t = Tournament::new(players, TournamentConfig { best_of: 1 });
        let mut out: Vec<(usize, MatchResult)> = Vec::new();
        let mut last = Vec::new();
        while !t.is_complete() {
            let round = t.rounds.len() + 1;
            let pairings = t.generate_pairings();
            let results: Vec<MatchResult> = pairings.iter().filter(|&&(_, b)| b != BYE).map(|&(a, b)| MatchResult {
                player_a: a, player_b: b, wins_a: 1, wins_b: 0,
                games: vec![serde_json::from_value(serde_json::json!({ "winner": a, "turns": 5 })).unwrap()],
            }).collect();
            if round == t.total_rounds() {
                last.clone_from(&pairings);
                if half {
                    out.push((round, results[0].clone()));
                }
                break;
            }
            out.extend(results.iter().map(|r| (round, r.clone())));
            t.record_round(pairings, results);
        }
        (out, last)
    };
    fn as_refs(r: &[(usize, MatchResult)]) -> Vec<(usize, &MatchResult)> {
        r.iter().map(|(n, m)| (*n, m)).collect()
    }

    let (four, last) = records(4, true);
    let played = four.last().unwrap().1.clone();
    let left = seats_with_matches_left(&as_refs(&four), 4, 1);
    for seat in 0..4 {
        let done = seat == played.player_a || seat == played.player_b;
        assert_eq!(left[seat], !done, "seat {seat}, last round {last:?}, played {}-{}", played.player_a, played.player_b);
    }
    assert_eq!(seats_with_matches_left(&[], 4, 1), vec![true; 4], "before the last round every seat plays");

    let (three, last) = records(3, false);
    let bye = last.iter().find(|&&(_, b)| b == BYE).map(|&(a, _)| a).expect("three seats have a bye");
    let left = seats_with_matches_left(&as_refs(&three), 3, 1);
    for seat in 0..3 {
        assert_eq!(left[seat], seat != bye, "seat {seat}, the bye is seat {bye}");
    }
}

/// #999: the resume note said "a match that was in progress is played
/// again from its first game" on every resume, a table stopped while
/// building included. It names the round's unfinished matches only once
/// every deck is final, and says nothing of a match otherwise.
#[test]
fn a_resume_names_the_matches_it_plays_again_and_only_those() {
    use mtg_draft::tournament::{MatchResult, Tournament, TournamentConfig, BYE};
    use mtg_draft_runner::lobby::{SavedDeck, SavedMatch, SavedSeat, TableSave};
    let seat = |kind: &str| SavedSeat { kind: kind.to_string(), name: kind.to_string(), key: Some("k".to_string()),
        joined: true, kicked: false, gave_up: false, notes: None };
    let deck = |seat: usize, ready: bool| SavedDeck { seat, main: Vec::new(), lands: std::collections::HashMap::new(),
        sideboard: Vec::new(), fallback: false, ready };
    let mut save = TableSave { version: 1, set: "ISD".to_string(), seed: 5101, best_of: 1,
        seats: vec![seat("human"), seat("ai:x"), seat("ai:x")], picks: Vec::new(),
        decks: vec![deck(1, true), deck(2, true)], matches: Vec::new(), deal: None, prompt: None };
    // Still building: the human seat has no deck.
    assert_eq!(save.resumed_matches_clause(), None);
    // A deck sent and not final is still building too.
    save.decks.push(deck(0, false));
    assert_eq!(save.resumed_matches_clause(), None);
    // Every deck final: round 1's one match (three seats, one bye) is named.
    save.decks[2].ready = true;
    let t = Tournament::new(3, TournamentConfig { best_of: 1 });
    let pairings = t.generate_pairings();
    let &(a, b) = pairings.iter().find(|&&(_, b)| b != BYE).unwrap();
    assert_eq!(save.resumed_matches_clause().as_deref(),
        Some(format!("round 1's match still to play, seat {a} v seat {b}, is played from its first game").as_str()));
    // A kicked seat's match is a forfeit, not played from its first game
    // (#1029's review: the seats are not told it is either).
    let mut forfeit = save.clone();
    forfeit.seats[a].kicked = true;
    assert_eq!(forfeit.resumed_matches_clause(), None, "the one match left is a forfeit");
    // Every match played: nothing is played again.
    let mut t = t;
    let mut round = 1;
    while !t.is_complete() {
        let pairings = t.generate_pairings();
        let results: Vec<MatchResult> = pairings.iter().filter(|&&(_, b)| b != BYE).map(|&(a, b)| MatchResult {
            player_a: a, player_b: b, wins_a: 1, wins_b: 0,
            games: vec![serde_json::from_value(serde_json::json!({ "winner": a, "turns": 5 })).unwrap()],
        }).collect();
        for r in &results {
            save.matches.push(SavedMatch { round, a: r.player_a, b: r.player_b, result: r.clone() });
        }
        t.record_round(pairings, results);
        round += 1;
    }
    assert_eq!(save.resumed_matches_clause(), None, "every match is played");
}
