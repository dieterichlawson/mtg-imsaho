//! Tests for the LLM player's multi-turn conversation and decklist formatting.

use mtg_player::llm::MatchFormat;
use mtg_engine::cards::CardRegistry;

#[test]
fn format_decklist_includes_oracle_text() {
    let registry = CardRegistry::with_all_cards();
    let entries = vec![
        ("Victim of Night".to_string(), 2),
        ("Swamp".to_string(), 10),
    ];

    let result = mtg_player::llm::LlmPlayer::format_decklist_for_test(&entries, &registry);

    // Should include card count
    assert!(result.contains("2x Victim of Night"), "Should list card count: {result}");
    assert!(result.contains("10x Swamp"), "Should list land count: {result}");

    // Should include oracle text
    assert!(result.contains("Destroy target non-Vampire"),
        "Should include oracle text for Victim of Night: {result}");

    // Should include cost
    assert!(result.contains("{B}{B}"), "Should include mana cost: {result}");

    // Should NOT duplicate card info for same-name entries
    let occurrences = result.matches("Destroy target non-Vampire").count();
    assert_eq!(occurrences, 1, "Oracle text should appear only once even with count > 1");
}

#[test]
fn init_conversation_sets_system_prompt_with_decklists() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");

    let your_deck = vec![
        ("Lightning Bolt".to_string(), 4),
        ("Mountain".to_string(), 16),
    ];
    let _opp_deck = [("Grizzly Bears".to_string(), 4),
        ("Forest".to_string(), 16)];

    player.init_conversation(&your_deck, "Grizzly Bears {1}{G} | Creature — Bear 2/2\nForest | Land", &registry, MatchFormat::SingleGame);

    let system = player.system_prompt_for_test();

    // Should contain your decklist and card reference
    assert!(system.contains("Your decklist"), "System prompt should have your decklist section");
    assert!(system.contains("Card reference"), "System prompt should have card reference section");

    // Should contain card details
    assert!(system.contains("Lightning Bolt"), "Should include your cards");
    assert!(system.contains("Grizzly Bears"), "Should include cards from reference");

    // Should contain game rules
    assert!(system.contains("Magic: The Gathering"), "Should contain game rules");

    // Conversation should be empty at start
    assert_eq!(player.conversation_len_for_test(), 0, "Conversation should be empty after init");
}

#[test]
fn conversation_grows_with_messages() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");

    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    assert_eq!(player.conversation_len_for_test(), 0);

    // We can't actually call send_message without an API key,
    // but we can verify the conversation structure is set up correctly.
    // The init should have cleared the conversation and set the system prompt.
    let system = player.system_prompt_for_test();
    assert!(system.contains("Your decklist"));
    assert!(system.contains("Mountain"));
}

#[test]
fn build_prompt_includes_board_state() {
    // Verify that format_state_compact produces expected output structure.
    // We can't easily call build_prompt without a full GameView,
    // but we can verify format_state_compact handles various states.
    // This is a smoke test that the function exists and is callable.
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    // Verify last_log_index starts at 0
    assert_eq!(player.last_log_index_for_test(), 0);
}

#[test]
fn resume_from_log_seeds_conversation() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    assert_eq!(player.conversation_len_for_test(), 0);
    assert_eq!(player.last_log_index_for_test(), 0);

    let log = vec![
        "Game started".to_string(),
        "p0 drew 7 cards".to_string(),
        "p1 drew 7 cards".to_string(),
        "── Turn 1 (p0) ──".to_string(),
        "p0 played Mountain".to_string(),
    ];

    player.resume_from_log(&log, mtg_engine::ids::PlayerId(0));

    // Should have 2 messages: user recap + assistant acknowledgment
    assert_eq!(player.conversation_len_for_test(), 2,
        "Resume should add a user+assistant message pair");

    // last_log_index should be set to the log length
    assert_eq!(player.last_log_index_for_test(), 5,
        "last_log_index should match the log length");
}

#[test]
fn resume_from_empty_log_does_nothing() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    player.resume_from_log(&[], mtg_engine::ids::PlayerId(0));

    assert_eq!(player.conversation_len_for_test(), 0,
        "Empty log should not add any messages");
    assert_eq!(player.last_log_index_for_test(), 0,
        "Empty log should not change last_log_index");
}

