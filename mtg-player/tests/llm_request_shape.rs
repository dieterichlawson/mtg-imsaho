//! What the metered seats put in a request body, independent of any key.

/// The current models reject a fixed thinking budget with a 400, and the
/// older families have no adaptive mode — so the seat has to ask for the
/// one its model accepts.
#[test]
fn the_thinking_parameter_matches_what_the_model_accepts() {
    use mtg_player::llm::thinking_param;
    for current in ["claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-sonnet-5", "claude-fable-5-1", "claude-sonnet-4-6"] {
        assert_eq!(thinking_param(current)["type"], "adaptive", "{current} takes adaptive thinking");
        assert!(thinking_param(current).get("budget_tokens").is_none(),
            "{current} rejects a fixed budget");
    }
    for older in ["claude-haiku-4-5", "claude-sonnet-4-5", "claude-3-5-haiku-20241022"] {
        assert_eq!(thinking_param(older)["type"], "enabled", "{older} has no adaptive mode");
        assert!(thinking_param(older)["budget_tokens"].as_u64().is_some_and(|b| b >= 1024),
            "{older} needs a budget of at least the API minimum");
    }
}

/// A tool schema's top-level property keys are checked by the API against
/// `^[a-zA-Z0-9_.-]{1,64}$`, and a failing key is a 400 — the request never
/// reaches the model, the seat gets nothing back, and the engine cancels
/// whatever was waiting on the answer.
///
/// Issue #398: three "mark some of these" prompts keyed their booleans by
/// the card's display name, so a `claude -p` seat could not cast Skaab
/// Goliath at all — six attempts in one game, every one rejected before it
/// was asked. The two top-level ones are index arrays now; the third
/// (`choose_pile_division`) nests its per-card keys under `pile_1`, where
/// the pattern does not apply.
///
/// The failure is invisible from inside a normal run: only a real API
/// rejects the schema, and the harness turns that rejection into an empty
/// answer that reads exactly like a seat declining. So the rule is written
/// down here and asserted where every structured request funnels through.
#[test]
fn no_schema_keys_a_top_level_property_by_a_card_name() {
    use mtg_player::llm::schema_key_is_legal;

    // The rule itself, on the shapes that matter.
    for legal in ["thoughts", "indices", "card_indices", "obj_62", "pile_1", "x.y-z"] {
        assert!(schema_key_is_legal(legal), "{legal} is a legal key");
    }
    for illegal in ["Spectral Rider (#62)", "0: Grizzly Bears", "Forbidden Alchemy", "", &"a".repeat(65)] {
        assert!(!schema_key_is_legal(illegal), "{illegal:?} must be refused");
    }

    // The rule is enforced where every structured request funnels
    // through — `send_message_structured` debug-asserts it, so any prompt
    // a test or a dev run actually builds is checked. It cannot be checked
    // from here: the builders need a live `GameView` and a backend.
}

