//! What the metered seats put in a request body, independent of any key.

/// The current models reject a fixed thinking budget with a 400, and the
/// older families have no adaptive mode — so the seat has to ask for the
/// one its model accepts.
#[test]
fn the_thinking_parameter_matches_what_the_model_accepts() {
    use mtg_player::llm::thinking_param;
    for current in ["claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-sonnet-5", "claude-fable-5-1", "claude-sonnet-4-6",
                    "claude-haiku-5-5", "claude-opus-5-5", "claude-sonnet-5-5"] {
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
/// was asked. All three are index arrays now; the third
/// (`choose_pile_division`) nested its per-card keys under `pile_a` until
/// #857, which the pattern allows but the prompt cache does not.
///
/// The failure is invisible from inside a normal run: only a real API
/// rejects the schema, and the harness turns that rejection into an empty
/// answer that reads exactly like a seat declining. So the rule is written
/// down here and asserted where every structured request funnels through.
#[test]
fn no_schema_keys_a_top_level_property_by_a_card_name() {
    use mtg_player::llm::schema_key_is_legal;

    // The rule itself, on the shapes that matter.
    for legal in ["thoughts", "indices", "card_indices", "obj_62", "pile_a", "x.y-z"] {
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
            "floating": {"type": "object", "properties": {"black": {"type": "string", "enum": ["0", "1"]}}},
            "sources": {"type": "array", "items": {"type": "string"},
                        "prefixItems": [{"type": "string", "enum": ["0", "2", "4"]}]}
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
    // the X-funding workaround is left as it was written — but for
    // `prefixItems`, which neither provider documents and the answer is held
    // to client-side (#857); the floating pool's string enum stays.
    let s = sanitize_schema_for_gemini(&funding);
    assert_eq!(s["properties"]["floating"], funding["properties"]["floating"]);
    assert!(s["properties"]["sources"].get("prefixItems").is_none(), "{s}");
    assert_eq!(s["properties"]["sources"]["items"], funding["properties"]["sources"]["items"]);
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

/// Issue #637: a combat damage division is answered as the amount, an
/// integer enum over exactly `[lethal, all that is left]`, under a fixed
/// top-level key — and the Gemini path restates it as the same range.
#[test]
fn the_damage_amount_schema_offers_exactly_the_legal_amounts() {
    use mtg_player::llm::{damage_amount_schema, sanitize_schema_for_gemini};
    let schema = damage_amount_schema(2, 5);
    let offered: Vec<u64> = schema["properties"]["amount"]["enum"].as_array()
        .expect("an integer enum").iter().map(|v| v.as_u64().expect("an amount")).collect();
    assert_eq!(offered, vec![2, 3, 4, 5], "lethal to all of it, and nothing else");
    assert_eq!(schema["required"], serde_json::json!(["thoughts", "amount"]));
    for key in schema["properties"].as_object().expect("properties").keys() {
        assert!(mtg_player::llm::schema_key_is_legal(key), "{key:?}");
    }
    let g = sanitize_schema_for_gemini(&schema);
    assert_eq!((&g["properties"]["amount"]["minimum"], &g["properties"]["amount"]["maximum"]),
        (&serde_json::json!(2), &serde_json::json!(5)));
}

/// The thinking level a run asks for reaches the request in the form each
/// model family accepts: adaptive thinking plus `output_config.effort` on
/// the current models, a `budget_tokens` on the ones that still take one,
/// and no `thinking` parameter at all for `off`. The level is the biggest
/// output-token term a decision has once the history is bounded, so the
/// default is the low setting and `MTG_LLM_THINKING` moves it.
#[test]
fn the_thinking_level_is_sent_the_way_the_model_accepts_it() {
    use mtg_player::llm::{apply_thinking, max_output_tokens, parse_thinking_level, ThinkingLevel};

    assert_eq!(parse_thinking_level(""), Some(ThinkingLevel::Off), "unset is off");
    assert_eq!(parse_thinking_level("OFF"), Some(ThinkingLevel::Off));
    assert_eq!(parse_thinking_level("medium"), Some(ThinkingLevel::Medium));
    assert_eq!(parse_thinking_level("high"), Some(ThinkingLevel::High));
    assert_eq!(parse_thinking_level("2048"), Some(ThinkingLevel::Budget(2048)));
    assert_eq!(parse_thinking_level("0"), Some(ThinkingLevel::Off));
    assert_eq!(parse_thinking_level("lots"), None, "a typo is not quietly some level");

    let body = || serde_json::json!({"model": "m", "output_config": {"format": {"type": "json_schema"}}});

    // Adaptive models: effort steers depth, a budget would be a 400.
    for model in ["claude-sonnet-4-6", "claude-opus-5", "claude-fable-5-1"] {
        for (level, effort) in [
            (ThinkingLevel::Low, "low"), (ThinkingLevel::Medium, "medium"),
            (ThinkingLevel::High, "high"), (ThinkingLevel::Budget(1024), "low"),
            (ThinkingLevel::Budget(20000), "high"),
        ] {
            let mut b = body();
            apply_thinking(&mut b, model, level);
            assert_eq!(b["thinking"]["type"], "adaptive", "{model} {level:?}");
            assert!(b["thinking"].get("budget_tokens").is_none(), "{model} rejects a budget");
            assert_eq!(b["output_config"]["effort"], effort, "{model} {level:?}");
            assert_eq!(b["output_config"]["format"]["type"], "json_schema", "the format survives");
            assert_eq!(b["max_tokens"].as_u64(), Some(max_output_tokens(level)), "{model} {level:?}");
        }
        let mut b = body();
        apply_thinking(&mut b, model, ThinkingLevel::Off);
        assert!(b.get("thinking").is_none(), "{model}: off sends no thinking parameter");
        assert_eq!(b["output_config"]["effort"], "low");
        assert_eq!(b["max_tokens"].as_u64(), Some(max_output_tokens(ThinkingLevel::Off)));
    }

    // Budget models: the budget is the knob, effort would be rejected.
    for model in ["claude-haiku-4-5", "claude-sonnet-4-5"] {
        for (level, budget) in [
            (ThinkingLevel::Low, 1024), (ThinkingLevel::Medium, 4096),
            (ThinkingLevel::High, 16384), (ThinkingLevel::Budget(2048), 2048),
            (ThinkingLevel::Budget(100), 1024),
        ] {
            let mut b = body();
            apply_thinking(&mut b, model, level);
            assert_eq!(b["thinking"]["type"], "enabled", "{model} {level:?}");
            assert_eq!(b["thinking"]["budget_tokens"].as_u64(), Some(budget), "{model} {level:?}");
            assert!(b["output_config"].get("effort").is_none(), "{model} rejects effort");
            assert!(b["max_tokens"].as_u64().unwrap() > budget, "the budget fits under max_tokens");
        }
        let mut b = body();
        apply_thinking(&mut b, model, ThinkingLevel::Off);
        assert!(b.get("thinking").is_none(), "{model}: off sends no thinking parameter");
    }
}

/// `max_tokens` is a cap on a runaway, not a budget: it fits the largest
/// structured answer with room to spare at every level, grows with the
/// thinking the level asks for, and never exceeds the 8,192 the default
/// level used to send.
#[test]
fn the_output_cap_fits_the_largest_answer_and_grows_with_the_thinking() {
    use mtg_player::llm::{max_output_tokens, ThinkingLevel, ANSWER_TOKENS};
    // The largest answer the harness asks for: the draft runner's deck,
    // every drafted name with a count. Forty-five names at ≈ 8 tokens
    // each, five basics, two sentences — well under a quarter of this.
    assert!(ANSWER_TOKENS >= 1024);
    let off = max_output_tokens(ThinkingLevel::Off);
    let low = max_output_tokens(ThinkingLevel::Low);
    let medium = max_output_tokens(ThinkingLevel::Medium);
    let high = max_output_tokens(ThinkingLevel::High);
    assert!(off >= ANSWER_TOKENS, "off has at least the answer's own room");
    assert_eq!(off, low, "a model that thinks at off anyway gets low's room");
    assert!(low <= medium && medium <= high, "{off} {low} {medium} {high}");
    assert!(low <= 4096, "the default level is capped at half of what it was: {low}");
    assert!(max_output_tokens(ThinkingLevel::Budget(1024)) > 1024, "a budget fits under its cap");
}

/// The schema is part of the cached prefix on both Anthropic request paths,
/// so what a request carries is the schema without its per-call values: an
/// ordering of three and an ordering of five go out as the same bytes, and
/// only the checks the answer is held to differ.
#[test]
fn a_stable_schema_drops_the_values_that_change_and_nothing_else() {
    use mtg_player::llm::{cache_stable_schema, ordering_schema, schema_violation};
    let three = cache_stable_schema(&ordering_schema(3));
    assert_eq!(three, cache_stable_schema(&ordering_schema(5)));
    let text = three.to_string();
    for gone in ["\"enum\"", "\"minItems\"", "\"maxItems\""] {
        assert!(!text.contains(gone), "{gone} is a per-call value: {text}");
    }
    assert_eq!(three["properties"]["order"]["items"]["type"], "integer", "the types stay: {text}");
    assert!(three["properties"]["order"]["description"].is_string(), "descriptions stay: {text}");

    // A field that happens to be named like a keyword is a field.
    let named = serde_json::json!({"type": "object", "properties": {
        "enum": {"type": "integer", "enum": [0, 1]}, "minimum": {"type": "boolean"}}});
    let kept = cache_stable_schema(&named);
    assert!(kept["properties"].get("enum").is_some() && kept["properties"].get("minimum").is_some(), "{kept}");
    assert!(kept["properties"]["enum"].get("enum").is_none(), "{kept}");

    // And the answer is held to what was left out.
    let full = ordering_schema(3);
    assert_eq!(schema_violation(&full, &serde_json::json!({"order": [2, 0, 1]})), None);
    assert_eq!(schema_violation(&full, &serde_json::json!({"thoughts": "t", "order": [2, 0, 1]})), None);
    assert_eq!(schema_violation(&full, &serde_json::json!({"order": [2, 0, 3]})).as_deref(),
        Some("`order[2]` is 3; it must be 0 to 2"));
    assert_eq!(schema_violation(&full, &serde_json::json!({"order": [2, 0]})).as_deref(),
        Some("`order` has 2 entries; it needs at least 3"));
    assert_eq!(schema_violation(&full, &serde_json::json!({})).as_deref(), Some("`order` is missing"));
    let plan = serde_json::json!({"type": "object", "properties": {
        "then": {"type": "array", "items": {"type": "string", "enum": ["Mountain", "Lightning Bolt"]}}},
        "required": ["then"]});
    assert_eq!(schema_violation(&plan, &serde_json::json!({"then": ["Lightning Blot"]})).as_deref(),
        Some(r#"`then[0]` is "Lightning Blot"; it must be one of "Mountain", "Lightning Bolt""#));
}

/// A menu row copied into a plan names its card plainly enough: with the
/// enum gone from the request, decoding does not force the exact name, and
/// the answer is read as the one card it names rather than asked again.
#[test]
fn a_string_that_holds_one_allowed_value_is_read_as_it() {
    use mtg_player::llm::{repair_enum_strings, schema_violation};
    let plan = serde_json::json!({"type": "object", "properties": {
        "then": {"type": "array", "items": {"type": "string",
            "enum": ["Forest", "Goblin Piker", "Mikaeus, the Lunarch", "Kessig Forest Wolf"]}}}});
    let answer = serde_json::json!({"then": ["Cast Goblin Piker (tap Mountain, Forest)", "Play Forest",
        "Cast Mikaeus, the Lunarch", "Cast Kessig Forest Wolf", "Lightning Bolt"]});
    let fixed = repair_enum_strings(&plan, &answer);
    assert_eq!(fixed["then"], serde_json::json!(["Goblin Piker", "Forest", "Mikaeus, the Lunarch",
        "Kessig Forest Wolf", "Lightning Bolt"]), "the longest name a string holds wins; none is none");
    assert!(schema_violation(&plan, &fixed).is_some_and(|w| w.contains("Lightning Bolt")),
        "a card not in hand is still not an answer");

    // #863: a row about something that is not a card in hand names lands
    // in its tap list. It is not read as one of them.
    let island = serde_json::json!({"type": "object", "properties": {
        "then": {"type": "array", "items": {"type": "string", "enum": ["Island", "Forest"]}}}});
    for row in ["Flashback Think Twice (tap Island, Island, Island)", "Activate Nephalia Drownyard (tap Island)",
                "Equip Blazing Torch (tap Forest)"] {
        let answer = serde_json::json!({"then": [row]});
        assert_eq!(repair_enum_strings(&island, &answer), answer, "{row}");
        assert!(schema_violation(&island, &answer).is_some(), "{row} is asked again");
    }
    assert_eq!(repair_enum_strings(&island, &serde_json::json!({"then": ["Play Island"]})),
        serde_json::json!({"then": ["Island"]}));
    // A copied row keeps its index or bullet; a value is read to a word's
    // end, so "10" is not "1" and "all to B" is not pile "A".
    for (row, read) in [("3: Play Island", "Island"), ("1. Island", "Island"), ("- Forest", "Forest"),
                        ("3) Play Island (land)", "Island")] {
        assert_eq!(repair_enum_strings(&island, &serde_json::json!({"then": [row]})),
            serde_json::json!({"then": [read]}), "{row}");
    }
    let amount = serde_json::json!({"type": "string", "enum": ["0", "1", "2"]});
    assert_eq!(repair_enum_strings(&amount, &serde_json::json!("2 mana")), serde_json::json!("2"));
    assert_eq!(repair_enum_strings(&amount, &serde_json::json!("10")), serde_json::json!("10"));
    assert_eq!(repair_enum_strings(&amount, &serde_json::json!("-1")), serde_json::json!("-1"), "not a bullet");
    assert_eq!(repair_enum_strings(&amount, &serde_json::json!("2.")), serde_json::json!("2"), "not an index");
    let pile = serde_json::json!({"type": "string", "enum": ["A", "B"]});
    assert_eq!(repair_enum_strings(&pile, &serde_json::json!("all to B")), serde_json::json!("all to B"));
    assert_eq!(repair_enum_strings(&pile, &serde_json::json!("A (sacrificed)")), serde_json::json!("A"));
}