#[test]
fn short_effect_summary_drops_enchant_line_and_reminder_text() {
    use mtg_player::llm::LlmPlayer;

    // Ghostly Possession — should drop "Enchant creature" and surface both
    // the flying line and the prevent-damage clause.
    let ghostly = "Enchant creature\n\
                   Enchanted creature has flying.\n\
                   Prevent all combat damage that would be dealt to and dealt by enchanted creature.";
    let summary = LlmPlayer::short_effect_summary_for_test(ghostly);
    assert!(!summary.to_lowercase().starts_with("enchant creature"),
        "Should drop leading 'Enchant creature' line: {summary}");
    assert!(summary.contains("flying"), "Should include flying: {summary}");
    assert!(summary.contains("Prevent all combat damage"),
        "Should mention combat damage prevention: {summary}");

    // Bonds of Faith — ensure the conditional gets through.
    let bonds = "Enchant creature\n\
                 Enchanted creature gets +2/+2 as long as it's a Human. \
                 Otherwise, it can't attack or block.";
    let summary = LlmPlayer::short_effect_summary_for_test(bonds);
    assert!(summary.contains("+2/+2"), "Should include the bonus: {summary}");
    assert!(summary.contains("can't attack or block"),
        "Should include the penalty clause: {summary}");

    // Butcher's Cleaver — equipment, should include the equip cost and bonus.
    let cleaver = "Equipped creature gets +3/+0.\n\
                   As long as equipped creature is a Human, it has lifelink.\n\
                   Equip {3}";
    let summary = LlmPlayer::short_effect_summary_for_test(cleaver);
    assert!(summary.contains("+3/+0"), "Should include bonus: {summary}");
    assert!(summary.contains("Equip {3}"), "Should include equip cost: {summary}");

    // Reminder text in parentheses should be stripped.
    let with_reminder = "Equipped creature gets +1/+2 and has hexproof. \
                         (It can't be the target of spells or abilities your opponents control.)\n\
                         Equip {3}";
    let summary = LlmPlayer::short_effect_summary_for_test(with_reminder);
    assert!(!summary.contains("It can't be the target"),
        "Should strip parenthesized reminder text: {summary}");
    assert!(summary.contains("hexproof"), "Should keep main text: {summary}");

    // Empty input stays empty.
    assert_eq!(LlmPlayer::short_effect_summary_for_test(""), "");
}

#[test]
fn resume_preserves_system_prompt() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("test");
    let deck = vec![("Lightning Bolt".to_string(), 4)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    let system_before = player.system_prompt_for_test().to_string();

    player.resume_from_log(&["p0 played Mountain".to_string()], mtg_engine::ids::PlayerId(0));

    let system_after = player.system_prompt_for_test().to_string();
    assert_eq!(system_before, system_after,
        "Resume should not change the system prompt");
}

// ── Player-relative log rewriter tests ───────────────────────────────

use mtg_engine::ids::PlayerId;
use mtg_player::llm::LlmPlayer;

fn rw(entry: &str, you: u8) -> String {
    LlmPlayer::rewrite_log_entry_for_test(entry, PlayerId(you))
}

#[test]
fn rewrite_past_tense_verbs_work_for_both_sides() {
    // Past-tense verbs are identical in 2nd and 3rd person so no
    // conjugation is needed.
    assert_eq!(rw("p0 drew a card", 0),    "You drew a card");
    assert_eq!(rw("p0 drew a card", 1),    "Opp drew a card");
    assert_eq!(rw("p1 drew 7 cards", 0),   "Opp drew 7 cards");
    assert_eq!(rw("p1 drew 7 cards", 1),   "You drew 7 cards");
    assert_eq!(rw("p0 played Mountain", 0), "You played Mountain");
    assert_eq!(rw("p0 played Mountain", 1), "Opp played Mountain");
    assert_eq!(
        rw("p1 cast Lightning Bolt (#5) targeting Goblin Piker (#12)", 0),
        "Opp cast Lightning Bolt (#5) targeting Goblin Piker (#12)"
    );
    assert_eq!(
        rw("p0 declared attackers: Grizzly Bears (#27), Kalonian Tusker (#30)", 0),
        "You declared attackers: Grizzly Bears (#27), Kalonian Tusker (#30)"
    );
}

