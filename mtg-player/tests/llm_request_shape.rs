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


/// An index-array prompt states its shape in the schema, where the API can
/// enforce it, and not only in prose the client then has to police.
///
/// `choose_ordering` demands a permutation of `0..n`, and
/// `parse_order_response` refuses anything else — but the schema carried no
/// `minItems`/`maxItems`, so `[]`, `[0]` and `[0,0,0]` were all valid
/// answers to a question that had already said they were not. The refusal
/// substitutes the order as listed and writes one MALFORMED line, and a
/// damage assignment order decides which blocker dies (CR 510.1c): that is
/// a strategic decision made for the seat, in silence (issue #547).
#[test]
fn the_ordering_schema_bounds_the_permutation_it_demands() {
    use mtg_player::llm::ordering_schema;

    for n in 1..=5usize {
        let schema = ordering_schema(n);
        let order = &schema["properties"]["order"];

        assert_eq!(order["minItems"].as_u64(), Some(n as u64),
            "an answer shorter than the list is not an ordering of it: {order}");
        assert_eq!(order["maxItems"].as_u64(), Some(n as u64),
            "nor is one longer: {order}");

        let offered: Vec<u64> = order["items"]["enum"].as_array()
            .expect("the entries are an index enum")
            .iter().map(|v| v.as_u64().expect("an index")).collect();
        assert_eq!(offered, (0..n as u64).collect::<Vec<u64>>(),
            "every index of the list is offered, and nothing else");

        // The shape the stub seat answered with, and the one the harness
        // then threw away, is no longer one the schema admits.
        assert!(order["minItems"].as_u64() != Some(0),
            "`[]` was schema-valid for a prompt that demanded {n} entries");
    }

    // Top-level keys stay inside what the API accepts (issue #398).
    for key in ordering_schema(3)["properties"].as_object().expect("properties").keys() {
        assert!(mtg_player::llm::schema_key_is_legal(key), "{key:?}");
    }
}
