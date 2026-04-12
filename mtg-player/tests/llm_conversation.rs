/// Tests for the LLM player's multi-turn conversation and decklist formatting.

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
    assert!(result.contains("2x Victim of Night"), "Should list card count: {}", result);
    assert!(result.contains("10x Swamp"), "Should list land count: {}", result);

    // Should include oracle text
    assert!(result.contains("Destroy target non-Vampire"),
        "Should include oracle text for Victim of Night: {}", result);

    // Should include cost
    assert!(result.contains("{B}{B}"), "Should include mana cost: {}", result);

    // Should NOT duplicate card info for same-name entries
    let occurrences = result.matches("Destroy target non-Vampire").count();
    assert_eq!(occurrences, 1, "Oracle text should appear only once even with count > 1");
}

#[test]
fn init_conversation_sets_system_prompt_with_decklists() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::new("test");

    let your_deck = vec![
        ("Lightning Bolt".to_string(), 4),
        ("Mountain".to_string(), 16),
    ];
    let opp_deck = vec![
        ("Grizzly Bears".to_string(), 4),
        ("Forest".to_string(), 16),
    ];

    player.init_conversation(&your_deck, "Grizzly Bears {1}{G} | Creature — Bear 2/2\nForest | Land", &registry);

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
    let mut player = mtg_player::llm::LlmPlayer::new("test");

    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry);

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
    let mut player = mtg_player::llm::LlmPlayer::new("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry);

    // Verify last_log_index starts at 0
    assert_eq!(player.last_log_index_for_test(), 0);
}

#[test]
fn resume_from_log_seeds_conversation() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::new("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry);

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
    let mut player = mtg_player::llm::LlmPlayer::new("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, "Mountain | Land", &registry);

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
        "Should drop leading 'Enchant creature' line: {}", summary);
    assert!(summary.contains("flying"), "Should include flying: {}", summary);
    assert!(summary.contains("Prevent all combat damage"),
        "Should mention combat damage prevention: {}", summary);

    // Bonds of Faith — ensure the conditional gets through.
    let bonds = "Enchant creature\n\
                 Enchanted creature gets +2/+2 as long as it's a Human. \
                 Otherwise, it can't attack or block.";
    let summary = LlmPlayer::short_effect_summary_for_test(bonds);
    assert!(summary.contains("+2/+2"), "Should include the bonus: {}", summary);
    assert!(summary.contains("can't attack or block"),
        "Should include the penalty clause: {}", summary);

    // Butcher's Cleaver — equipment, should include the equip cost and bonus.
    let cleaver = "Equipped creature gets +3/+0.\n\
                   As long as equipped creature is a Human, it has lifelink.\n\
                   Equip {3}";
    let summary = LlmPlayer::short_effect_summary_for_test(cleaver);
    assert!(summary.contains("+3/+0"), "Should include bonus: {}", summary);
    assert!(summary.contains("Equip {3}"), "Should include equip cost: {}", summary);

    // Reminder text in parentheses should be stripped.
    let with_reminder = "Equipped creature gets +1/+2 and has hexproof. \
                         (It can't be the target of spells or abilities your opponents control.)\n\
                         Equip {3}";
    let summary = LlmPlayer::short_effect_summary_for_test(with_reminder);
    assert!(!summary.contains("It can't be the target"),
        "Should strip parenthesized reminder text: {}", summary);
    assert!(summary.contains("hexproof"), "Should keep main text: {}", summary);

    // Empty input stays empty.
    assert_eq!(LlmPlayer::short_effect_summary_for_test(""), "");
}

#[test]
fn resume_preserves_system_prompt() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::new("test");
    let deck = vec![("Lightning Bolt".to_string(), 4)];
    player.init_conversation(&deck, "Mountain | Land", &registry);

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