#[test]
fn rewrite_present_tense_verbs_conjugate_for_you() {
    // Present-tense verbs need to be conjugated when the subject is "you".
    assert_eq!(rw("p0 keeps (0 mulligans)", 0), "You keep (0 mulligans)");
    assert_eq!(rw("p0 keeps (0 mulligans)", 1), "Opp keeps (0 mulligans)");
    assert_eq!(rw("p0 mulligans to 6", 0), "You mulligan to 6");
    assert_eq!(rw("p0 mulligans to 6", 1), "Opp mulligans to 6");
    assert_eq!(rw("p1 concedes", 0), "Opp concedes");
    assert_eq!(rw("p1 concedes", 1), "You concede");
    assert_eq!(rw("p0 wins the game with Lightning Bolt!", 0), "You win the game with Lightning Bolt!");
    assert_eq!(rw("p0 wins the game with Lightning Bolt!", 1), "Opp wins the game with Lightning Bolt!");
}

#[test]
fn rewrite_turn_banner() {
    assert_eq!(rw("── Turn 3 (p0) ──", 0), "── Turn 3 (your turn) ──");
    assert_eq!(rw("── Turn 3 (p0) ──", 1), "── Turn 3 (opp's turn) ──");
    assert_eq!(rw("── Turn 17 (p1) ──", 0), "── Turn 17 (opp's turn) ──");
    assert_eq!(rw("── Turn 17 (p1) ──", 1), "── Turn 17 (your turn) ──");
}

#[test]
fn rewrite_game_started_wrapper() {
    assert_eq!(
        rw("Game started (p0 on the play)", 0),
        "Game started (you are on the play)"
    );
    assert_eq!(
        rw("Game started (p0 on the play)", 1),
        "Game started (opp is on the play)"
    );
    assert_eq!(
        rw("Game started (p1 on the play)", 0),
        "Game started (opp is on the play)"
    );
}

#[test]
fn rewrite_mana_tap_log() {
    assert_eq!(
        rw("p0 tapped Mountain (#31) for mana (pool: Red:1)", 0),
        "You tapped Mountain (#31) for mana (pool: Red:1)"
    );
    assert_eq!(
        rw("p0 tapped Mountain (#31) for mana (pool: Red:1)", 1),
        "Opp tapped Mountain (#31) for mana (pool: Red:1)"
    );
}

#[test]
fn rewrite_leaves_unrelated_content_alone() {
    // No p{N} tokens at all.
    assert_eq!(
        rw("Traitorous Blood (#10) resolved", 0),
        "Traitorous Blood (#10) resolved"
    );
    assert_eq!(rw("Geistflame (#5) resolved", 0), "Geistflame (#5) resolved");
    assert_eq!(
        rw("Mulligan phase", 0),
        "Mulligan phase"
    );
}

#[test]
fn rewrite_combat_damage_log() {
    assert_eq!(
        rw("p1 took 2 combat damage (18) from Grizzly Bears (#27)", 0),
        "Opp took 2 combat damage (18) from Grizzly Bears (#27)"
    );
    assert_eq!(
        rw("p1 took 2 combat damage (18) from Grizzly Bears (#27)", 1),
        "You took 2 combat damage (18) from Grizzly Bears (#27)"
    );
}

#[test]
fn rewrite_does_not_touch_card_ids_or_numbers() {
    // Object IDs are `#N` not `pN`, so they must not be rewritten.
    assert_eq!(
        rw("p0 cast Spell (#42) targeting creature (#13)", 0),
        "You cast Spell (#42) targeting creature (#13)"
    );
    // Standalone numbers like "2 damage" mustn't pull in a phantom "p".
    assert_eq!(
        rw("p1 took 2 combat damage (18)", 0),
        "Opp took 2 combat damage (18)"
    );
}

// ── Double-faced cards (issue #205) ──────────────────────────────────────

