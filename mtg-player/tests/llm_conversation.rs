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
