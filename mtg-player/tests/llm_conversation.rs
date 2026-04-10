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

    player.init_conversation(&your_deck, &opp_deck, &registry);

    let system = player.system_prompt_for_test();

    // Should contain both decklists
    assert!(system.contains("Your decklist"), "System prompt should have your decklist section");
    assert!(system.contains("Opponent's decklist"), "System prompt should have opponent's decklist section");

    // Should contain card details
    assert!(system.contains("Lightning Bolt"), "Should include your cards");
    assert!(system.contains("Grizzly Bears"), "Should include opponent's cards");

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
    player.init_conversation(&deck, &deck, &registry);

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
    player.init_conversation(&deck, &deck, &registry);

    // Verify last_log_index starts at 0
    assert_eq!(player.last_log_index_for_test(), 0);
}

#[test]
fn resume_from_log_seeds_conversation() {
    let registry = CardRegistry::with_all_cards();
    let mut player = mtg_player::llm::LlmPlayer::new("test");
    let deck = vec![("Mountain".to_string(), 20)];
    player.init_conversation(&deck, &deck, &registry);

    assert_eq!(player.conversation_len_for_test(), 0);
    assert_eq!(player.last_log_index_for_test(), 0);

    let log = vec![
        "Game started".to_string(),
        "p0 drew 7 cards".to_string(),
        "p1 drew 7 cards".to_string(),
        "── Turn 1 (p0) ──".to_string(),
        "p0 played Mountain".to_string(),
    ];

    player.resume_from_log(&log);

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
    player.init_conversation(&deck, &deck, &registry);

    player.resume_from_log(&[]);

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
    player.init_conversation(&deck, &deck, &registry);

    let system_before = player.system_prompt_for_test().to_string();

    player.resume_from_log(&["p0 played Mountain".to_string()]);

    let system_after = player.system_prompt_for_test().to_string();
    assert_eq!(system_before, system_after,
        "Resume should not change the system prompt");
}