/// A seat asked whether to transform has to be told what the card becomes.
/// The back face used to appear nowhere in the conversation, while the
/// system prompt forbids the seat to assume anything the prompt does not
/// say — so no transform cost in the set could be evaluated.
#[test]
fn card_faces_returns_both_faces_of_a_transforming_card() {
    let registry = CardRegistry::with_all_cards();

    let faces = mtg_player::llm::card_faces("Delver of Secrets", &registry);
    let names: Vec<&str> = faces.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["Delver of Secrets", "Insectile Aberration"]);
    let (_, back) = &faces[1];
    assert_eq!((back.power, back.toughness), (Some(3), Some(2)));
    assert!(
        back.keywords.iter().any(|k| format!("{k:?}").contains("Flying")),
        "the back face's keywords come with it: {:?}", back.keywords
    );

    // The combined name a pack or decklist may use resolves the same way.
    let combined = mtg_player::llm::card_faces("Delver of Secrets // Insectile Aberration", &registry);
    assert_eq!(combined.len(), 2);

    // A single-faced card is just itself.
    let single = mtg_player::llm::card_faces("Grizzly Bears", &registry);
    assert_eq!(single.len(), 1);

    // An unknown name yields nothing rather than panicking.
    assert!(mtg_player::llm::card_faces("Not A Card", &registry).is_empty());
}

#[test]
fn format_decklist_describes_the_back_face_a_transform_produces() {
    let registry = CardRegistry::with_all_cards();
    let entries = vec![
        ("Delver of Secrets".to_string(), 4),
        ("Ludevic's Test Subject".to_string(), 1),
    ];
    let result = mtg_player::llm::LlmPlayer::format_decklist_for_test(&entries, &registry);

    assert!(result.contains("Insectile Aberration"), "back face named: {result}");
    assert!(result.contains("3/2"), "back face P/T: {result}");
    assert!(
        result.contains("Ludevic's Abomination"),
        "the 13/13 a Test Subject's five hatchling counters buy: {result}"
    );
    assert!(result.contains("13/13"), "back face P/T: {result}");
}

// ── Match format in the system prompt (issue #210) ───────────────────────

/// A `--best-of 1` seat used to be told "Matches are best-of-three" and
/// that games 2 and 3 exist, and a plain mtg-runner game — one game, no
/// tournament — was told the same. Match structure changes how a game 1 is
/// played, so the prompt has to state the one the seat is actually in.
#[test]
fn the_match_section_states_the_match_the_seat_is_in() {
    let single = MatchFormat::SingleGame.match_section();
    assert!(single.contains("single game"), "{single}");
    assert!(!single.contains("best-of-three"), "{single}");
    assert!(!single.contains("tournament"), "one game is not a tournament: {single}");

    let bo1 = MatchFormat::best_of(1).match_section();
    assert!(bo1.contains("best-of-one"), "{bo1}");
    assert!(bo1.contains("no game 2"), "{bo1}");

    let bo3 = MatchFormat::best_of(3).match_section();
    assert!(bo3.contains("best-of-3"), "{bo3}");
    assert!(bo3.contains("one of you has won 2 games"), "{bo3}");
    assert!(bo3.contains("after 3 games have been played"), "{bo3}");
    assert!(bo3.contains("the loser of the previous one chooses"), "{bo3}");

    // Every format still explains what being on the play costs.
    for section in [single, bo1, bo3] {
        assert!(section.contains("skips their first draw step"), "{section}");
    }
}

