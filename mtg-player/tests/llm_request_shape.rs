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

/// Every integer `enum` a seat's schema carries, by JSON path — the feature
/// the harness's own portability comment says the Gemini seat's provider
/// rejects outright.
fn integer_enums(value: &serde_json::Value, path: &str, found: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            let integer = matches!(
                map.get("type").and_then(serde_json::Value::as_str), Some("integer" | "number"));
            if integer && map.contains_key("enum") {
                found.push(path.to_string());
            }
            for (k, v) in map {
                integer_enums(v, &format!("{path}.{k}"), found);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                integer_enums(v, &format!("{path}[{i}]"), found);
            }
        }
        _ => {}
    }
}

/// The harness states one portability rule — "Anthropic rejects
/// `minimum`/`maximum` on integer fields, Gemini rejects `enum` on integer
/// fields, and only `enum` on string fields is both accepted and enforced by
/// both" — and used to enforce half of it. The Anthropic and `claude -p`
/// paths sanitize; the Gemini request sent the caller's schema verbatim as
/// `response_format`, and an integer `enum` is what 751 of 851 structured
/// requests in one night's harvest were made of (issue #546).
#[test]
fn a_gemini_schema_carries_no_integer_enum() {
    use mtg_player::llm::{ordering_schema, sanitize_schema_for_gemini};

    // Harvested verbatim from a stub seat: the four top-level shapes that
    // carry one, plus the X-funding shape that carries a string enum.
    let action = serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {"type": "string"},
            "action": {"type": "integer", "enum": [0, 1, 2, 3], "description": "The action index"}
        },
        "required": ["thoughts", "action"]
    });
    let blockers = serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {"type": "string"},
            "0": {"type": "integer", "enum": [0, -1]},
            "1": {"type": "integer", "enum": [-1]}
        }
    });
    let indices = serde_json::json!({
        "type": "object",
        "properties": {
            "indices": {
                "type": "array",
                "items": {"type": "integer", "enum": [0, 1]},
                "minItems": 1, "maxItems": 1
            }
        }
    });
    let funding = serde_json::json!({
        "type": "object",
        "properties": {
            "lands": {"type": "string", "enum": ["0", "1", "2"]},
            "rocks": {"type": "object", "properties": {}}
        }
    });

    for (name, schema) in [
        ("action", &action), ("blockers", &blockers), ("indices", &indices),
        ("ordering", &ordering_schema(3)), ("x-funding", &funding),
    ] {
        let sanitized = sanitize_schema_for_gemini(schema);
        let mut left = Vec::new();
        integer_enums(&sanitized, name, &mut left);
        assert!(left.is_empty(), "{name}: integer enums survive at {left:?}: {sanitized}");
    }

    // The constraint is not dropped, it is restated: a contiguous run of
    // integers is exactly its own min and max.
    let s = sanitize_schema_for_gemini(&action);
    assert_eq!(s["properties"]["action"]["minimum"], serde_json::json!(0));
    assert_eq!(s["properties"]["action"]["maximum"], serde_json::json!(3));
    assert_eq!(s["properties"]["action"]["description"], action["properties"]["action"]["description"],
        "and the rest of the field is untouched");

    let s = sanitize_schema_for_gemini(&blockers);
    assert_eq!((&s["properties"]["0"]["minimum"], &s["properties"]["0"]["maximum"]),
        (&serde_json::json!(-1), &serde_json::json!(0)), "`[0, -1]` is the range -1..=0");
    assert_eq!((&s["properties"]["1"]["minimum"], &s["properties"]["1"]["maximum"]),
        (&serde_json::json!(-1), &serde_json::json!(-1)), "a one-value enum is a point range");

    // Inside an array's items, and with the array's own bounds intact.
    let s = sanitize_schema_for_gemini(&ordering_schema(3));
    assert_eq!(s["properties"]["order"]["items"]["maximum"], serde_json::json!(2));
    assert_eq!(s["properties"]["order"]["minItems"], serde_json::json!(3));

    // String enums are the one form both providers accept and enforce, so
    // the X-funding workaround is left exactly as it was written.
    assert_eq!(sanitize_schema_for_gemini(&funding), funding);
}

/// There are two request paths, not one — `mtg-player` for the game and
/// `mtg-draft-runner` for the draft — and the draft copy has already missed
/// a fix the game path got (#404). Neither may hand a provider a schema no
/// sanitizer has seen.
#[test]
fn neither_request_path_sends_a_schema_no_sanitizer_has_seen() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent()
        .expect("the workspace root is the crate's parent");
    let paths = ["mtg-player/src/llm.rs", "mtg-draft-runner/src/llm_client.rs"];

    let mut sites = 0;
    for rel in paths {
        let src = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
        for (n, line) in src.lines().enumerate() {
            // The Gemini request body names the schema field itself.
            if !line.contains("\"response_format\"") {
                continue;
            }
            sites += 1;
            assert!(line.contains("sanitize_schema_for_gemini("),
                "{rel}:{} sends a schema straight to the provider: {}", n + 1, line.trim());
        }
    }
    assert_eq!(sites, 2, "both Gemini request paths are covered, and no third one appeared");
}