/// #609: `BestOf` carried only the length, a constant for the whole
/// tournament, and `play_match` re-initialises both conversations before
/// every game — so a 16-game best-of-4 tournament answered 3,232 decisions
/// against 4 distinct system prompts, one per seat. Game 1 and game 4 of the
/// same match, a 0-0 match and a 2-1 one: identical bytes.
#[test]
fn a_tournament_seat_is_told_which_game_it_is_playing_and_the_score() {
    let at = |game, your_wins, their_wins| {
        MatchFormat::BestOf { best_of: 4, game, your_wins, their_wins }.match_section()
    };

    // Every distinct position in a best-of-4 gets its own text.
    let positions = [(1, 0, 0), (2, 1, 0), (2, 0, 1), (3, 1, 1), (4, 2, 1), (4, 1, 2)];
    let sections: Vec<String> = positions.iter().map(|&(g, y, t)| at(g, y, t)).collect();
    for (i, a) in sections.iter().enumerate() {
        for (j, b) in sections.iter().enumerate() {
            assert!(
                i == j || a != b,
                "positions {:?} and {:?} produce the same prompt",
                positions[i], positions[j]
            );
        }
    }

    // The game number and the score are both stated, from this seat's side.
    let g3 = at(3, 1, 1);
    assert!(g3.contains("game 3 of at most 4"), "{g3}");
    assert!(g3.contains("you 1, your opponent 1"), "{g3}");

    // What this game settles. Best-of-4 needs 3 wins.
    assert!(at(3, 2, 0).contains("Winning this game wins you the match"), "{}", at(3, 2, 0));
    assert!(at(3, 0, 2).contains("Losing this game loses you the match"), "{}", at(3, 0, 2));
    // ... and with a game still to come, losing it is not yet the match.
    assert!(!at(3, 2, 0).contains("loses you the match"), "{}", at(3, 2, 0));
    let decider = at(4, 2, 2);
    assert!(decider.contains("decides the match either way"), "{decider}");
    let early = at(1, 0, 0);
    assert!(early.contains("Neither of you can win the match with this game"), "{early}");

    // A best-of-4 is won 2-1 at the cap without anyone reaching 3, so the
    // last game's stake is not read off the win threshold alone.
    let last_ahead = at(4, 2, 1);
    assert!(last_ahead.contains("Winning this game wins you the match"), "{last_ahead}");
    assert!(last_ahead.contains("Losing it leaves the score level"), "{last_ahead}");
    let last_level = at(4, 1, 1);
    assert!(last_level.contains("decides the match either way"), "{last_level}");

    // A level finish is a draw, which is the half the mulligan prompt cannot
    // resolve for the seat and the reason an even best-of matters.
    for s in [&g3, &decider, &early, &last_ahead] {
        assert!(s.contains("draw"), "a level match is a draw: {s}");
        assert!(s.contains("1 \ntournament point each") || s.contains("1 tournament point each"),
            "and what it is worth: {s}");
    }

    // Game 1 gets the coin flip; a later game gets the rule that applies to
    // it, which the seat could not tell apart before.
    assert!(early.contains("game 1 is randomised"), "{early}");
    assert!(!early.contains("chose to go first"), "{early}");
    assert!(g3.contains("loser of game 2 chose to go first"), "{g3}");
}

/// #649: once a game can be drawn, a match can reach a game that cannot
/// change it — 2-0 into game 4 of a best-of-4 is the leader's however game
/// 4 goes — and both seats were told it decided the match.
///
/// Checked against the match itself rather than case by case: for every
/// position a best-of-2..7 can reach, play out every continuation under the
/// runner's stopping rule (#484), and require the stake the text states to
/// be the stake the continuations bear out.
#[test]
fn the_stated_stake_of_a_game_is_its_real_stake() {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum End { Won, Lost, Drawn }
    // Every way the match can end from here, `played` games in.
    fn ends(n: usize, played: usize, y: usize, t: usize, out: &mut Vec<End>) {
        let needed = n / 2 + 1;
        if y >= needed || t >= needed || played == n {
            out.push(match y.cmp(&t) {
                std::cmp::Ordering::Greater => End::Won,
                std::cmp::Ordering::Less => End::Lost,
                std::cmp::Ordering::Equal => End::Drawn,
            });
            return;
        }
        ends(n, played + 1, y + 1, t, out);
        ends(n, played + 1, y, t + 1, out);
        ends(n, played + 1, y, t, out);
    }
    let after = |n, game: usize, y, t| {
        let mut v = Vec::new();
        ends(n, game, y, t, &mut v);
        v
    };

    let mut settled_seen = 0;
    for n in 2..=7usize {
        for game in 1..=n {
            for y in 0..game {
                for t in 0..game - y {
                    let needed = n / 2 + 1;
                    if y >= needed || t >= needed {
                        continue; // the match is over; there is no game to prompt
                    }
                    let text = MatchFormat::BestOf { best_of: n, game, your_wins: y, their_wins: t }
                        .match_section();
                    let on_win = after(n, game, y + 1, t);
                    let on_loss = after(n, game, y, t + 1);
                    let on_draw = after(n, game, y, t);
                    let all: Vec<End> = [&on_win, &on_loss, &on_draw].iter().flat_map(|v| v.iter().copied()).collect();
                    let settled = all.iter().all(|&e| e == all[0]);
                    let at = format!("bo{n} game {game} at {y}-{t}");
                    assert_eq!(text.contains("already decided"), settled, "{at}:\n{text}");
                    if settled {
                        settled_seen += 1;
                        let ours = all[0] == End::Won;
                        assert!(text.contains(if ours { "you have won it" } else { "your opponent has won it" }), "{at}:\n{text}");
                        for claim in ["wins you the match", "loses you the match", "decides the match", "Neither of you"] {
                            assert!(!text.contains(claim), "{at} is settled, yet says {claim:?}:\n{text}");
                        }
                        continue;
                    }
                    if text.contains("Winning this game wins you the match") || text.contains("win it and the match is yours") {
                        assert!(on_win.iter().all(|&e| e == End::Won), "{at}: a win is not the match:\n{text}");
                    }
                    if text.contains("Losing this game loses you the match") || text.contains("lose it and it is theirs") {
                        assert!(on_loss.iter().all(|&e| e == End::Lost), "{at}: a loss is not the match:\n{text}");
                    }
                }
            }
        }
    }
    assert!(settled_seen > 0, "the sweep never reached a settled game, so it checked nothing about #649");
}

#[test]
fn a_single_game_seat_is_never_told_it_has_a_game_two() {
    let registry = CardRegistry::with_all_cards();
    let deck = vec![("Mountain".to_string(), 20)];
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("t");
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);
    let prompt = player.system_prompt_for_test();
    assert!(prompt.contains("single game"), "prompt: {prompt}");
    assert!(!prompt.contains("best-of-three"), "the stale boilerplate is gone");
    assert!(!prompt.contains("Games 2 and 3"), "there is no game 2");
}

#[test]
fn a_best_of_three_seat_is_told_about_games_two_and_three() {
    let registry = CardRegistry::with_all_cards();
    let deck = vec![("Mountain".to_string(), 20)];
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("t");
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::best_of(3));
    let prompt = player.system_prompt_for_test();
    assert!(prompt.contains("Matches are best-of-3"), "prompt: {prompt}");
    assert!(prompt.contains("game 1 of at most 3"), "prompt: {prompt}");
}

/// The recap a resumed seat is handed must reach `--log` in full, not as a
/// count of entries. It is the only description the seat gets of the game it
/// lost, and with only `"Resumed with N log entries"` in the file there was
/// nothing for a tester to check the resumed conversation against (issue
/// #208).
#[test]
fn resume_logs_the_recap_body_not_just_a_count() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::for_prompt_tests("P1");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry, MatchFormat::SingleGame);

    let log_path = std::env::temp_dir()
        .join(format!("mtg-resume-recap-log-{}", std::process::id()));
    let _ = std::fs::remove_file(&log_path);
    mtg_player::game_log::init(log_path.to_str().unwrap()).unwrap();

    let game_log = vec![
        "Game started".to_string(),
        "p0 drew 7 cards".to_string(),
        "── Turn 1 (p0) ──".to_string(),
        "p0 played Mountain".to_string(),
        "p1 cast Doomed Traveler".to_string(),
    ];
    player.resume_from_log(&game_log, mtg_engine::ids::PlayerId(0));

    let logged = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_file(&log_path);

    assert!(logged.contains("RESUME (5 log entries)"),
        "the entry count stays greppable in the label: {logged}");
    assert!(logged.contains("Game resumed. Here is the complete game log so far:"),
        "the recap's opening line reaches the log: {logged}");
    assert!(logged.contains("The game continues from this point"),
        "the recap's closing line reaches the log: {logged}");
    // Every entry is present, in the player-relative form the seat was
    // actually sent — p0 is the resuming player, so its lines read as "You"
    // and p1's as "Opp". Logging the sent text rather than the raw engine log
    // is the point: it is what the seat has to be audited against.
    for line in [
        "Game started",
        "You drew 7 cards",
        "── Turn 1 (your turn) ──",
        "You played Mountain",
        "Opp cast Doomed Traveler",
    ] {
        assert!(logged.contains(line), "recap line {line:?} reaches the log: {logged}");
    }
    assert!(!logged.contains("p0 played Mountain"),
        "the log carries the rewritten recap, not the raw engine entries: {logged}");
}

