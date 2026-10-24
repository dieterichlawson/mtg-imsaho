use std::env;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::types::{CardType, Step};
use mtg_engine::view::GameView;
use reqwest::blocking::Client;

use crate::Player;

/// Per-model token usage tracking for `LlmPlayer` game calls.
use std::sync::Mutex;
use std::collections::HashMap;
use std::fmt::Write;

pub mod answer_text;
mod claude_code;
pub mod cost;
pub use claude_code::{available as claude_code_available, binary as claude_code_binary, kill_live_calls as claude_code_kill_live_calls, kill_live_calls_from_signal as claude_code_kill_live_calls_from_signal, BINARY_ENV as CLAUDE_CODE_BINARY_ENV, MAX_LIVE_CALLS as CLAUDE_CODE_MAX_LIVE_CALLS};
// The one `claude -p` subprocess driver, for both seats in the workspace
// (#404).
pub use claude_code::{prepare_seat as claude_code_prepare_seat, run_print_mode as claude_code_run};
pub use claude_code::{ANSWER_SCHEMA_ENV as CLAUDE_CODE_ANSWER_SCHEMA_ENV, CACHE_TTL_ENV as CLAUDE_CODE_CACHE_TTL_ENV, UNREADABLE_LIMIT as CLAUDE_CODE_UNREADABLE_LIMIT};
pub use claude_code::{stop_gracefully_on_signal, stop_signal};
pub use cost::{api_equivalent, cost, is_plan_quota, model_prices, total_cost, Cost, ModelPrices};


#[derive(Default, Debug, Clone)]
pub struct LlmModelUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
    pub calls: u64,
    /// Calls that came back but whose answer the harness could not use, so it
    /// substituted a fallback (a default action, a forced keep, X=0). Counted
    /// separately from `calls` because a rejected answer is a *successful*
    /// call — without this, a seat that never once chose anything reported
    /// the same "70 calls" as a healthy one (issue #211).
    pub rejected: u64,
    /// Decisions where the backend never answered at all — the subprocess
    /// died, or the API gave up after its retries — so the harness had
    /// nothing to reject and substituted a fallback anyway.
    ///
    /// Separate from `rejected` because the two call for opposite responses
    /// from the operator: "your CLI is logged out / you are rate limited"
    /// against "the model is playing badly". Folded together, they were one
    /// number and the per-decision line quoted `{}` as the seat's answer —
    /// an answer the model never gave (issue #587). Not part of `calls`
    /// either: no call succeeded.
    pub unanswered: u64,
    /// The part of `output` that was thinking, where the backend reports
    /// it (the `claude -p` result's `output_tokens_details`). Thinking is
    /// billed as output, and once the history is bounded it is the largest
    /// term a decision has, so the summary says how much of the output it
    /// was. Zero where the backend does not say.
    pub thinking: u64,
}

static LLM_MODEL_USAGE: std::sync::LazyLock<Mutex<HashMap<String, LlmModelUsage>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The `thinking` parameter this model accepts.
///
/// The current models take adaptive thinking and reject a fixed token
/// budget outright — `{"type": "enabled", "budget_tokens": N}` is a 400 on
/// Opus 5, Opus 4.8/4.7, Sonnet 5 and the Fable family — so a seat that
/// hard-codes a budget works only until someone asks for a current model.
/// The older families are the other way round: they have no adaptive mode
/// and need the budget to think at all. Depth is steered by
/// `output_config.effort` on everything that takes adaptive thinking.
#[must_use]
/// True once a further mulligan cannot change what this seat keeps.
///
/// CR 103.4 allows any number of mulligans; after seven, the bottoming
/// obligation is the whole hand, so every subsequent mulligan draws seven
/// and bottoms seven for the same empty keep. An automated seat that kept
/// answering "mulligan" here would never finish the game, so the seat
/// stops asking. The engine still offers the action.
fn mulligan_is_dominated(view: &GameView) -> bool {
    view.your_mulligan_count as usize >= mtg_engine::state::OPENING_HAND_SIZE
}

/// Whether a string may be a *top-level* property key of a tool's
/// input schema.
///
/// The API checks these against `^[a-zA-Z0-9_.-]{1,64}$` and rejects the
/// whole request with a 400 if one fails — before the model sees it, so
/// the seat gets no answer and the engine cancels whatever was waiting.
/// A card's display name (`Spectral Rider (#62)`) fails on the space, the
/// parentheses and the `#`; issue #398 is six casts lost to it in one
/// game. Keys nested below the top level are not checked, but a card's
/// name is a runtime string wherever it goes: it makes the schema a new one
/// each call (`cache_stable_schema`, #857), so no prompt keys by one.
#[must_use]
pub fn schema_key_is_legal(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Rewrite a schema into the subset Anthropic's structured-output mode accepts:
/// - add `additionalProperties: false` to every object
/// - strip unsupported numeric constraints (`minimum`, `maximum`, `multipleOf`)
/// - strip `thoughts` unless the caller keeps it
///
/// Every seat now passes `keep_thoughts` true. The Messages API seat used to
/// pass false and read its reasoning from the thinking block, which is empty
/// with thinking off (the default) or where the model omits it. The `claude
/// -p` seat's result object carries no thinking block at all — so stripping the
/// field there erased the reasoning entirely: the schema asked for it, this
/// stripped it, and nothing read a thinking channel, which is why 101
/// decisions produced zero THOUGHT lines (issue #213).
///
/// Both request paths call this one function. The draft crate used to carry
/// its own copy, which had no `keep_thoughts` parameter at all, so #213's fix
/// never travelled and a `cc` seat's 42 picks were recorded with no reasoning
/// in the same log where the tournament recorded 202 (issue #607) — the same
/// drift, in the same pair of files, that #546 removed for the Gemini
/// sanitizer and #404 met before that.
#[must_use]
pub fn sanitize_schema_for_anthropic(value: &serde_json::Value, keep_thoughts: bool) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                // Strip unsupported numeric constraints.
                if key == "minimum" || key == "maximum" || key == "multipleOf" {
                    continue;
                }
                new_map.insert(key.clone(), sanitize_schema_for_anthropic(val, keep_thoughts));
            }
            // Add additionalProperties: false to object types.
            if new_map.get("type").and_then(|t| t.as_str()) == Some("object") {
                new_map.entry("additionalProperties".to_string())
                    .or_insert(serde_json::Value::Bool(false));
                if !keep_thoughts {
                    if let Some(props) = new_map.get_mut("properties").and_then(|p| p.as_object_mut()) {
                        props.remove("thoughts");
                    }
                    if let Some(req) = new_map.get_mut("required").and_then(|r| r.as_array_mut()) {
                        req.retain(|v| v.as_str() != Some("thoughts"));
                    }
                }
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter()
                .map(|v| sanitize_schema_for_anthropic(v, keep_thoughts)).collect())
        }
        other => other.clone(),
    }
}
/// Rewrite a schema into the subset the Gemini seat's provider accepts.
///
/// The harness has always stated the portability rule — "Anthropic rejects
/// `minimum`/`maximum` on integer fields, Gemini rejects `enum` on integer
/// fields, and only `enum` on string fields is both accepted and enforced by
/// both" — and enforced exactly half of it. `AnthropicBackend::sanitize_schema`
/// strips the numeric constraints on the Anthropic and `claude -p` paths, and
/// the draft crate has its own copy for its two. Nothing at all stood between
/// a schema and the Gemini request, which sends it verbatim as
/// `response_format`.
///
/// An integer `enum` is what nearly every schema here is made of: 751 of 851
/// structured requests in one night's harvest carried one — every menu
/// decision, every blocker assignment, every ordering prompt, every index set
/// (#546). On the provider's own account of itself that is a 400 on 88% of a
/// game's decisions, and the failure is silent by construction: a 400 is not
/// in the retry set, `call_interactions_structured` returns `{}` after one
/// attempt, and an empty answer is what a seat that declined looks like
/// (#398). Six of the ten callers substitute without logging anything at all.
///
/// So an integer `enum` becomes the `minimum`/`maximum` that says the same
/// thing. That is exact for a contiguous run of integers, which every enum in
/// this program is — `0..n-1` for a menu or an ordering, `[0, -1]` and `[-1]`
/// for a blocker assignment — and a weaker bound otherwise, never a wrong one.
/// String enums are left alone: they are the one form the comment says both
/// providers accept *and* enforce, which is why the X-funding schema is built
/// out of them.
///
/// This does not depend on the claim being true today. A schema without an
/// integer `enum` is accepted either way, and if the claim is stale the cost
/// is a range constraint in place of a set constraint on one provider. What
/// it removes is the asymmetry: neither request path now sends a schema no
/// sanitizer has seen.
#[must_use]
pub fn sanitize_schema_for_gemini(value: &serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(map) = value else {
        if let serde_json::Value::Array(items) = value {
            return serde_json::Value::Array(items.iter().map(sanitize_schema_for_gemini).collect());
        }
        return value.clone();
    };

    // Only an integer field's enum, and only when every value in it is one:
    // anything else is left exactly as the caller wrote it.
    let is_integer = matches!(
        map.get("type").and_then(serde_json::Value::as_str), Some("integer" | "number"));
    let bounds = if is_integer {
        map.get("enum")
            .and_then(serde_json::Value::as_array)
            .filter(|values| !values.is_empty())
            .and_then(|values| values.iter().map(serde_json::Value::as_i64).collect::<Option<Vec<i64>>>())
            .and_then(|values| Some((*values.iter().min()?, *values.iter().max()?)))
    } else {
        None
    };

    let mut out = serde_json::Map::new();
    for (key, val) in map {
        // `prefixItems` is not in the subset either provider documents; the
        // answer is held to it client-side (`schema_violation`).
        if (bounds.is_some() && key == "enum") || key == UNREAD || key == ONLY_VALUE || key == "prefixItems" {
            continue;
        }
        out.insert(key.clone(), sanitize_schema_for_gemini(val));
    }
    if let Some((low, high)) = bounds {
        out.entry("minimum".to_string()).or_insert(serde_json::json!(low));
        out.entry("maximum".to_string()).or_insert(serde_json::json!(high));
    }
    serde_json::Value::Object(out)
}

/// The constraints [`cache_stable_schema`] leaves out of a request, and
/// the markers [`UNREAD`] and [`ONLY_VALUE`].
const PER_CALL_CONSTRAINTS: [&str; 8] = [
    "enum", "minItems", "maxItems", "minimum", "maximum", "prefixItems", UNREAD, ONLY_VALUE,
];

/// A field this question asks for only so that its schema is the same as
/// its kind's, and whose answer is not read: the plan fields on an
/// opponent's turn. The answer is not held to it, and no request carries
/// the marker.
pub const UNREAD: &str = "x-unread";

/// A field with one legal value this time, which its question reads as
/// that value when it is left out: a pool colour nothing is floating in,
/// whose amount can only be 0 (#930). The format line does not offer it
/// and the check does not require it; a value written for it is still
/// held to its `enum`. It changes from call to call, so no request
/// carries the marker.
pub const ONLY_VALUE: &str = "x-only-value";

/// The schema a request to Anthropic carries: `schema` without the
/// constraints that change from one call to the next — every `enum` (a
/// menu's indices, a plan's card names) and an array's `minItems` /
/// `maxItems`.
///
/// The schema is part of the cached prefix. On the Messages API, changing
/// `output_config.format` invalidates the prompt cache; through `claude -p`
/// the schema is a tool definition, which renders ahead of the system
/// prompt. With an `enum: [0..n]` on every menu and the hand's card names on
/// every main-phase plan, nearly every call's schema was new, and a seat
/// wrote its whole system prompt again at 1.25x the input price where a
/// read costs 0.1x — 60 to 95% of a game's calls read nothing from the
/// cache (reports/llm-cost.md, phase 3). Without them a prompt kind sends
/// the same bytes every time, and the answer is checked against the full
/// schema instead ([`schema_violation`]). Descriptions stay, so they must
/// not carry a runtime value either: the count, range or name a question
/// depends on goes in the prompt.
#[must_use]
pub fn cache_stable_schema(schema: &serde_json::Value) -> serde_json::Value {
    fn strip(value: &serde_json::Value, property_names: bool) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(map.iter()
                // The keys of `properties` are field names, not keywords.
                .filter(|(k, _)| property_names || !PER_CALL_CONSTRAINTS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), strip(v, !property_names && k == "properties")))
                .collect()),
            serde_json::Value::Array(items) =>
                serde_json::Value::Array(items.iter().map(|v| strip(v, false)).collect()),
            other => other.clone(),
        }
    }
    strip(schema, false)
}

/// `answer` with each string that its schema's `enum` does not hold, but
/// that is about exactly one longest allowed value, read as that value:
/// `Cast Goblin Piker (tap Mountain, Forest)`, a menu row copied into a
/// plan, is `Goblin Piker`. The request no longer carries the enum
/// ([`cache_stable_schema`]), so decoding does not force the exact name,
/// and an answer whose meaning is plain is not asked again for it.
///
/// "About" is the value the string starts with, past a `Cast` or `Play`
/// and with its parenthesised parts left out — not any value it contains.
/// `Flashback Think Twice (tap Island, Island, Island)`, a row for no card
/// in hand, held one: an Island, and the plan played an Island nobody asked
/// for (#863). It is asked again now, told what the plan can name.
#[must_use]
pub fn repair_enum_strings(schema: &serde_json::Value, answer: &serde_json::Value) -> serde_json::Value {
    if let (Some(allowed), Some(text)) = (schema.get("enum").and_then(serde_json::Value::as_array), answer.as_str()) {
        if allowed.contains(answer) {
            return answer.clone();
        }
        // A copied menu row may keep its index or a bullet: `3: Cast X`,
        // `3) Cast X`, `- Cast X`. Not a bare "2." or a "-1": those are
        // the answer itself.
        let lower = text.to_lowercase();
        let lower = lower.trim();
        let digits = lower.len() - lower.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let row = match lower[digits..].strip_prefix([':', '.', ')']) {
            Some(rest) if digits > 0 && !rest.trim().is_empty() => rest,
            _ => lower.strip_prefix("- ").or_else(|| lower.strip_prefix("* ")).unwrap_or(lower),
        };
        let mut bare = String::new();
        let mut depth = 0usize;
        for c in row.chars() {
            match c {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ if depth == 0 => bare.push(c),
                _ => {}
            }
        }
        let bare = bare.trim();
        let subject = bare.strip_prefix("cast ").or_else(|| bare.strip_prefix("play ")).unwrap_or(bare);
        // The whole value, to a word's end: "10" is not "1", "all to B"
        // is not pile "A".
        let held: Vec<&str> = allowed.iter().filter_map(serde_json::Value::as_str)
            .filter(|v| subject.strip_prefix(v.to_lowercase().as_str())
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric())))
            .collect();
        let longest = held.iter().map(|v| v.len()).max().unwrap_or(0);
        let mut best = held.into_iter().filter(|v| v.len() == longest);
        return match (best.next(), best.next()) {
            (Some(only), None) => serde_json::json!(only),
            _ => answer.clone(),
        };
    }
    match answer {
        serde_json::Value::Object(map) => {
            let props = schema.get("properties").and_then(serde_json::Value::as_object);
            serde_json::Value::Object(map.iter().map(|(k, v)| {
                let fixed = props.and_then(|p| p.get(k)).map_or_else(|| v.clone(), |sub| repair_enum_strings(sub, v));
                (k.clone(), fixed)
            }).collect())
        }
        // `prefixItems` gives entry `i` its own values, as in `schema_violation`.
        serde_json::Value::Array(items) => {
            let prefix = schema.get("prefixItems").and_then(serde_json::Value::as_array);
            serde_json::Value::Array(items.iter().enumerate().map(|(i, v)| {
                prefix.and_then(|p| p.get(i)).or_else(|| schema.get("items"))
                    .map_or_else(|| v.clone(), |item| repair_enum_strings(item, v))
            }).collect())
        }
        _ => answer.clone(),
    }
}

/// `answer` with each list nothing may go in (`maxItems` 0) that it left
/// out put in as `[]`. The format line does not offer such a list and the
/// check does not require it (#897), so the callers read an answer without
/// it as the empty list it stands for, not as "no usable list".
#[must_use]
pub fn with_empty_lists(schema: &serde_json::Value, answer: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(mut object) = answer else { return answer };
    for (key, spec) in schema.get("properties").and_then(serde_json::Value::as_object).into_iter().flatten() {
        if spec.get("maxItems").and_then(serde_json::Value::as_u64) == Some(0) && !object.contains_key(key) {
            object.insert(key.clone(), serde_json::json!([]));
        }
    }
    serde_json::Value::Object(object)
}

/// Why `answer` is not one `schema` allows, if it is not: the check a
/// request without the per-call constraints ([`cache_stable_schema`]) no
/// longer asks the API to make. A missing `thoughts` is not a reason.
#[must_use]
pub fn schema_violation(schema: &serde_json::Value, answer: &serde_json::Value) -> Option<String> {
    fn listed(values: &[serde_json::Value]) -> String {
        if let Some(ints) = answer_text::numeric_enum(values) {
            let (lo, hi) = (ints.iter().min().copied().unwrap_or(0), ints.iter().max().copied().unwrap_or(0));
            if ints.len() == 1 {
                return lo.to_string();
            }
            if ints.len() > 2 && usize::try_from(hi - lo + 1).ok() == Some(ints.len()) {
                return format!("{lo} to {hi}");
            }
            if ints.len() <= 12 {
                return format!("one of {}", ints.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "));
            }
        }
        let shown: Vec<String> = values.iter().take(12).map(ToString::to_string).collect();
        let more = if values.len() > 12 { ", …" } else { "" };
        format!("one of {}{more}", shown.join(", "))
    }
    fn at(schema: &serde_json::Value, value: &serde_json::Value, path: &str) -> Option<String> {
        if let Some(allowed) = schema.get("enum").and_then(serde_json::Value::as_array) {
            // A number whose digits a string enum holds is the amount the
            // question offers, and its reader takes it as one (an X
            // funding's amounts, #596): `0` was asked again for not being
            // `"0"` on every X spell a seat answered in numbers (#930).
            let as_digits = value.is_number() && allowed.contains(&serde_json::json!(value.to_string()));
            if as_digits {
                return None;
            }
            if !allowed.contains(value) {
                return Some(format!("`{path}` is {value}; it must be {}", listed(allowed)));
            }
        }
        let count = |key: &str| schema.get(key).and_then(serde_json::Value::as_u64)
            .and_then(|n| usize::try_from(n).ok());
        match schema.get("type").and_then(serde_json::Value::as_str) {
            Some("object") => {
                let Some(object) = value.as_object() else {
                    return Some(format!("`{path}` is {value}, not an object"));
                };
                let properties = schema.get("properties").and_then(serde_json::Value::as_object);
                let unread = |key: &str| properties.and_then(|p| p.get(key))
                    .is_some_and(|sub| sub.get(UNREAD).is_some());
                // Not asked for, so not missing: what the format line
                // leaves out is what this does not require (#897).
                let not_asked = |key: &str| properties.and_then(|p| p.get(key))
                    .is_some_and(answer_text::not_asked);
                let required = schema.get("required").and_then(serde_json::Value::as_array);
                for key in required.into_iter().flatten().filter_map(serde_json::Value::as_str) {
                    if key != "thoughts" && !not_asked(key) && !object.contains_key(key) {
                        let inner = if path.is_empty() { key.to_string() } else { format!("{path}.{key}") };
                        return Some(format!("`{inner}` is missing"));
                    }
                }
                properties.into_iter().flatten().filter(|(key, _)| !unread(key)).find_map(|(key, sub)| {
                    let inner = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                    object.get(key).and_then(|v| at(sub, v, &inner))
                })
            }
            Some("array") => {
                let Some(items) = value.as_array() else {
                    return Some(format!("`{path}` is {value}, not a list"));
                };
                if let Some(min) = count("minItems").filter(|min| items.len() < *min) {
                    return Some(format!("`{path}` has {} entries; it needs at least {min}", items.len()));
                }
                if let Some(max) = count("maxItems").filter(|max| items.len() > *max) {
                    return Some(format!("`{path}` has {} entries; it takes at most {max}", items.len()));
                }
                // `prefixItems` holds entry `i` to its own schema (an X
                // funding source's amounts); `items` holds the rest.
                let prefix = schema.get("prefixItems").and_then(serde_json::Value::as_array);
                items.iter().enumerate().find_map(|(i, v)| {
                    let item = prefix.and_then(|p| p.get(i)).or_else(|| schema.get("items"))?;
                    at(item, v, &format!("{path}[{i}]"))
                })
            }
            Some("integer") if !(value.is_i64() || value.is_u64()) =>
                Some(format!("`{path}` is {value}, not a whole number")),
            Some("string") if !value.is_string() => Some(format!("`{path}` is {value}, not a string")),
            Some("boolean") if !value.is_boolean() => Some(format!("`{path}` is {value}, not true or false")),
            _ => None,
        }
    }
    at(schema, answer, "")
}

/// The schema an ordering prompt is answered through: `order`, a
/// permutation of `0..n`.
///
/// The requirement used to live only in the description, where no provider
/// can enforce it. `[]`, `[0]` and `[0,0,0]` were all schema-valid, and
/// `parse_order_response` — whose contract is a strict permutation —
/// refused each of them and substituted the order as listed, leaving one
/// MALFORMED line behind. That substitution is a strategic decision made
/// for the seat: a damage assignment order decides which blocker dies
/// (CR 510.1c), and the prompt's own rule text says to put the blocker you
/// most want dead first (#547).
///
/// `minItems`/`maxItems` is how every other index-array prompt in the
/// harness bounds itself — `mark_indices` and the mulligan-bottom prompt
/// both emit them — so it is known to be accepted on the paths a seat
/// runs on. `uniqueItems` would say the rest of it and is deliberately not
/// added: nothing in the program uses it, and whether both providers
/// enforce it is the open question of #546. The client-side check stays
/// either way, since a length-bounded answer can still repeat an index.
#[must_use]
pub fn ordering_schema(n: usize) -> serde_json::Value {
    let valid_indices: Vec<serde_json::Value> = (0..n).map(|i| serde_json::json!(i)).collect();
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
            "order": {
                "type": "array",
                "items": {"type": "integer", "enum": valid_indices},
                "minItems": n,
                "maxItems": n,
                // The prompt says how many: a count here would make each
                // length its own schema (`cache_stable_schema`).
                "description": "Every listed index exactly once, first to last"
            }
        },
        "required": ["thoughts", "order"]
    })
}

/// The schema a combat damage division is answered through (CR 510.1c-d,
/// issue #637): `amount`, one integer from `min` (lethal) to `max` (all the
/// damage left). The amount itself, not an index into a list of them: the
/// question is "how much", and an index the model has to offset by `min`
/// is one more place to be wrong.
#[must_use]
pub fn damage_amount_schema(min: u32, max: u32) -> serde_json::Value {
    let amounts: Vec<serde_json::Value> = (min..=max).map(|a| serde_json::json!(a)).collect();
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
            "amount": {
                "type": "integer",
                "enum": amounts,
                // The prompt names the range, for the same reason.
                "description": "Damage assigned to this blocker, from lethal to all that is left"
            }
        },
        "required": ["thoughts", "amount"]
    })
}

/// How long a `claude -p` seat keeps retrying a failing call before it gives
/// up, in the draft and in the games alike.
///
/// The failure a long run actually meets is a usage limit or a transient
/// CLI or network outage, which lasts minutes. Three tries over six seconds
/// ended a draft at pick 300 (#218); the game seat kept that shape after
/// the draft lost it, so the same outage cost the games their decisions
/// (#587). A wall-clock budget says what is meant: keep trying for ten
/// minutes, backing off up to a minute between tries.
pub const RETRY_BUDGET: std::time::Duration = std::time::Duration::from_secs(600);

/// The longest wait between two attempts. Exponential up to here, then flat.
pub const MAX_RETRY_BACKOFF: std::time::Duration = std::time::Duration::from_secs(60);

/// [`RETRY_BUDGET`], unless `env` names a number of seconds to use instead —
/// which is how the give-up path is tested in seconds.
#[must_use]
pub fn retry_budget(env: &str) -> std::time::Duration {
    env_seconds(env, RETRY_BUDGET)
}

/// The longest a seconds override from the environment is taken as: a day.
/// Every such duration is added to an `Instant`, and a sentinel like 2^64
/// panicked there ("overflow when adding duration to instant"), killing
/// the seat's thread (#812's flags, found again in these test knobs).
pub const MAX_ENV_SECONDS: u64 = 86_400;

/// `default`, unless `env` names a number of seconds to use instead, capped
/// at [`MAX_ENV_SECONDS`].
#[must_use]
pub fn env_seconds(env: &str, default: std::time::Duration) -> std::time::Duration {
    std::env::var(env)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(default, |secs| std::time::Duration::from_secs(secs.min(MAX_ENV_SECONDS)))
}

/// How long to wait before attempt number `attempt` (1-based on retries).
#[must_use]
pub fn retry_backoff(attempt: u32) -> std::time::Duration {
    MAX_RETRY_BACKOFF.min(std::time::Duration::from_secs(2u64.pow(attempt.min(6))))
}

/// `https://api.anthropic.com`, unless `ANTHROPIC_BASE_URL` names another
/// server — which is how a seat's failure paths are tested without a metered
/// call (#719). Both request paths, the game's and the draft's, ask here.
#[must_use]
pub fn anthropic_base_url() -> String {
    base_url_from("ANTHROPIC_BASE_URL", "https://api.anthropic.com")
}

/// [`anthropic_base_url`] for Gemini, overridden by `GEMINI_BASE_URL`.
#[must_use]
pub fn gemini_base_url() -> String {
    base_url_from("GEMINI_BASE_URL", "https://generativelanguage.googleapis.com")
}

fn base_url_from(env_name: &str, default: &str) -> String {
    std::env::var(env_name).ok().filter(|u| !u.is_empty())
        .map_or_else(|| default.to_string(), |u| u.trim_end_matches('/').to_string())
}

/// What one attempt at a metered API call came to.
#[derive(Debug)]
pub enum CallAttempt<T> {
    /// The model answered.
    Answer(T),
    /// Worth asking again: a rate limit, an overload, a 5xx, a timeout, a
    /// connection that failed.
    Transient(String),
    /// This request cannot succeed, but the next one may: a 400 the request
    /// itself earned, such as a schema the API refuses (#398).
    Refused(String),
    /// No request from this seat will succeed: the key was refused.
    Dead(String),
}

/// What a whole call came to, after its retries.
#[derive(Debug)]
pub enum CallOutcome<T> {
    Answer(T),
    /// No answer to this call; the seat may still answer the next.
    Failed(String),
    /// No answer, and none is coming: the retry budget is spent, or the key
    /// is dead. The seat has stopped answering, and its runner forfeits it.
    GaveUp(String),
}

/// The class of a non-success HTTP status from a model API.
#[must_use]
pub fn classify_http_status<T>(code: u16, why: String) -> CallAttempt<T> {
    match code {
        401 | 403 => CallAttempt::Dead(why),
        408 | 409 | 429 | 500..=599 => CallAttempt::Transient(why),
        _ => CallAttempt::Refused(why),
    }
}

/// Run `attempt` until it answers, retrying transient failures for as long
/// as `budget` lasts with [`retry_backoff`] between tries.
///
/// The `claude -p` seat waited out a budget and forfeited past it (#587);
/// the two metered seats kept three and six fixed attempts over seconds,
/// returned an empty answer on any other status without saying no answer
/// had happened — so a 400 or a 401 was logged as the model's malformed
/// answer — and never gave up, so a dead key played a match on fallbacks
/// (#719). One loop, every HTTP seat. `on_failure` is told each failed
/// attempt and whether it will be retried, for the seat's own log lines.
pub fn call_within_budget<T>(
    budget: std::time::Duration,
    mut attempt: impl FnMut(u32) -> CallAttempt<T>,
    mut on_failure: impl FnMut(u32, &str, bool),
) -> CallOutcome<T> {
    let began = std::time::Instant::now();
    let deadline = began + budget;
    let mut tries = 0u32;
    let mut last = String::new();
    loop {
        if tries > 0 {
            let backoff = retry_backoff(tries);
            if std::time::Instant::now() + backoff > deadline {
                break;
            }
            std::thread::sleep(backoff);
        }
        tries += 1;
        match attempt(tries) {
            CallAttempt::Answer(t) => return CallOutcome::Answer(t),
            CallAttempt::Transient(why) => {
                on_failure(tries, &why, true);
                last = why;
            }
            CallAttempt::Refused(why) => {
                on_failure(tries, &why, false);
                return CallOutcome::Failed(why);
            }
            CallAttempt::Dead(why) => {
                on_failure(tries, &why, false);
                return CallOutcome::GaveUp(format!("gave up: the API refused this seat's key: {why}"));
            }
        }
    }
    CallOutcome::GaveUp(format!(
        "gave up after {tries} attempt{} over {}s (retry budget {}s); last: {last}",
        if tries == 1 { "" } else { "s" }, began.elapsed().as_secs(), budget.as_secs()))
}

#[must_use]
pub fn thinking_param(model: &str) -> serde_json::Value {
    if model_takes_a_thinking_budget(model) {
        serde_json::json!({ "type": "enabled", "budget_tokens": 4096 })
    } else {
        serde_json::json!({ "type": "adaptive" })
    }
}

/// Whether this model family still steers thinking with `budget_tokens`
/// (Haiku 4.5, the 4.5 Sonnet, the 3.x models) rather than adaptive
/// thinking plus `output_config.effort`. Not every Haiku: the current one
/// takes adaptive thinking and refuses a budget with a 400, which matching
/// on "haiku" sent it.
fn model_takes_a_thinking_budget(model: &str) -> bool {
    model.contains("-4-5") || model.contains("-3-")
}

/// How hard a seat thinks per decision.
///
/// Thinking output is billed at output rates and, once the conversation
/// history is bounded, it is the largest term a decision has left — a seat
/// that reasons at length over "pass or Bolt the 2/1" spends more on the
/// reasoning than on reading the board. The default is off: the seat
/// reasons in its two sentences of `thoughts`, before its answer. Replayed
/// over real positions against a stronger model's reference, the default
/// seat at `off` agreed with the reference as often as at `low`, and `low`
/// spent 84-130 thinking tokens a decision on top (reports/llm-cost.md,
/// phase 3). [`THINKING_ENV`] raises it for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel {
    /// No thinking at all, where the model allows that.
    Off,
    Low,
    Medium,
    High,
    /// An explicit token budget. Models that take only an effort level
    /// get the nearest one.
    Budget(u32),
}

/// `MTG_LLM_THINKING`: `off` (the default), `low`, `medium`, `high`, or a
/// token budget such as `2048`.
pub const THINKING_ENV: &str = "MTG_LLM_THINKING";

/// The level this run's seats think at, from [`THINKING_ENV`].
///
/// An unrecognised value falls back to the default and says so once: a
/// typo that silently ran a whole night of games at the default would be
/// found only in the bill.
#[must_use]
pub fn thinking_level() -> ThinkingLevel {
    static WARNED: std::sync::Once = std::sync::Once::new();
    let Ok(raw) = std::env::var(THINKING_ENV) else { return ThinkingLevel::Off };
    if let Some(level) = parse_thinking_level(&raw) {
        return level;
    }
    WARNED.call_once(|| {
        crate::stderr_line!(
            "Warning: {THINKING_ENV}={raw:?} is not off, low, medium, high or a token \
             count; thinking stays off.");
    });
    ThinkingLevel::Off
}

/// [`thinking_level`]'s parser, `None` for a value it does not know.
#[must_use]
pub fn parse_thinking_level(raw: &str) -> Option<ThinkingLevel> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "low" => Some(ThinkingLevel::Low),
        "" | "off" | "none" => Some(ThinkingLevel::Off),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        n => n.parse::<u32>().ok().map(|n| if n == 0 { ThinkingLevel::Off } else { ThinkingLevel::Budget(n) }),
    }
}

/// Set `thinking`, `output_config.effort` and `max_tokens` on a Messages
/// API request body for this model at this level.
///
/// The two model families take different parameters (see
/// [`thinking_param`]): an adaptive model is steered by `effort` and
/// rejects a budget, an older one needs the budget and rejects `effort`.
/// `off` omits the `thinking` parameter altogether, which is "no thinking"
/// on the models that allow it and the model's default on the ones that
/// do not (the 5.x family thinks regardless; `effort: low` still applies).
///
/// # Panics
/// If `body` is not a JSON object — a request body always is.
pub fn apply_thinking(body: &mut serde_json::Value, model: &str, level: ThinkingLevel) {
    let obj = body.as_object_mut().expect("a request body is a JSON object");
    obj.remove("thinking");
    let mut max_tokens: u64 = max_output_tokens(level);
    if model_takes_a_thinking_budget(model) {
        let budget = match level {
            ThinkingLevel::Off => None,
            ThinkingLevel::Low => Some(1024),
            ThinkingLevel::Medium => Some(4096),
            ThinkingLevel::High => Some(16384),
            // The API's minimum is 1024.
            ThinkingLevel::Budget(n) => Some(u64::from(n).max(1024)),
        };
        if let Some(budget) = budget {
            obj.insert("thinking".into(), serde_json::json!({"type": "enabled", "budget_tokens": budget}));
            // The budget must be below `max_tokens`, with room for the answer.
            max_tokens = max_tokens.max(budget + ANSWER_TOKENS);
        }
    } else {
        if level != ThinkingLevel::Off {
            obj.insert("thinking".into(), serde_json::json!({"type": "adaptive"}));
        }
        let effort = effort_for(level);
        let output_config = obj.entry("output_config").or_insert_with(|| serde_json::json!({}));
        output_config["effort"] = serde_json::json!(effort);
    }
    obj.insert("max_tokens".into(), serde_json::json!(max_tokens));
}

/// The `effort` a level is sent at, on a model steered by effort rather
/// than a thinking budget — on the Messages API (`output_config.effort`)
/// and on a `claude -p` seat (`--effort`) alike.
#[must_use]
pub fn effort_for(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off | ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::Budget(n) if n <= 2048 => "low",
        ThinkingLevel::Budget(n) if n <= 8192 => "medium",
        ThinkingLevel::High | ThinkingLevel::Budget(_) => "high",
    }
}

/// Whether a seat on this model is steered by `effort` — every current
/// model but the 4.5 and 3.x families, which take a thinking budget. A
/// `claude -p` seat with no model named runs the CLI's default, a current
/// model.
#[must_use]
pub fn model_takes_effort(model: Option<&str>) -> bool {
    model.is_none_or(|m| !model_takes_a_thinking_budget(m))
}

/// Output tokens the largest structured answer needs, with room to spare:
/// an ordering of twelve triggers is ≈ 100 tokens of JSON, a blockers
/// answer for six creatures ≈ 120, the draft runner's 40-card deck — a
/// count per pool row, five basics, two sentences of `thoughts` — well
/// under the ≈ 600 it took when it named every card (`mtg-draft-runner`'s
/// `deck_schema_for`). The
/// measured game answer is 60–120 tokens of JSON (`reports/llm-cost.md`).
pub const ANSWER_TOKENS: u64 = 2048;

/// The `max_tokens` a request is sent at this level — the cap on a
/// runaway, not a budget the model is expected to use.
///
/// Thinking counts against `max_tokens` on the Messages API and on a
/// `claude -p` seat alike (`CLAUDE_CODE_MAX_OUTPUT_TOKENS` is the CLI's
/// `max_tokens`), so the cap is the answer plus the thinking the level
/// asks for. The default `low` was measured at 728 thinking tokens per
/// decision on average and under 2,000 at most over two games
/// (`reports/llm-cost.md`, phase 2), so 4,096 is twice the largest
/// decision seen and half the 8,192 it replaced. `off` has the same room:
/// the newest models think when thinking is not asked for, and some cannot
/// turn it off at all — the current Haiku at effort `low` was measured at
/// up to 1,425 output tokens a decision, within reach of the answer's own
/// 2,048 (reports/llm-cost.md, phase 3). A truncated answer is a wasted
/// call — the harness reads no JSON and falls back — which is why the cap
/// is not tighter.
#[must_use]
pub fn max_output_tokens(level: ThinkingLevel) -> u64 {
    match level {
        ThinkingLevel::Off | ThinkingLevel::Low => 4096,
        ThinkingLevel::Medium => 8192,
        ThinkingLevel::High => 16384,
        ThinkingLevel::Budget(n) => u64::from(n) + ANSWER_TOKENS,
    }
}

/// The `MAX_THINKING_TOKENS` a `claude -p` child is given at this level,
/// which is the one thinking knob the CLI exposes; `None` leaves the
/// CLI's own default.
///
/// The default stays at `low`: measured on a one-line blocking question,
/// the CLI's default spent ~730 thinking tokens and `MAX_THINKING_TOKENS=
/// 1024` spent ~1450, so naming a small budget is not a way down from the
/// default on this path. `0` is: it turns thinking off (`thinking_tokens:
/// 0` in the result's usage).
#[must_use]
pub fn claude_code_thinking_tokens(level: ThinkingLevel) -> Option<u32> {
    match level {
        ThinkingLevel::Off => Some(0),
        ThinkingLevel::Low => None,
        ThinkingLevel::Medium => Some(4096),
        ThinkingLevel::High => Some(16384),
        ThinkingLevel::Budget(n) => Some(n),
    }
}

/// `MTG_LLM_HISTORY`: how many earlier exchanges a decision is sent along
/// with its own prompt. The default is none.
///
/// Every decision prompt restates the whole position — the board, the
/// hand, the stack, the events since the last prompt, every legal action
/// — so the exchanges before it are not information the seat lacks, they
/// are the same information again. A seat that re-sent its whole history
/// grew quadratically in the number of decisions: input tokens per game
/// went from a few hundred thousand to tens of millions over a long game,
/// and prompt caching only turned the price into a tenth of itself. The
/// seat now sends the system prompt, at most this many earlier exchanges
/// and the current prompt, and carries its plan across decisions in its
/// own notes (see `LlmPlayer::notes`).
///
/// The Messages API seat keeps a sliding window of this many exchanges.
/// The Gemini and `claude -p` seats cannot slide a server-held
/// conversation, so they start a fresh one every this-many decisions.
pub const HISTORY_ENV: &str = "MTG_LLM_HISTORY";

/// Say once that `var` holds `value`, which it does not know, and what the
/// run does instead. A setting taken as its default in silence ran the
/// opposite of what was asked with no sign of it (#911).
pub fn unknown_setting(var: &str, value: &str, instead: &str) {
    static WARNED: std::sync::LazyLock<Mutex<std::collections::HashSet<String>>> =
        std::sync::LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));
    if WARNED.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(var.to_string()) {
        crate::stderr_line!("Warning: {var}={value:?} is not a value it takes; {instead}.");
    }
}

/// Whether `var` is set to `1`, as the on-or-off settings are; `0`, empty
/// or unset is off, and anything else is off and said.
#[must_use]
pub fn env_flag(var: &str) -> bool {
    match std::env::var(var).as_deref().map(str::trim) {
        Ok("1") => true,
        Err(_) | Ok("" | "0") => false,
        Ok(other) => {
            unknown_setting(var, other, "it is off (1 turns it on)");
            false
        }
    }
}

/// The history window this run's seats use, from [`HISTORY_ENV`].
#[must_use]
pub fn history_exchanges() -> usize {
    let Ok(value) = std::env::var(HISTORY_ENV) else { return 0 };
    value.trim().parse().unwrap_or_else(|_| {
        if !value.trim().is_empty() {
            unknown_setting(HISTORY_ENV, &value, "no history is kept (a whole number of exchanges sets one)");
        }
        0
    })
}

/// `MTG_LLM_PASS_UNTIL`: whether a priority offer carries the
/// [`PASS_UNTIL_ROW`]. `off` (or `0`, `false`, `no`) leaves the row out,
/// which is how a run is measured against one without it; unset or
/// anything else keeps it.
pub const PASS_UNTIL_ENV: &str = "MTG_LLM_PASS_UNTIL";

/// Whether this run's seats are offered the pass-until row, from
/// [`PASS_UNTIL_ENV`].
#[must_use]
pub fn pass_until_enabled() -> bool {
    !std::env::var(PASS_UNTIL_ENV).is_ok_and(|v| {
        matches!(v.trim().to_ascii_lowercase().as_str(), "off" | "0" | "false" | "no")
    })
}

/// The one row every priority offer carries beside the engine's actions,
/// right after `Pass`: the seat's way of saying "go".
///
/// A person holding an instant does not deliberate at every priority of
/// both players' turns; they say "go" and respond when something happens.
/// The browser page has this as the `f` key and the CLI as its auto-pass
/// mode. For an LLM seat every priority it is asked is a model call —
/// nine in ten calls of a game were a seat holding a Lightning Bolt being
/// asked at every step (`reports/llm-cost.md`) — so the row is offered
/// where the keystroke would be, and the answers it stands in for are not
/// sent anywhere. What it stops for is listed once, in
/// [`PASS_UNTIL_RULES`], beside [`LlmPlayer::pass_until_stop`], which is
/// that list in code. The row used to restate a shorter list in every
/// priority prompt — about 300 B a prompt, and three stops short of the
/// one applied (#756, #757).
pub const PASS_UNTIL_ROW: &str = "Pass until something happens";

/// The system prompt's account of [`PASS_UNTIL_ROW`], sent only to a seat
/// that is offered the row: with `MTG_LLM_PASS_UNTIL=off` it told the seat
/// index 1 was the row when index 1 was an engine action (#755). The stops
/// are the arms of [`LlmPlayer::pass_until_stop`]; a new entry on the stack
/// is any, the seat's own triggers included (#756).
const PASS_UNTIL_RULES: &str = r#"

## Pass until something happens

Every priority offer has one more row, right after `Pass`: `1: Pass until something happens`, with the engine's rows after it numbered from 2. It is the way a person says "go", and the usual way to pass: answer it whenever you would answer `Pass` again at the next offer, and keep plain `Pass` for a window you mean to use later this turn (an instant at the end step). It passes now and keeps passing later priority offers for you, unasked, until something of the opponent's is put on the stack; a blocker is declared against your attacker; the opponent attacks you and blocks are in (your window after blocks); your next first main phase; a later main phase of yours with a land drop or a sorcery-speed cast or activation on offer; or, once the opponent has put something on the stack since you said go, any prompt that is not a plain priority pass (combat declarations, targets and choices are always asked, except an attack you named in your first main phase). After you declare attackers you pass this way too, until a block or your second main phase. You are never asked to respond to your own spell or ability while only yours are on the stack. When it stops, the next prompt says why in one line, e.g. `After 6 priority offers passed for you unasked, the pass stopped: Grizzly Bears (the opponent's) is on the stack.`"#;

/// The seat said "go": it is passing every plain priority offer without a
/// model call until something it would want to see happens. What it was
/// looking at when it said so, so a later offer can be told apart from it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct PassUntil {
    turn: u32,
    step: Step,
    /// Whose turn it was: a pass-until engaged on the opponent's turn
    /// ends when the seat's own turn starts.
    our_turn: bool,
    /// The stack as it stood, top first as the view lists it, by each
    /// entry's identity. An entry resolving off the top is expected;
    /// anything else on the stack is new.
    stack: Vec<(ObjectId, mtg_engine::ids::CardId, String)>,
    /// The engine's count of entries ever put on the stack. A second
    /// activation of an ability already on it, or a repeat of a trigger,
    /// has the same identity as the first; the count moving is what says
    /// it is new (#750).
    stack_puts: u64,
    /// The engine's count of the opponent's entries put on the stack
    /// (`GameView::opponent_stack_puts`). The engine passes for a seat with
    /// nothing to respond with, so an instant or a flash creature can come
    /// and go between two offers this seat sees (#861).
    #[serde(default = "puts_unknown")]
    opp_puts: u64,
    /// How many priority offers have been passed unasked since.
    passes: u32,
    /// The seat said it was done with the main phase this was engaged in
    /// (its plan ran out, or `ask_again` was false): the land-drop and
    /// sorcery-speed stops do not ask it that main phase again.
    #[serde(default)]
    done_here: bool,
}

/// What the CR 616.1 prompt is asking, sent with it: the options are a
/// numbered list like any other, and what "first" means is not.
pub const DAMAGE_EFFECT_RULE: &str = "Two or more replacement or prevention effects apply to this damage \
and their order changes the result, so the affected player — the one dealt the damage, or the \
controller of the creature dealt it — chooses (CR 616.1). Pick the effect to apply FIRST; the others \
apply after only if they still can: a prevention or a mill ends the damage, doubling leaves more for \
the rest.";

/// The mulligan advice the mulligan prompt carries.
pub const MULLIGAN_ADVICE: &str = "Mulligan a 0- or 7-lander or a hand with no play in its first three \
turns; keep 2-4 lands and a curve. More than two mulligans is rarely right.";

/// The rest of a main phase the seat laid out in one answer: cards from its
/// hand to play or cast after the action it chose, in order, each taken
/// without asking it again while nothing else happens. A person says "land,
/// then the Tusker" once; asking for each was a model call per card that
/// restated the plan the seat had just made (reports/llm-cost.md, phase 3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Plan {
    turn: u32,
    step: Step,
    /// Card names still to play or cast, next first.
    steps: Vec<String>,
    /// How many have been taken, for the log and the stop line.
    taken: u32,
    /// The seat asked to be asked again once the plan is done, rather than
    /// to pass until something happens (`ask_again`).
    #[serde(default)]
    ask_again: bool,
    /// `GameView::opponent_stack_puts` when the plan was made (#861).
    #[serde(default = "puts_unknown")]
    opp_puts: u64,
}

/// The attack a seat named in its first main phase: declared at its
/// declare-attackers step without asking it again, if nothing has happened
/// in between and each creature can still attack. A person says "Bears,
/// then attack with the Tusker" once; the declaration was a call of its own
/// on most of a seat's turns — a quarter of a game's calls (reports/llm-cost.md,
/// phase 3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct AttackPlan {
    turn: u32,
    attackers: Vec<ObjectId>,
    /// `GameView::opponent_stack_puts` when the attack was named: anything
    /// the opponent casts or activates after it — a flash blocker the
    /// engine passed this seat through — is "something happening" (#861).
    #[serde(default = "puts_unknown")]
    opp_puts: u64,
}

/// What the `attack` field asks for.
pub const ATTACK_DESCRIPTION: &str =
    "In your first main phase (else []): the # numbers of your creatures to attack the opponent with this turn, once this main \
     phase is done. They are declared without asking you if nothing happens first and each \
     can still attack; otherwise you are asked at combat. [] to be asked at combat.";

/// A save from before the count was kept: nothing is taken to have
/// happened since.
fn puts_unknown() -> u64 {
    u64::MAX
}

/// Why a plan, an attack plan or a pass-until stops for something of the
/// opponent's that is no longer on the stack to name.
const OPPONENT_ACTED: &str = "the opponent cast or activated something since (it has resolved)";

/// What a plan does at an offer: take its next card, finish, or stop.
enum PlanMove {
    Act(Action),
    Done,
    Stop(String),
}

/// What the `then` field asks for. Its own description, not a system
/// prompt section: the field is offered only where a plan can be made.
pub const PLAN_DESCRIPTION: &str =
    "In your own main phase with an empty stack (else []): cards from your hand, by name alone, \
     to play or cast this main phase AFTER `action`, in order, each \
     without being asked again once the stack is empty (targets are still asked). Put your \
     first play in `action` and the rest here; if `action` is a pass, these start now instead. \
     The plan stops and you are asked if the opponent puts anything on the stack or a card \
     cannot be played when its turn comes. [] for nothing after `action`.";

/// What the `ask_again` field asks for, beside [`PLAN_DESCRIPTION`]. An
/// empty plan used to mean "ask me again", so a seat that played its land
/// and had nothing else to do was asked the main phase a second time to
/// say so — one call in six of a game (reports/llm-cost.md, phase 3).
pub const ASK_AGAIN_DESCRIPTION: &str =
    "In your own main phase (else false): false, the usual answer, when `action` and `then` are all you will do this main phase: \
     you then pass until something happens, and your attack is still asked. true to be asked \
     again here once they are done, e.g. to play off what a spell drew.";

/// What an LLM seat carries from one decision to the next, and so what a
/// save has to hold for a resumed seat to be asked exactly what it would
/// have been asked live.
///
/// A decision is sent no history (see [`HISTORY_ENV`]): the seat's whole
/// memory is its notes, how far into the game log it has been shown, and
/// whether it is passing until something happens. A save that held only
/// the notes (#836) resumed a seat that had to be handed the complete game
/// log as a recap — thousands of tokens on one prompt, and a prompt no live
/// decision ever looks like. With this in the save, a resumed seat's next
/// prompt is byte-for-byte the one it would have had, which is also what
/// lets a saved position be replayed as one decision
/// (`mtg-runner --stop-after`, `scripts/decision-eval/`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeatMemory {
    notes: Option<String>,
    /// How many entries of the seat's `display_log` it has been shown.
    log_cursor: usize,
    pass_until: Option<PassUntil>,
    pass_until_stopped: Option<(u32, String)>,
    #[serde(default)]
    plan: Option<Plan>,
    #[serde(default)]
    plan_stopped: Option<String>,
    #[serde(default)]
    attack_plan: Option<AttackPlan>,
}

impl SeatMemory {
    /// The notes this memory carries, for a reader of the save.
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

impl PassUntil {
    fn engaged_at(view: &GameView) -> Self {
        PassUntil {
            turn: view.turn_number,
            step: view.step,
            our_turn: view.active_player == view.you,
            stack: Self::stack_identity(view),
            stack_puts: view.stack_puts,
            opp_puts: view.opponent_stack_puts,
            passes: 0,
            done_here: false,
        }
    }

    /// Engaged by a seat that said it was done with this main phase.
    fn done_at(view: &GameView) -> Self {
        PassUntil { done_here: true, ..Self::engaged_at(view) }
    }

    fn stack_identity(view: &GameView) -> Vec<(ObjectId, mtg_engine::ids::CardId, String)> {
        view.stack.iter().map(|s| (s.object_id, s.card_id, s.name.clone())).collect()
    }

    /// Where it was engaged, for the log.
    fn since(&self) -> String {
        format!("turn {} {} ({})", self.turn, LlmPlayer::step_name(self.step, false),
            if self.our_turn { "your turn" } else { "opp's turn" })
    }
}

/// One line of the priority menu an LLM seat is sent: an engine offer, or
/// the seat's own [`PASS_UNTIL_ROW`], which is not an engine action and so
/// is not a `DisplayEntry` (the surface-parity check holds those against
/// the CLI's menu, and this row has no CLI row — its CLI counterpart is a
/// keystroke).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuEntry {
    Engine(DisplayEntry),
    PassUntil,
}

fn record_llm_usage(model: &str, input: u64, output: u64, cache_read: u64, cache_create: u64) {
    let mut map = LLM_MODEL_USAGE.lock().unwrap();
    let entry = map.entry(model.to_string()).or_default();
    entry.calls += 1;
    entry.input += input;
    entry.output += output;
    entry.cache_read += cache_read;
    entry.cache_create += cache_create;
}

/// How much of a call's output was thinking, for a backend that reports it.
fn record_llm_thinking(model: &str, thinking: u64) {
    LLM_MODEL_USAGE.lock().unwrap().entry(model.to_string()).or_default().thinking += thinking;
}

/// The suffix a usage line carries when some of its calls came back with an
/// answer the harness could not use.
///
/// Both runners print it, from here: a call count alone reads as a healthy
/// seat even when the harness chose every move itself (#211), and the draft
/// runner's summary, which kept its own copy of the usage struct, dropped
/// the counter one line before it would have printed this (#489).
#[must_use]
pub fn rejected_note(rejected: u64) -> String {
    if rejected == 0 {
        String::new()
    } else {
        format!(", {rejected} answer{} rejected → fallback", if rejected == 1 { "" } else { "s" })
    }
}

/// The suffix for decisions the backend never answered.
///
/// Printed beside `rejected_note` rather than added into it: a dead CLI and
/// a bad answer are different events with different remedies (#587).
#[must_use]
pub fn unanswered_note(unanswered: u64) -> String {
    if unanswered == 0 {
        String::new()
    } else {
        format!(", {unanswered} decision{} the backend never answered → fallback",
            if unanswered == 1 { "" } else { "s" })
    }
}

/// A call whose answer was unusable. See `LlmModelUsage::rejected`.
fn record_llm_rejected(model: &str, seat: &str) {
    let mut map = LLM_MODEL_USAGE.lock().unwrap();
    map.entry(model.to_string()).or_default().rejected += 1;
    drop(map);
    *REJECTED_BY_SEAT.lock().unwrap().entry(seat.to_string()).or_default() += 1;
}

/// A decision the backend never answered. See `LlmModelUsage::unanswered`.
fn record_llm_unanswered(model: &str, seat: &str) {
    let mut map = LLM_MODEL_USAGE.lock().unwrap();
    map.entry(model.to_string()).or_default().unanswered += 1;
    drop(map);
    *UNANSWERED_BY_SEAT.lock().unwrap().entry(seat.to_string()).or_default() += 1;
}

/// Rejected answers by seat as well as by model.
///
/// A tournament's seats are all the same model — every `cc` seat is logged
/// as `claude-code` — so a per-model tally can say that answers were
/// rejected but not whose, while the draft phase names the seat for a
/// substituted pick or deck (#195, #200). This is the game phase's version
/// of that (issue #489).
static REJECTED_BY_SEAT: std::sync::LazyLock<Mutex<HashMap<String, u64>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Decisions each seat's backend never answered, by seat.
static UNANSWERED_BY_SEAT: std::sync::LazyLock<Mutex<HashMap<String, u64>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// How many decisions each seat's backend never answered at all.
#[must_use]
pub fn get_unanswered_by_seat() -> HashMap<String, u64> {
    UNANSWERED_BY_SEAT
        .lock()
        .unwrap()
        .iter()
        .map(|(seat, n)| (seat.clone(), *n))
        .collect()
}

/// How many answers each seat gave that the harness could not use.
#[must_use]
pub fn get_rejected_by_seat() -> HashMap<String, u64> {
    REJECTED_BY_SEAT
        .lock()
        .unwrap()
        .iter()
        .map(|(seat, n)| (seat.clone(), *n))
        .collect()
}

fn record_anthropic_llm_usage(model: &str, json: &serde_json::Value) {
    let usage = &json["usage"];
    record_llm_usage(
        model,
        usage["input_tokens"].as_u64().unwrap_or(0),
        usage["output_tokens"].as_u64().unwrap_or(0),
        usage["cache_read_input_tokens"].as_u64().unwrap_or(0),
        usage["cache_creation_input_tokens"].as_u64().unwrap_or(0),
    );
}

fn record_gemini_llm_usage(model: &str, usage: &serde_json::Value) {
    let input_tokens = usage["total_input_tokens"].as_u64().unwrap_or(0);
    let cached_tokens = usage["total_cached_tokens"].as_u64().unwrap_or(0);
    let uncached_input = input_tokens.saturating_sub(cached_tokens);
    record_llm_usage(
        model,
        uncached_input,
        usage["total_output_tokens"].as_u64().unwrap_or(0),
        cached_tokens,
        0,
    );
}

/// Get the per-model usage map.
///
/// # Panics
/// Panics if the global `LLM_MODEL_USAGE` mutex is poisoned (i.e. another
/// thread panicked while holding the lock).
pub fn get_llm_model_usage() -> HashMap<String, LlmModelUsage> {
    LLM_MODEL_USAGE.lock().unwrap().clone()
}

/// Format a `reqwest::Error` with kind tags and the underlying source chain.
/// Produces something like: "[timeout,connect] error sending request: ... → Connection refused"
fn format_reqwest_error(e: &reqwest::Error) -> String {
    let mut tags = Vec::new();
    if e.is_timeout() { tags.push("timeout"); }
    if e.is_connect() { tags.push("connect"); }
    if e.is_request() { tags.push("request"); }
    if e.is_body() { tags.push("body"); }
    if e.is_decode() { tags.push("decode"); }
    if e.is_redirect() { tags.push("redirect"); }
    if e.is_status() { tags.push("status"); }

    let tag_str = if tags.is_empty() { String::new() } else { format!("[{}] ", tags.join(",")) };

    // Walk the source chain to get the underlying cause.
    let mut chain = vec![e.to_string()];
    let mut src: Option<&dyn std::error::Error> = std::error::Error::source(e);
    while let Some(s) = src {
        chain.push(s.to_string());
        src = s.source();
    }
    format!("{}{}", tag_str, chain.join(" → "))
}

/// Every face a card name refers to: the card itself, and a
/// double-faced card's back face under its own name.
///
/// A DFC's back face used to appear nowhere in the conversation — not in
/// the decklist, not in the card reference, and not on the board after the
/// permanent flipped. A seat was asked "pay {2}{B}{B} to transform?" with
/// no statement anywhere of what it transforms into, under a system prompt
/// that forbids it to assume anything the prompt does not say (issue
/// #205). `back_face_data` was already populated for these cards; nothing
/// consulted it.
///
/// Both faces are returned in printed order, so a caller renders them the
/// same way it renders any card.
#[must_use]
pub fn card_faces(
    name: &str,
    registry: &mtg_engine::cards::CardRegistry,
) -> Vec<(String, mtg_engine::cards::CardData)> {
    // A decklist may name a DFC either by its front face or as
    // "Front // Back"; the registry knows both, and the front face is what
    // the card's data is under.
    let lookup = name.split(" // ").next().unwrap_or(name);
    // A transformed permanent is named by its back face; that names the
    // same card, both faces, so an opponent's Insectile Aberration in view
    // is described rather than printed as an empty entry (#785's review).
    let Some(id) = registry.get_id_by_name(lookup)
        .or_else(|| registry.get_id_by_back_name(lookup)) else { return Vec::new() };
    let Some(front) = registry.card_data(id) else { return Vec::new() };

    let mut faces = vec![(front.name.clone(), front)];
    if let Some(back) = registry.get(id).and_then(mtg_engine::cards::CardBehavior::back_face_data) {
        faces.push((back.name.clone(), back));
    }
    faces
}

/// Rules text without its reminder text: a parenthetical that explains a
/// keyword or a mechanic — `Lifelink (Damage dealt by this creature also
/// causes you to gain that much life.)` — is dropped, the keyword kept.
/// The seat knows what lifelink and flashback mean, and the reminders were
/// a tenth of a drafted deck's reference, re-read on every decision
/// (reports/llm-cost.md, phase 3). A parenthetical that is not a sentence
/// (`{2}{G}{W}, {T}: ...` has none; a name in brackets would) is kept.
#[must_use]
pub fn without_reminder_text(oracle: &str) -> String {
    let mut out = String::with_capacity(oracle.len());
    for (k, line) in oracle.lines().enumerate() {
        let mut kept = String::with_capacity(line.len());
        let mut rest = line;
        while let Some(open) = rest.find('(') {
            let Some(close) = rest[open..].find(')').map(|i| open + i) else { break };
            let inner = &rest[open + 1..close];
            if inner.ends_with('.') || inner.ends_with(".\"") {
                kept.push_str(rest[..open].trim_end());
                rest = &rest[close + 1..];
            } else {
                kept.push_str(&rest[..=close]);
                rest = &rest[close + 1..];
            }
        }
        kept.push_str(rest);
        let kept = kept.trim();
        if kept.is_empty() {
            continue;
        }
        if k > 0 && !out.is_empty() {
            out.push('\n');
        }
        out.push_str(kept);
    }
    out
}

/// One card face on one line: name, cost, type line and size.
///
/// The heading of its card reference entry, and the whole of its line
/// wherever a list has to stay one row per card — a draft pack, a drafted
/// pool — so that the two surfaces describe a card the same way.
#[must_use]
pub fn card_headline(face_name: &str, data: &mtg_engine::cards::CardData) -> String {
    let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
    let type_line = mtg_engine::types::type_line(&data.supertypes, &data.card_types, &data.subtypes);
    let pt = match (data.power, data.toughness) {
        (Some(p), Some(t)) => format!(" {p}/{t}"),
        _ => String::new(),
    };
    format!("{face_name}{cost} | {type_line}{pt}")
}

/// One card's line in a card reference: name, cost, type line and P/T,
/// then its rules text indented under it. Both faces of a double-faced
/// card are listed, each under its own name: the back face is what a
/// transform decision is about, and what the board line reads after the
/// permanent flips (issue #205).
#[must_use]
pub fn card_reference_entry(name: &str, registry: &mtg_engine::cards::CardRegistry) -> String {
    let mut s = String::new();
    for (face_name, data) in card_faces(name, registry) {
        writeln!(s, "{}", card_headline(&face_name, &data)).unwrap();
        let text = without_reminder_text(&data.oracle_text);
        if !text.is_empty() {
            writeln!(s, "  {}", text.replace('\n', "\n  ")).unwrap();
        }
    }
    s
}

/// A card reference for `names`, sorted and deduplicated, one
/// [`card_reference_entry`] each.
#[must_use]
pub fn build_card_reference(names: &[String], registry: &mtg_engine::cards::CardRegistry) -> String {
    let mut names: Vec<&String> = names.iter().collect();
    names.sort();
    names.dedup();
    names.iter().map(|n| card_reference_entry(n, registry)).collect()
}

/// The match a seat is playing, and where in it this game sits.
///
/// This used to be a paragraph of the fixed `GAME_RULES` const reading
/// "Matches are best-of-three ... In this tournament", which every seat was
/// handed — including a `--best-of 1` draft, where there is no game 2, and a
/// plain `mtg-runner` game, which is not a tournament at all. Match
/// structure is decision-relevant (how far to go on a risky race, whether to
/// concede a lost game quickly), so a seat told it has two more games plays
/// game 1 differently from one that knows the match is decided now (issue
/// #210).
///
/// The format alone was not enough. `BestOf` carried only the length, a
/// constant for the whole tournament, and `play_match` re-initialises both
/// conversations before every game — so all 3,232 decisions of a 16-game
/// best-of-4 tournament were answered against 4 distinct system prompts, one
/// per seat, identical between game 1 and game 4 and between a 0-0 match and
/// a 2-1 one. A seat could not tell an elimination game from a dead rubber,
/// could not tell it had already won, and could not apply the one rule this
/// section spends five lines teaching, which is keyed on whether this is game
/// 1. Since #484 an even `best_of` can also end level, and a level match is a
/// draw worth 1 point against 3, so a seat behind with one game left is
/// playing for a draw and was never told (issue #609).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchFormat {
    /// One game, standing alone — no match around it and no tournament.
    SingleGame,
    /// One game of a tournament match, with the seat's own side of the score.
    BestOf {
        /// Games the match is decided over; it stops after this many played.
        best_of: usize,
        /// 1-based number of the game about to be played.
        game: usize,
        /// Games this seat has won so far in the match.
        your_wins: usize,
        /// Games the opponent has won so far in the match.
        their_wins: usize,
    },
}

impl MatchFormat {
    /// Game 1 of a best-of-`n` match, before anything has been played.
    #[must_use]
    pub fn best_of(n: usize) -> Self {
        Self::BestOf { best_of: n, game: 1, your_wins: 0, their_wins: 0 }
    }

    /// The "## This match" section of the system prompt: the format, where
    /// this game sits in it, what winning or losing it settles, and what
    /// being on the play costs.
    #[must_use]
    pub fn match_section(self) -> String {
        let mut s = String::from("\n\n## This match\n\n");
        match self {
            Self::SingleGame => s.push_str(
                "This is a single game, not a match: there is no game 2, and nothing \
carries over from it. The starting player is randomised (a fair coin flip); the \
mulligan prompt tells you which you are.\n",
            ),
            Self::BestOf { best_of: 1, .. } => s.push_str(
                "Matches are best-of-one: this game decides the match, and there is no \
game 2. The winner of a coin flip may choose to play or draw, and plays first if it \
does not choose; the mulligan prompt tells you which you are.\n",
            ),
            Self::BestOf { best_of: n, game, your_wins, their_wins } => {
                // Two bounds end a match, and only one of them is a win
                // threshold: `wins_needed` is `n / 2 + 1`, and the match also
                // stops after `n` games however they went (#484). At the cap
                // `MatchResult::winner` is simply whoever has more game wins,
                // so a best-of-4 can be won 2-1 without anyone reaching 3 —
                // which is why the stake below is computed from both bounds
                // rather than from the threshold alone.
                let needed = n / 2 + 1;
                // Games after this one, if the match runs to the cap.
                let left = n.saturating_sub(game);
                let win_takes_it = your_wins + 1 >= needed
                    || (left == 0 && your_wins + 1 > their_wins);
                let loss_loses_it = their_wins + 1 >= needed
                    || (left == 0 && their_wins + 1 > your_wins);
                // The last game, where the result only levels the score.
                let win_levels = left == 0 && your_wins + 1 == their_wins;
                let loss_levels = left == 0 && their_wins + 1 == your_wins;
                // A drawn game wins nothing but still uses up one of the n,
                // so a match can reach a game that cannot change it: 2-0
                // into game 4 of a best-of-4 is the leader's whatever game 4
                // does. Even winning this game and every one after it leaves
                // the trailer behind (#649).
                let already_won = your_wins > their_wins + left + 1;
                let already_lost = their_wins > your_wins + left + 1;

                // The last game is game n, not game 3: a seat told its match
                // ends at game 3 when it does not plays the end of a longer
                // match wrong, and one told there is a game 3 in a
                // best-of-two is told about a game that will not happen
                // (#484).
                writeln!(s, "Matches are best-of-{n}: the match ends as soon as one of you \
has won {needed} games, or after {n} games have been played, whichever comes first. \
Whoever has won more games then wins the match.").unwrap();
                writeln!(s, "\n**This is game {game} of at most {n}.** The score so far is \
**you {your_wins}, your opponent {their_wins}**.").unwrap();

                if already_won || already_lost {
                    let who = if already_won { "you have" } else { "your opponent has" };
                    writeln!(s, "**The match is already decided:** {who} won it whatever \
happens in this game{}. The game still counts toward the standings, where game wins \
break ties between seats on the same match points.",
                        if left == 0 { "" } else { " and the rest" }).unwrap();
                } else if win_takes_it && loss_loses_it {
                    s.push_str("This game decides the match either way: win it and the \
match is yours, lose it and it is theirs.\n");
                } else if win_takes_it {
                    s.push_str("Winning this game wins you the match.");
                    if loss_levels {
                        s.push_str(" Losing it leaves the score level, which is a draw.");
                    }
                    s.push('\n');
                } else if loss_loses_it {
                    s.push_str("Losing this game loses you the match.");
                    if win_levels {
                        s.push_str(" Winning it levels the score, which is a draw.");
                    }
                    s.push('\n');
                } else {
                    let more = if left == 1 { "1 more game follows it" }
                        else { &format!("{left} more games follow it") };
                    writeln!(s, "Neither of you can win the match with this game; {more}.").unwrap();
                }
                if !(already_won || already_lost) {
                    s.push_str("A match that ends with the score level is a **draw** — 1 \
tournament point each, against 3 for a match win — so a game that cannot win you the \
match is still worth not losing.\n");
                }

                // Who chose is all the seat is told: a drafting seat
                // chooses to play or draw itself, between games, and a
                // person may choose to draw, so "the loser always plays"
                // stopped being true.
                if game == 1 {
                    s.push_str("\nThe winner of a coin flip may choose who goes first in game \
1, and plays first if it does not choose; before each later game the loser of the \
previous one chooses in the same way. The mulligan prompt tells you which side you are on.\n");
                } else {
                    writeln!(s, "\nThe loser of game {} may choose who goes first in this game \
(after a drawn game, whoever chose before), and plays first if it does not choose. The \
mulligan prompt tells you which side you are on.", game - 1).unwrap();
                }
            }
        }
        s.push_str(
            "\nThe player on the play skips their first draw step; the player on the draw \
gets a normal first turn. That means after turn 1 the on-draw player has seen one \
more card.\n",
        );
        s
    }
}

/// Shared game rules and strategy — used by all backends.
/// The notes line's claim about the conversation, true with the history
/// window at 0 (the default).
const NO_HISTORY_KEPT: &str = "No earlier prompt is kept, so this is how a plan carries over";

/// [`GAME_RULES`] as this run's seats are sent them. The notes line says no
/// earlier prompt is kept; under `MTG_LLM_HISTORY=N` the seat resumes its
/// last exchanges, and the one knob whose system prompt did not follow it
/// told the model something false about its own conversation (#858). The
/// default run is sent the same bytes as before.
fn game_rules(history: usize) -> std::borrow::Cow<'static, str> {
    match history {
        0 => std::borrow::Cow::Borrowed(GAME_RULES),
        n => std::borrow::Cow::Owned(GAME_RULES.replacen(NO_HISTORY_KEPT, &format!(
            "Up to {n} of your earlier decisions are also kept in this conversation and older ones are not, so these notes are how a plan carries over"), 1)),
    }
}

/// A system prompt built for one history window, said for another: a
/// backend's window can be set after its prompt is built (`with_history`),
/// and the prompt is the backend's own, sent with every call (#858's
/// review).
fn rewindow(system_prompt: &str, from: usize, to: usize) -> String {
    system_prompt.replacen(game_rules(from).as_ref(), game_rules(to).as_ref(), 1)
}

const GAME_RULES: &str = r#"## Prompt format

Every prompt restates the whole position, in this order:
- `Turn N - <step> (your turn|opp's turn)`. Steps: Untap, Upkeep, Draw, Main Phase 1, Begin Combat, Declare Attackers, Declare Blockers, First-Strike Combat Damage, Combat Damage, End Combat, Main Phase 2, End Step, Cleanup.
- `Your notes from your last decision:` your own thoughts from the last prompt. No earlier prompt is kept, so this is how a plan carries over; trust the board over them.
- `Recent events:` what happened since your last prompt, at most the most recent 80 entries (a marker such as `… 227 earlier entries omitted, through turn 94 …` says when older ones were dropped).
- `You:`/`Opp:` life (hp), hand, library, graveyard and exile counts; `This turn:` spells each side cast (werewolves transform on these) and `a creature died` (morbid).
- `Mana pool:` yours and `Opp mana pool:` (when not empty); `Your board:` and `Opp board:`; `Stack:` (top first, with controller and targets); `Hand:` (cost, and base P/T for creatures); graveyards; exile; `Flashback available`.
- `Opp's cards in view:` rules text for the opponent's cards on the battlefield, on the stack, in exile or revealed, and for a graveyard card that works from there (flashback; cast or returned from a graveyard) or that the question or a stack object points at. You learn them as they come into view, never before. `(×N)` ends a line repeated N times; `3x` is three of a card.
- A context line (`[MAIN PHASE 1]`, `[RESPOND TO opp's <spell>]`, `[AFTER BLOCKERS DECLARED]`, ...) and the question.

Boards group lands (`2x Forest (1 tapped)`); any other permanent is `Name (#id) P/T keywords [flags]`. The `#id` tells apart permanents of one name. P/T is current, bonuses included, and a creature has exactly the keywords printed after it. Status flags after a permanent, in one bracket: `T` tapped; `S` summoning sick (no attacking or {T} abilities until its controller's next turn; it can block); `attacking you`/`attacking Opp`/`attacking <planeswalker> (#id)`; `blocking <attacker> (#id)`; `blocked by <blocker> (#id)`; `blocked (no blockers left)` (deals no combat damage unless it has trample); `Ndmg` damage marked; `damaged this turn by <permanent> (#id)` (what on the battlefield has dealt it damage this turn, marked or since removed); `regen shield`/`N regen shields`; `token`; `copy`; `owned by opponent`/`owned by you` (it returns to its owner when the control effect ends, and goes to its owner's graveyard); `+1+1xN`, `-1-1xN`, `LOYxN` or another counter by kind (`Slimex2`); `names: <card>`; `enchanting you`/`enchanting opponent` (a Curse's player); `legendary` (CR 704.5j: of two with one name under your control, you keep one). Elsewhere an object carries its id and whose it is: `(your)`, `(opponent's)`, `(in your graveyard)`, `(in opponent's exile)`.

## Answering

- An action list has one numbered row per option; answer with the index. `Cast`/`Activate` rows show what they tap for you (`Cast Doom Blade (tap 2x Swamp)`), chosen so as not to strand another spell or ability you could pay for. `Tap <source>` rows only float mana, and a mana pool empties between steps. A spell with targets asks `<card name>: select a target:` next, unless the row names its one legal target (`Cast Brimstone Volley (tap 3x Mountain) targeting you`). Copies of one permanent offering one ability share a row, one index per copy: `5-7: Activate Ludevic's Test Subject ({1}{U}: Put a hatchling counter. At 5, transform.) (tap 2x Island) — one per copy: 5=#43, 6=#45, 7=#46`. `Concede` asks you to confirm.
- `Choose attackers:` asks for the indices that attack (forced attackers are added); `Declare blocks` for `{"blocker", "attacker"}` index pairs, at most one per blocker.
- Every other prompt — a target, a cost, X, an order, damage, a mulligan — says what it asks.
- Creatures, sorceries, enchantments, artifacts and lands only in your main phase with an empty stack; instants and abilities whenever you have priority.

## Playing

- Reason only from what the prompt lists: a keyword not printed after the P/T is not there; a card not in view is unknown.
- Blocked creatures trade by current P/T; first strike deals its damage first; an attacker assigns lethal to one blocker before the next. Chump the highest-power attacker, not the smallest.
- Let combat damage resolve before casting a "creature died this turn" effect.
- Behind on board and life, look for a line that changes the race — an equip, an aura, a trick — rather than passing to topdeck.
- Concede only when no play of yours survives the opponent's next turn: count its damage through your best blocks (a chump stops a creature without trample), and the turns until a library runs out. An opponent may not find its best line.
"#;

/// What every schema's `thoughts` field asks for. The reasoning is private
/// and the plan carries over as the seat's notes, so the field is the
/// decision, not the deliberation: "Concise but complete summary of your
/// internal thoughts" came back at ≈ 300 characters per decision, and
/// output is the largest term a decision has (`reports/llm-cost.md`).
pub const THOUGHTS_DESCRIPTION: &str =
    "One or two short sentences: the deciding reason, and your plan.";

/// Backend trait for LLM API communication.
/// Separates provider-specific API mechanics from shared game logic.
/// A backend that keeps the conversation but sends nothing anywhere. See
/// [`LlmPlayer::for_prompt_tests`] — the prompt is what those tests read
/// back, so it is recorded exactly as a real backend would record it.
#[derive(Default)]
struct InertBackend {
    system_prompt: String,
    turns: usize,
}

impl LlmBackend for InertBackend {
    fn send(&mut self, _message: &str) -> String {
        self.turns += 1;
        String::new()
    }
    fn init(&mut self, deck_info: &str) {
        // Composed exactly as the API backend composes it: the rules and the
        // response-format preamble come from the backend, so a stand-in that
        // only stored what it was handed would drop most of the prompt the
        // tests are there to inspect.
        self.system_prompt = format!("{THOUGHTS_IN_JSON_FORMAT}{}{deck_info}", game_rules(history_exchanges()));
        self.turns = 0;
    }
    fn conversation_len(&self) -> usize { self.turns }
    fn system_prompt(&self) -> &str { &self.system_prompt }
    fn model_name(&self) -> &str { "inert" }
}

trait LlmBackend {
    /// Send a message and get a response. Manages conversation state internally.
    fn send(&mut self, message: &str) -> String;
    /// Send a message with a custom JSON response schema. Returns the parsed JSON.
    /// Default implementation: calls `send()` and wraps the text in a JSON string.
    fn send_with_schema(&mut self, message: &str, _schema: &serde_json::Value) -> serde_json::Value {
        let text = self.send(message);
        serde_json::Value::String(text)
    }
    /// Initialize with a system prompt (rules + decklists).
    fn init(&mut self, system_prompt: &str);
    /// How many earlier exchanges a decision is sent with; see
    /// [`HISTORY_ENV`], whose value every backend starts from.
    fn set_history_window(&mut self, _exchanges: usize) {}
    /// Whether a `claude -p` seat asks for its answer in the prompt rather
    /// than with `--json-schema` ([`answer_text`]); no-op for the others.
    fn set_text_answers(&mut self, _on: bool) {}
    /// Set thinking level (Gemini only, no-op for others).
    fn set_thinking_level(&mut self, _level: &str) {}
    /// Point a `claude -p` backend at another model, keeping the binary it
    /// runs; `false` from a backend that has to be built anew instead.
    fn retarget_model(&mut self, _model: &str) -> bool { false }
    /// Why the last call produced no answer at all, if it produced none.
    ///
    /// A backend that gives up hands the caller an empty value, which is
    /// indistinguishable from a model that answered with one — so the log
    /// said the seat sent `{}` and the summary counted it beside genuine
    /// bad answers (issue #587). Taken, like `take_thinking`, so it belongs
    /// to exactly one decision.
    fn take_call_failure(&mut self) -> Option<String> { None }
    /// Why the backend has stopped answering for good, once it has: a call
    /// spent its whole retry budget without an answer (#587). Unlike
    /// `take_call_failure` this is not taken — a seat that gave up stays
    /// given up, and its runner forfeits it.
    fn gave_up(&self) -> Option<String> { None }
    /// Get the conversation length (for tests).
    fn conversation_len(&self) -> usize { 0 }
    /// Get the system prompt (for tests).
    fn system_prompt(&self) -> &str;
    /// Get the model identifier.
    fn model_name(&self) -> &str;
    /// Return and clear the thinking text from the last API call, if any.
    fn take_thinking(&mut self) -> Option<String> { None }
    /// The id of the conversation this backend is currently in, for a
    /// backend that has one. `None` for a backend whose history it holds
    /// itself, which is every backend but the `claude -p` CLI seat.
    fn session_id(&self) -> Option<&str> { None }
    /// The seat this backend answers for, so the records it writes itself —
    /// `API_RETRY`, `API_ERROR` and the rest — say whose call it was. A
    /// tournament match runs both seats' backends on one thread, so the
    /// thread name does not (#659).
    fn set_seat(&mut self, _seat: &str) {}
}

/// A backend record's label, naming the seat the way `LlmPlayer`'s own
/// records do (`MALFORMED [Seat3]`), when the backend knows it (#659).
pub(crate) fn api_label(kind: &str, seat: &str) -> String {
    if seat.is_empty() { kind.to_string() } else { format!("{kind} [{seat}]") }
}

/// The same seat, ahead of a backend's line on stderr.
pub(crate) fn seat_tag(seat: &str) -> String {
    if seat.is_empty() { String::new() } else { format!("[{seat}] ") }
}

/// The response intro for a backend whose reasoning the harness can only see
/// if it comes back INSIDE the JSON: Gemini, and the `claude -p` CLI seat,
/// whose result object carries no thinking block the harness can read
/// (issue #213).
const THOUGHTS_IN_JSON_FORMAT: &str = r#"You are playing Magic: The Gathering, a one-on-one Limited (draft) match: 40-card
decks, 20 life, reduce the opponent to 0. Each decision is one prompt with the
whole position and the legal options; answer with the JSON its schema asks for.
`thoughts` is private: the deciding reason and your plan, which come back to you
as notes in your next prompt. Ground every claim in the prompt text.
"#;


/// Anthropic Claude backend using the Messages API with prompt caching.
struct AnthropicBackend {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: String,
    conversation: Vec<serde_json::Value>,
    last_thinking: Option<String>,
    /// Set when the retries run out, so the caller can tell "no answer"
    /// from an answer it could not use (#587).
    last_call_failure: Option<String>,
    /// Set, and never cleared, once the seat has stopped answering; see
    /// `LlmBackend::gave_up` (#719).
    gave_up: Option<String>,
    /// `https://api.anthropic.com` unless `ANTHROPIC_BASE_URL` says
    /// otherwise — which is how the failure paths are tested without a
    /// metered call.
    base_url: String,
    /// The seat this backend answers for; see `LlmBackend::set_seat`.
    seat: String,
    /// How hard the seat thinks per decision; see [`thinking_level`].
    thinking: ThinkingLevel,
    /// How many earlier exchanges a request carries; see [`HISTORY_ENV`].
    window: usize,
}

impl AnthropicBackend {
    fn new(model: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable must be set");
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            system_prompt: format!("{THOUGHTS_IN_JSON_FORMAT}{}", game_rules(history_exchanges())),
            conversation: Vec::new(),
            last_thinking: None,
            last_call_failure: None,
            gave_up: None,
            base_url: anthropic_base_url(),
            seat: String::new(),
            thinking: thinking_level(),
            window: history_exchanges(),
        }
    }

    /// Keep only the last `window` exchanges, so the next request carries
    /// them and nothing older (see [`HISTORY_ENV`]).
    fn trim_history(&mut self) {
        let keep = self.window * 2;
        if self.conversation.len() > keep {
            let drop = self.conversation.len() - keep;
            self.conversation.drain(..drop);
        }
    }

    /// Build the system prompt and messages with cache control breakpoints.
    fn prepare_request(&self, messages: &[serde_json::Value]) -> (serde_json::Value, Vec<serde_json::Value>) {
        let system = serde_json::json!([{
            "type": "text",
            "text": self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        if msgs.len() >= 2 {
            let idx = msgs.len() - 2;
            if let Some(content) = msgs[idx].get("content").and_then(|c| c.as_str()).map(std::string::ToString::to_string) {
                msgs[idx] = serde_json::json!({
                    "role": msgs[idx]["role"],
                    "content": [{
                        "type": "text",
                        "text": content,
                        "cache_control": {"type": "ephemeral"}
                    }]
                });
            }
        }

        (system, msgs)
    }

    /// Send a request to the Anthropic API and return the text content.
    /// Scans content blocks for thinking (logged) and text (returned).
    ///
    /// Retries within the game seat's budget (#587, #719); a call that gets
    /// no answer returns `"0"` with `last_call_failure` saying why, and one
    /// that spends the budget or meets a refused key marks the seat given
    /// up.
    fn call_api(&mut self, body: &serde_json::Value) -> String {
        if let Some(why) = &self.gave_up {
            self.last_call_failure = Some(why.clone());
            return "0".to_string();
        }
        let budget = retry_budget(claude_code::RETRY_BUDGET_ENV);
        let url = format!("{}/v1/messages", self.base_url);
        let seat = self.seat.clone();
        let (client, api_key, model) = (&self.client, &self.api_key, &self.model);
        let mut thinking = None;
        let outcome = call_within_budget(budget, |attempt| {
            let started = std::time::Instant::now();
            let response = client
                .post(&url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .timeout(std::time::Duration::from_secs(120))
                .json(body)
                .send();
            let elapsed_ms = started.elapsed().as_millis();
            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_anthropic_llm_usage(model, &json);
                    thinking = None;
                    let mut text_content = String::from("0");
                    if let Some(content) = json["content"].as_array() {
                        for block in content {
                            match block["type"].as_str() {
                                Some("thinking") => {
                                    if let Some(t) = block["thinking"].as_str() {
                                        thinking = Some(t.to_string());
                                    }
                                }
                                Some("text") => {
                                    if let Some(text) = block["text"].as_str() {
                                        text_content = text.trim().to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    CallAttempt::Answer(text_content)
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    let snippet: String = text.chars().take(200).collect();
                    classify_http_status(code, format!(
                        "Anthropic HTTP {code} (attempt {attempt}, {elapsed_ms}ms): {snippet}"))
                }
                Err(e) => CallAttempt::Transient(format!(
                    "Anthropic request failed (attempt {attempt}, {elapsed_ms}ms): {}",
                    format_reqwest_error(&e))),
            }
        }, |_, msg, retried| {
            crate::stderr_line!("{}{msg}", seat_tag(&seat));
            if retried {
                crate::game_log::write(file!(), line!(), &api_label("API_RETRY", &seat), msg);
            } else {
                crate::game_log::write_at(crate::game_log::LogLevel::Error, file!(), line!(), &api_label("API_ERROR", &seat), msg);
            }
        });
        self.last_thinking = thinking;
        self.settle_call(outcome)
    }

    /// The seat's state after a call: an answer is returned; anything else
    /// is recorded as no answer, and a give-up stays given up.
    fn settle_call(&mut self, outcome: CallOutcome<String>) -> String {
        match outcome {
            CallOutcome::Answer(text) => text,
            CallOutcome::Failed(why) => {
                self.last_call_failure = Some(why);
                "0".to_string()
            }
            CallOutcome::GaveUp(why) => {
                let msg = format!("Anthropic game seat {why}");
                crate::game_log::write_at(crate::game_log::LogLevel::Error, file!(), line!(), &api_label("API_ERROR", &self.seat), &msg);
                crate::stderr_line!("{}{msg}", seat_tag(&self.seat));
                self.last_call_failure = Some(msg.clone());
                self.gave_up = Some(msg);
                "0".to_string()
            }
        }
    }

    fn call_with_messages(&mut self, messages: &[serde_json::Value]) -> String {
        let (system, msgs) = self.prepare_request(messages);
        let mut body = serde_json::json!({
            "model": self.model,
            "system": system,
            "messages": msgs,
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "action": {"type": "integer"}
                        },
                        "required": ["action"],
                        "additionalProperties": false
                    }
                }
            }
        });
        apply_thinking(&mut body, &self.model, self.thinking);
        self.call_api(&body)
    }

    fn call_with_messages_structured(&mut self, messages: &[serde_json::Value], schema: &serde_json::Value) -> serde_json::Value {
        let (system, msgs) = self.prepare_request(messages);
        // `thoughts` is kept: it is where the seat's notes come from (two
        // sentences, carried into its next prompt), and the reasoning a
        // seat with thinking off does at all. The thinking channel cannot
        // be the notes' source — at `off` there is none, and the newest
        // models return their thinking empty unless asked otherwise — so every
        // backend asks for it in the JSON, as the `claude -p` and Gemini
        // seats always have (reports/llm-cost.md, phase 3). And the schema
        // goes without its per-call constraints, since a changed
        // `output_config.format` invalidates the prompt cache
        // (`cache_stable_schema`).
        let sanitized = sanitize_schema_for_anthropic(&cache_stable_schema(schema), true);
        let mut body = serde_json::json!({
            "model": self.model,
            "system": system,
            "messages": msgs,
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": sanitized
                }
            }
        });
        apply_thinking(&mut body, &self.model, self.thinking);
        let text = self.call_api(&body);
        let mut parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}));
        // The thoughts are the seat's notes and its THOUGHT line, taken out
        // of the answer as the other backends take them out.
        if let Some(thoughts) = parsed.as_object_mut().and_then(|o| o.remove("thoughts")) {
            if let Some(t) = thoughts.as_str().filter(|t| !t.trim().is_empty()) {
                self.last_thinking = Some(t.to_string());
            }
        }
        parsed
    }
}

impl LlmBackend for AnthropicBackend {
    fn set_seat(&mut self, seat: &str) {
        seat.clone_into(&mut self.seat);
    }

    fn take_call_failure(&mut self) -> Option<String> {
        self.last_call_failure.take()
    }

    fn gave_up(&self) -> Option<String> {
        self.gave_up.clone()
    }

    fn send(&mut self, message: &str) -> String {
        self.conversation.push(serde_json::json!({"role": "user", "content": message}));
        let result = self.call_with_messages(&self.conversation.clone());
        self.conversation.push(serde_json::json!({"role": "assistant", "content": &result}));
        self.trim_history();
        // Extract action number from JSON response (e.g. {"action":1} → "1").
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
            if let Some(action) = parsed["action"].as_u64() {
                return action.to_string();
            }
        }
        result
    }

    fn send_with_schema(&mut self, message: &str, schema: &serde_json::Value) -> serde_json::Value {
        self.conversation.push(serde_json::json!({"role": "user", "content": message}));
        let result = self.call_with_messages_structured(&self.conversation.clone(), schema);
        let result_str = serde_json::to_string(&result).unwrap_or_default();
        self.conversation.push(serde_json::json!({"role": "assistant", "content": result_str}));
        self.trim_history();
        result
    }

    fn init(&mut self, deck_info: &str) {
        self.system_prompt = format!("{THOUGHTS_IN_JSON_FORMAT}{}{deck_info}", game_rules(self.window));
        self.conversation.clear();
    }

    fn set_history_window(&mut self, exchanges: usize) {
        self.system_prompt = rewindow(&self.system_prompt, self.window, exchanges);
        self.window = exchanges;
        self.trim_history();
    }

    fn conversation_len(&self) -> usize {
        self.conversation.len()
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn take_thinking(&mut self) -> Option<String> {
        self.last_thinking.take()
    }
}

/// Gemini backend using the Interactions API with server-managed conversation state.
struct GeminiBackend {
    client: Client,
    api_key: String,
    model: String,
    thinking_level: Option<String>,
    system_prompt: String,
    interaction_id: Option<String>,
    last_thinking: Option<String>,
    /// See `AnthropicBackend::last_call_failure` (#587).
    last_call_failure: Option<String>,
    /// See `AnthropicBackend::gave_up` (#719).
    gave_up: Option<String>,
    /// `https://generativelanguage.googleapis.com` unless `GEMINI_BASE_URL`
    /// says otherwise.
    base_url: String,
    /// The seat this backend answers for; see `LlmBackend::set_seat`.
    seat: String,
    /// How many decisions one server-held interaction chain carries before
    /// the seat starts a fresh one; see [`HISTORY_ENV`].
    window: usize,
    /// Decisions answered in the current chain.
    chain_len: usize,
}

impl GeminiBackend {
    fn new(model: &str) -> Self {
        let api_key = env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY environment variable must be set");
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            thinking_level: None,
            system_prompt: format!("{THOUGHTS_IN_JSON_FORMAT}{}", game_rules(history_exchanges())),
            interaction_id: None,
            last_thinking: None,
            last_call_failure: None,
            gave_up: None,
            base_url: gemini_base_url(),
            seat: String::new(),
            window: history_exchanges(),
            chain_len: 0,
        }
    }

    /// Core interactions API call. Sends a message with a custom JSON schema
    /// and returns the parsed JSON response. Handles retries, rate limits, etc.
    fn call_interactions_structured(&mut self, user_message: &str, schema: &serde_json::Value) -> serde_json::Value {
        // A chain that has carried its window of decisions is not continued:
        // the server would re-read every one of them on every call.
        if self.chain_len > self.window {
            self.interaction_id = None;
            self.chain_len = 0;
        }
        let mut body = serde_json::json!({
            "model": &self.model,
            "input": user_message,
            "response_mime_type": "application/json",
            "response_format": sanitize_schema_for_gemini(schema),
        });

        if let Some(ref level) = self.thinking_level {
            body["generation_config"] = serde_json::json!({"thinking_level": level});
        }

        // Only chain if we have a non-empty previous interaction ID.
        if let Some(prev_id) = self.interaction_id.as_ref().filter(|s| !s.is_empty()) {
            body["previous_interaction_id"] = serde_json::json!(&prev_id);
        } else {
            body["system_instruction"] = serde_json::json!(&self.system_prompt);
        }

        let url = format!("{}/v1beta/interactions?key={}", self.base_url, self.api_key);

        if let Some(why) = &self.gave_up {
            self.last_call_failure = Some(why.clone());
            return serde_json::json!({});
        }
        let budget = retry_budget(claude_code::RETRY_BUDGET_ENV);
        let seat = self.seat.clone();
        let system_prompt = self.system_prompt.clone();
        let (client, model) = (&self.client, &self.model);
        let mut interaction_id = self.interaction_id.clone();
        let mut thinking = None;
        let mut fresh_retry = false;
        let outcome = call_within_budget(budget, |attempt| {
            let started = std::time::Instant::now();
            let response = client
                .post(&url)
                .header("content-type", "application/json")
                .timeout(std::time::Duration::from_secs(120))
                .json(&body)
                .send();
            let elapsed_ms = started.elapsed().as_millis();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_gemini_llm_usage(model, &json["usage"]);

                    interaction_id = json["id"].as_str()
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string);

                    let mut output_text = String::new();
                    if let Some(outputs) = json["outputs"].as_array() {
                        for out in outputs {
                            if out["type"].as_str() == Some("text") {
                                if let Some(t) = out["text"].as_str() {
                                    output_text = t.trim().to_string();
                                }
                            }
                        }
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output_text) {
                        thinking = parsed["thoughts"].as_str().map(std::string::ToString::to_string);
                        return CallAttempt::Answer(parsed);
                    }
                    // The API was asked for JSON and did not return it: no
                    // answer was given, and saying so keeps it from being
                    // logged as the model's (#719).
                    CallAttempt::Refused(format!("Gemini returned non-JSON response: {:?}",
                        output_text.chars().take(100).collect::<String>()))
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    let snippet: String = text.chars().take(200).collect();
                    // If the interaction ID is invalid, fall back to a fresh conversation.
                    if code == 400 && text.contains("previous_interaction_id") && !fresh_retry {
                        let msg = "Invalid interaction ID, falling back to fresh conversation";
                        crate::stderr_line!("{}WARN: {msg}", seat_tag(&seat));
                        crate::game_log::write(file!(), line!(), &api_label("API_WARN", &seat), msg);
                        body.as_object_mut().unwrap().remove("previous_interaction_id");
                        body["system_instruction"] = serde_json::json!(&system_prompt);
                        interaction_id = None;
                        fresh_retry = true;
                        return CallAttempt::Transient(format!("Gemini HTTP {code} (attempt {attempt}): stale interaction id"));
                    }
                    // Fatal config errors — abort loudly so we don't silently produce garbage.
                    if code == 400 && (text.contains("thinking level") || text.contains("not a supported")) {
                        let msg = format!("Gemini config error: {}", text.chars().take(300).collect::<String>());
                        crate::stderr_line!("{}FATAL: {msg}", seat_tag(&seat));
                        crate::game_log::write(file!(), line!(), &api_label("API_FATAL", &seat), &msg);
                        std::process::exit(1);
                    }
                    classify_http_status(code, format!(
                        "Gemini HTTP {code} (attempt {attempt}, {elapsed_ms}ms): {snippet}"))
                }
                Err(e) => CallAttempt::Transient(format!(
                    "Gemini request failed (attempt {attempt}, {elapsed_ms}ms): {}",
                    format_reqwest_error(&e))),
            }
        }, |_, msg, retried| {
            crate::stderr_line!("{}{msg}", seat_tag(&seat));
            if retried {
                crate::game_log::write(file!(), line!(), &api_label("API_RETRY", &seat), msg);
            } else {
                crate::game_log::write_at(crate::game_log::LogLevel::Error, file!(), line!(), &api_label("API_ERROR", &seat), msg);
            }
        });
        self.interaction_id = interaction_id;
        match outcome {
            CallOutcome::Answer(parsed) => {
                self.last_thinking = thinking;
                self.chain_len += 1;
                parsed
            }
            CallOutcome::Failed(why) => {
                self.last_call_failure = Some(why);
                serde_json::json!({})
            }
            CallOutcome::GaveUp(why) => {
                let msg = format!("Gemini game seat {why}");
                crate::stderr_line!("{}{msg}", seat_tag(&self.seat));
                crate::game_log::write_at(crate::game_log::LogLevel::Error, file!(), line!(), &api_label("API_ERROR", &self.seat), &msg);
                self.last_call_failure = Some(msg.clone());
                self.gave_up = Some(msg);
                serde_json::json!({})
            }
        }
    }

    /// Convenience wrapper: sends with the default action schema, returns just
    /// the action number as a string (backward-compatible with existing callers).
    fn call_interactions(&mut self, user_message: &str) -> String {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "action": {"type": "integer", "minimum": 0}
            },
            "required": ["thoughts", "action"]
        });
        let parsed = self.call_interactions_structured(user_message, &schema);
        parsed["action"].as_u64().map_or_else(|| "0".to_string(), |n| n.to_string())
    }
}

impl LlmBackend for GeminiBackend {
    fn set_seat(&mut self, seat: &str) {
        seat.clone_into(&mut self.seat);
    }

    fn take_call_failure(&mut self) -> Option<String> {
        self.last_call_failure.take()
    }

    fn gave_up(&self) -> Option<String> {
        self.gave_up.clone()
    }

    fn send(&mut self, message: &str) -> String {
        self.call_interactions(message)
    }

    fn send_with_schema(&mut self, message: &str, schema: &serde_json::Value) -> serde_json::Value {
        self.call_interactions_structured(message, schema)
    }

    fn init(&mut self, deck_info: &str) {
        self.system_prompt = format!("{THOUGHTS_IN_JSON_FORMAT}{}{deck_info}", game_rules(self.window));
        self.interaction_id = None;
        self.chain_len = 0;
    }

    fn set_history_window(&mut self, exchanges: usize) {
        self.system_prompt = rewindow(&self.system_prompt, self.window, exchanges);
        self.window = exchanges;
    }

    fn set_thinking_level(&mut self, level: &str) {
        self.thinking_level = Some(level.to_string());
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn take_thinking(&mut self) -> Option<String> {
        self.last_thinking.take()
    }
}

/// Which backend family a player was built with, so `with_model` rebuilds
/// the same kind. `Anthropic` and `Gemini` are metered API seats;
/// `ClaudeCode` runs the same prompt protocol through `claude -p` and is
/// billed to whatever that CLI is logged into (a subscription, typically).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    Anthropic,
    Gemini,
    ClaudeCode,
}

/// What a row of the action list stands for: an action to submit, a spell
/// to cast through the target prompts, or an ability to activate through
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayEntry {
    /// Index into `LegalActions::actions`.
    Direct(usize),
    /// Index into `LegalActions::castable_spells`.
    Cast(usize),
    /// Index into `LegalActions::activatable_abilities`.
    Ability(usize),
}

/// One line of the action list an LLM seat is offered. Every line takes
/// one display index per option it holds, in order.
#[derive(Debug)]
pub(crate) enum ActionRow {
    /// A single option, `i: label`.
    One(String),
    /// Copies of one permanent that offer the same ability with the same
    /// tap plan, one index each: `i-j: label — one per copy: i=#a, …, j=#b`.
    /// The copies are told apart by their `#id`, which the board section
    /// carries beside each copy's counters and status.
    Copies { label: String, ids: Vec<ObjectId> },
}

impl ActionRow {
    /// The row's text, whichever shape it is.
    pub(crate) fn label(&self) -> &str {
        match self {
            ActionRow::One(label) | ActionRow::Copies { label, .. } => label,
        }
    }
}

pub struct LlmPlayer {
    name: String,
    /// Index into the game log — tracks which log entries have been sent.
    last_log_index: usize,
    /// Every card name (both faces) in this seat's own decklist, which the
    /// system prompt describes in full. Any other name that comes into
    /// view gets its rules text in the decision prompt instead.
    own_card_names: std::collections::HashSet<String>,
    /// The reference entry for every card name the registry knows, both
    /// faces, so a card that comes into view can be described without
    /// the registry in hand.
    card_texts: HashMap<String, String>,
    /// Provider-specific API backend.
    backend: Box<dyn LlmBackend>,
    provider: Provider,
    /// Optional guide text injected into the game-play system prompt.
    guide: Option<String>,
    /// The conversation id already written down, so a `SESSION` record is
    /// one per conversation rather than one per call.
    session_logged: Option<String>,
    /// Why this decision's backend call produced no answer, when it
    /// produced none. Refreshed on every structured request (#587).
    last_call_failure: Option<String>,
    /// The game-so-far recap a resumed seat is owed, delivered in its next
    /// decision prompt. It used to be a permanent entry at the head of the
    /// conversation; the conversation no longer carries earlier prompts
    /// (see [`HISTORY_ENV`]), so the recap rides along with the first
    /// prompt after the resume instead, where a human picking up a saved
    /// game would read the log once.
    pending_recap: Option<String>,
    /// The tail of the seat's own reasoning from its last decision, sent
    /// back to it at the top of the next prompt. With no earlier exchanges
    /// in the conversation this is how a plan survives from one decision to
    /// the next ("hold Bolt for the flier", "attack next turn when the
    /// Tusker loses summoning sickness"): a few hundred characters of the
    /// seat's own words, not the thousands of tokens of prompts they came
    /// from.
    notes: Option<String>,
    /// Where the runner keeps a copy of this seat's [`SeatMemory`] for its
    /// save file, when it asked for one (`mirror_memory_to`): a save that
    /// held the game but not the plan lost the plan on every `--resume`
    /// (#836), and one that held the plan but not the rest could only
    /// resume the seat with a recap of the whole game.
    memory_mirror: Option<std::sync::Arc<std::sync::Mutex<Option<SeatMemory>>>>,
    /// `Some` while the seat is passing every plain priority offer unasked
    /// (see [`PassUntil`]); cleared, with a reason, by the first stop.
    pass_until: Option<PassUntil>,
    /// How many offers the last pass-until passed and why it stopped, said
    /// once in the next prompt so the seat knows why it is being asked.
    pass_until_stopped: Option<(u32, String)>,
    /// Whether the priority menu carries [`PASS_UNTIL_ROW`] at all
    /// ([`PASS_UNTIL_ENV`]). The knob also turns off the other unasked
    /// passes and the main-phase plan, so `off` is the seat asked
    /// everything, for an A/B measurement.
    offer_pass_until: bool,
    /// The rest of a main phase the seat laid out ([`Plan`]).
    plan: Option<Plan>,
    /// Why the last plan stopped short, said once in the next prompt.
    plan_stopped: Option<String>,
    /// The plan's last card was just taken: pass until something happens
    /// from this offer on (engaged where the view is to hand).
    plan_done_pending: bool,
    /// The attack the seat named in its first main phase ([`AttackPlan`]).
    attack_plan: Option<AttackPlan>,
}

/// The model a metered `claude` seat runs when its spec names none, in the
/// games and in the draft. The cheapest current model: replayed over the
/// kept positions of real games, it chose the reference's action more often
/// than the previous default (87.5% of settled positions against 79.8%,
/// at about a twelfth of the price a decision), and refused none of them
/// (reports/llm-cost.md, phase 3). Name a model to run another.
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-5-5";

impl LlmPlayer {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(AnthropicBackend::new(DEFAULT_ANTHROPIC_MODEL)),
            provider: Provider::Anthropic,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    /// A player whose backend answers nothing, for the tests that only
    /// exercise prompt construction and conversation bookkeeping. Building
    /// a real backend reads an API key out of the environment and panics
    /// without one, which made those tests fail on a clean checkout for a
    /// reason that had nothing to do with what they assert.
    #[must_use]
    pub fn for_prompt_tests(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(InertBackend::default()),
            provider: Provider::Anthropic,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    #[must_use]
    pub fn new_gemini(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(GeminiBackend::new("gemini-2.5-flash")),
            provider: Provider::Gemini,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    /// A seat driven through the Claude Code CLI (`claude -p`) instead of
    /// the Messages API: no API key, billed to the CLI's own login. The
    /// binary comes from `CLAUDE_CODE_BIN` or `claude` on `PATH`; the
    /// model is the CLI's default until `with_model` names one.
    #[must_use]
    pub fn new_claude_code(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(claude_code::ClaudeCodeBackend::new(None)),
            provider: Provider::ClaudeCode,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    /// [`new_claude_code`](Self::new_claude_code) with an explicit binary,
    /// for tests that stand in a fake CLI and for unusual installs.
    #[must_use]
    pub fn new_claude_code_with_binary(name: &str, binary: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(claude_code::ClaudeCodeBackend::with_binary(binary, None)),
            provider: Provider::ClaudeCode,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    #[must_use]
    pub fn with_guide(mut self, guide: String) -> Self {
        self.guide = Some(guide);
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: &str) -> Self {
        // Recreate the backend with the new model name, keeping the
        // provider family: a Claude Code seat stays a Claude Code seat
        // (`claude-code:opus` must never quietly become an API-key seat).
        if self.provider == Provider::ClaudeCode {
            // In place, so a seat built on a stand-in binary
            // (`new_claude_code_with_binary`) keeps running it.
            if !self.backend.retarget_model(model) {
                self.backend = Box::new(claude_code::ClaudeCodeBackend::new(Some(model)));
            }
        } else if model.contains("gemini") {
            self.backend = Box::new(GeminiBackend::new(model));
            self.provider = Provider::Gemini;
        } else {
            self.backend = Box::new(AnthropicBackend::new(model));
            self.provider = Provider::Anthropic;
        }
        self
    }

    /// Send this many earlier exchanges with every decision instead of
    /// the run's [`HISTORY_ENV`] setting.
    #[must_use]
    pub fn with_history(mut self, exchanges: usize) -> Self {
        self.backend.set_history_window(exchanges);
        self
    }

    /// Whether a `claude -p` seat asks for its answer in the prompt,
    /// overriding the run's [`answer_text::ANSWERS_ENV`] setting.
    #[must_use]
    pub fn with_text_answers(mut self, on: bool) -> Self {
        self.backend.set_text_answers(on);
        self
    }

    /// Whether this seat's priority menus carry [`PASS_UNTIL_ROW`],
    /// overriding the run's [`PASS_UNTIL_ENV`] setting.
    #[must_use]
    pub fn with_pass_until(mut self, offered: bool) -> Self {
        self.offer_pass_until = offered;
        self
    }

    #[must_use]
    pub fn with_thinking_level(mut self, level: &str) -> Self {
        // Only affects Gemini — set on the backend if it's a GeminiBackend.
        // We need to recreate since we can't downcast through Box<dyn>.
        // Store it and apply when we have access.
        // For now, we use a workaround: GeminiBackend stores thinking_level.
        // Since with_model already creates the right backend type, we just
        // need to set thinking level on it.
        self.backend.set_thinking_level(level);
        self
    }

    /// Initialize the conversation with your decklist and a card reference.
    /// Call this once before the game starts.
    ///
    /// The card reference is whatever the run has decided is public: the
    /// whole set in a draft, nothing in a fixed-deck game, where the
    /// decklist section already describes every card the seat owns. It must
    /// not be the other deck — a seat that is handed the opponent's
    /// decklist knows on turn 1 what it is playing against, and what it is
    /// not (issue #466). Cards the seat has not been told about are
    /// described as they come into view, in the decision prompt.
    pub fn init_conversation(
        &mut self,
        your_deck: &[(String, u32)],
        card_reference: &str,
        registry: &mtg_engine::cards::CardRegistry,
        match_format: MatchFormat,
    ) {
        // The match structure is a property of the run, not of the rules, so
        // it is composed here where the caller knows it rather than baked
        // into the shared `GAME_RULES` const (issue #210).
        let mut deck_info = if self.offer_pass_until { PASS_UNTIL_RULES.to_string() } else { String::new() };
        deck_info.push_str(&match_format.match_section());
        if let Some(guide) = &self.guide {
            deck_info.push_str("\n\n## Guide\n\n");
            deck_info.push_str(guide);
        }
        deck_info.push_str("\n\n## Your decklist\n\n");
        deck_info.push_str(&Self::format_decklist(your_deck, registry));
        if !card_reference.is_empty() {
            deck_info.push_str("\n\n## Card reference\n\n");
            deck_info.push_str(card_reference);
        }
        self.backend.init(&deck_info);
        self.last_log_index = 0;
        // A new game: whatever the seat was passing towards is over, and
        // so is the plan it had for the last game's board — the draft
        // runner plays a match's games with one seat, and game 2's
        // mulligan prompt carried game 1's last thought (#830).
        self.pass_until = None;
        self.pass_until_stopped = None;
        self.plan = None;
        self.plan_stopped = None;
        // Game 2's turn numbers and object ids repeat game 1's, so a
        // leftover attack plan could be declared on the new board unasked.
        self.attack_plan = None;
        self.plan_done_pending = false;
        self.set_notes(None);
        self.own_card_names = your_deck.iter()
            .flat_map(|(name, _)| card_faces(name, registry))
            .map(|(face_name, _)| face_name)
            .collect();
        // Basic lands are left out: what a Swamp does is not news, and
        // the opponent's basics are in view from turn 1.
        self.card_texts = registry.all_names().iter()
            .flat_map(|name| card_faces(name, registry))
            .filter(|(_, data)| !data.supertypes.contains(&mtg_engine::types::Supertype::Basic))
            .map(|(face_name, _)| {
                let entry = card_reference_entry(&face_name, registry);
                (face_name, entry)
            })
            .collect();
        self.log("SYSTEM", self.backend.system_prompt());
    }

    /// The rules text of every card in view that is not from this seat's
    /// own deck — on the battlefield, on the stack, in a graveyard, in
    /// exile, or revealed — one entry per name, basic lands excepted, so the
    /// seat can read what its opponent's cards do without having been handed
    /// the opponent's decklist (issue #466). Empty when there is nothing to
    /// describe.
    ///
    /// A card in a graveyard and nowhere else is described only where its
    /// text works from there — flashback and its kin, or text that names
    /// the graveyard — and is otherwise its name in the graveyard's list.
    /// The text of every dead creature and spent spell was a third of a
    /// prompt's bytes late in a game, written fresh on every call, for
    /// cards that could no longer do anything (reports/llm-cost.md,
    /// phase 3). A graveyard card the question points at is described
    /// too: a target or option of `action_prompt` (by its `#` number, or
    /// a row that is its name), or a target of something on the stack —
    /// Memory's Journey's choice, Nevermore's names, the opponent's
    /// Ghoulcaller's Chant on a creature card (#864).
    fn format_cards_in_view(&self, view: &GameView, action_prompt: &str) -> String {
        let mut names: Vec<&str> = Vec::new();
        names.extend(view.battlefield.iter().map(|p| p.name.as_str()));
        names.extend(view.stack.iter().map(|s| s.name.as_str()));
        names.extend(view.exile.iter().map(|c| c.name.as_str()));
        names.extend(view.revealed_names.values().map(String::as_str));
        let live = names.len();
        names.extend(view.graveyards.iter().flat_map(|(_, cards)| cards.iter().map(|c| c.name.as_str())));
        let (live, graveyard_only): (Vec<&str>, Vec<&str>) = (names[..live].to_vec(), names[live..].to_vec());
        let targeted: Vec<ObjectId> = view.stack.iter().flat_map(|s| &s.targets)
            .filter_map(|t| match t { mtg_engine::actions::Target::Object(o) => Some(*o), _ => None })
            .collect();
        let rows: std::collections::HashSet<&str> = action_prompt.lines()
            .filter_map(|l| l.trim_start().split_once(": ").filter(|(i, _)| i.parse::<usize>().is_ok()))
            .map(|(_, row)| row.trim())
            .collect();
        let asked: Vec<&str> = view.graveyards.iter().flat_map(|(_, cards)| cards)
            .filter(|c| targeted.contains(&c.object_id)
                || action_prompt.contains(&format!("(#{})", c.object_id.0))
                || rows.contains(c.name.as_str()))
            .map(|c| c.name.as_str())
            .collect();
        let mut names: Vec<&str> = live.clone();
        names.extend(graveyard_only.into_iter().filter(|n| !live.contains(n)
            && (asked.contains(n) || self.card_texts.get(*n).is_some_and(|entry| Self::works_from_graveyard(entry)))));
        names.sort_unstable();
        names.dedup();

        let mut s = String::new();
        for name in names {
            if self.own_card_names.contains(name) { continue; }
            if let Some(entry) = self.card_texts.get(name) {
                if s.is_empty() {
                    s.push_str("Opp's cards in view:\n");
                }
                s.push_str(entry);
            }
        }
        s
    }

    /// Whether a card's text can act while it is in a graveyard: an
    /// ability that works from there (flashback and its kin), or a card
    /// cast or returned "from your graveyard". Text that only puts cards
    /// there — Mulch's "into your graveyard" — does not.
    fn works_from_graveyard(entry: &str) -> bool {
        let text = entry.to_lowercase();
        ["from your graveyard", "from a graveyard", "flashback", "unearth", "disturb", "escape",
         "embalm", "eternalize", "retrace", "aftermath", "jump-start", "scavenge", "dredge", "encore"]
            .iter().any(|k| text.contains(k))
    }

    /// Resume conversation from an existing game state.
    /// Sends the full game log as a catch-up message so the AI has context
    /// about what happened before the reload. `you` is the viewing
    /// player's id, used to rewrite `p0`/`p1` references into
    /// "you"/"opp" form before the recap is sent to the model.
    pub fn resume_from_log(&mut self, game_log: &[String], you: mtg_engine::ids::PlayerId) {
        if game_log.is_empty() {
            return;
        }
        // Build a catch-up message with the full game history, rewriting
        // engine-global player references to be player-relative.
        let mut recap = String::from("Game resumed. Here is the complete game log so far:\n\n");
        for entry in game_log {
            recap.push_str(&Self::rewrite_log_entry(entry, you));
            recap.push('\n');
        }
        recap.push_str("\nThe game continues from this point.");

        // Delivered with the next decision prompt rather than as a turn of
        // its own: a resume used to cost a whole API call to be told
        // "ready", and the conversation no longer keeps earlier turns.
        self.pending_recap = Some(recap.clone());
        self.set_notes(None);
        // Set log index to current length so we don't re-send these entries.
        self.last_log_index = game_log.len();
        // Log the recap body, not just its size. It is the largest message
        // the seat is handed after the system prompt and the only account it
        // gets of the game it lost, so a bare count left the resumed
        // conversation unauditable — the recap had to be reconstructed by
        // hand from the save file to check it at all (issue #208). Logged the
        // way `init_conversation` logs the system prompt; the entry count
        // stays in the label so `grep RESUME` still summarizes at a glance.
        self.log(
            &format!("RESUME ({} log entries)", game_log.len()),
            &recap,
        );
    }

    fn format_decklist(entries: &[(String, u32)], registry: &mtg_engine::cards::CardRegistry) -> String {
        let mut s = String::new();
        let mut seen = std::collections::HashSet::new();
        // One entry per name: `4x Name {cost} Type — Sub P/T`, its rules
        // text under it. The name used to be printed twice (the count line,
        // then the face line), a basic land had its type line and its mana
        // ability spelled out, and keywords carried their reminder text —
        // a third of a drafted deck's entries, re-read on every decision
        // (reports/llm-cost.md, phase 3).
        for (name, count) in entries {
            if !seen.insert(name.clone()) {
                writeln!(s, "{count}x {name}").unwrap();
                continue;
            }
            let faces = card_faces(name, registry);
            let basic = faces.first().is_some_and(|(_, d)| d.supertypes.contains(&mtg_engine::types::Supertype::Basic));
            if faces.is_empty() || basic {
                writeln!(s, "{count}x {name}").unwrap();
                continue;
            }
            // Both faces of a double-faced card: a seat that is going to be
            // asked whether to transform has to be told what it becomes
            // (issue #205). The back face is its own line under the front.
            for (k, (face_name, data)) in faces.iter().enumerate() {
                let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
                // The type line as printed, supertypes first (CR 205.4a):
                // "Legendary" is what arms the legend rule, and the seat
                // never saw the word (issue #333).
                let type_line = mtg_engine::types::type_line(
                    &data.supertypes, &data.card_types, &data.subtypes);
                let pt = match (data.power, data.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                if k == 0 {
                    writeln!(s, "{count}x {face_name}{cost} {type_line}{pt}").unwrap();
                } else {
                    writeln!(s, "  back face: {face_name}{cost} {type_line}{pt}").unwrap();
                }
                let text = without_reminder_text(&data.oracle_text);
                if !text.is_empty() {
                    writeln!(s, "  {}", text.replace('\n', "\n  ")).unwrap();
                }
            }
        }
        s
    }

    // ── Test helpers ──────────────────────────────────────────────

    /// Expose `format_decklist` for testing.
    #[must_use]
    pub fn format_decklist_for_test(entries: &[(String, u32)], registry: &mtg_engine::cards::CardRegistry) -> String {
        Self::format_decklist(entries, registry)
    }

    /// Expose `short_effect_summary` for testing.
    #[must_use]
    pub fn short_effect_summary_for_test(oracle_text: &str) -> String {
        Self::short_effect_summary(oracle_text)
    }

    /// Expose the player-relative log rewriter for testing.
    #[must_use]
    pub fn rewrite_log_entry_for_test(entry: &str, you: mtg_engine::ids::PlayerId) -> String {
        Self::rewrite_log_entry(entry, you)
    }

    /// Expose system prompt for testing.
    #[must_use]
    pub fn system_prompt_for_test(&self) -> &str {
        self.backend.system_prompt()
    }

    /// Expose conversation length for testing.
    #[must_use]
    pub fn conversation_len_for_test(&self) -> usize {
        self.backend.conversation_len()
    }

    /// Drive the backend's plain action call directly, for backend tests.
    pub fn backend_send_for_test(&mut self, message: &str) -> String {
        self.backend.set_seat(&self.name);
        self.backend.send(message)
    }

    /// The recap the next decision prompt will carry, for tests.
    #[must_use]
    pub fn pending_recap_for_test(&self) -> Option<&str> {
        self.pending_recap.as_deref()
    }

    /// Keep a copy of the seat's [`SeatMemory`] in `cell` from now on, for
    /// a save file to carry across `--resume` (#836). The copy is brought
    /// up to date whenever the seat finishes answering a decision, which is
    /// before the runner writes its next save.
    pub fn mirror_memory_to(&mut self, cell: std::sync::Arc<std::sync::Mutex<Option<SeatMemory>>>) {
        self.memory_mirror = Some(cell);
        self.sync_memory_mirror();
    }

    /// What this seat carries to its next decision.
    #[must_use]
    pub fn memory(&self) -> SeatMemory {
        SeatMemory {
            notes: self.notes.clone(),
            log_cursor: self.last_log_index,
            pass_until: self.pass_until.clone(),
            pass_until_stopped: self.pass_until_stopped.clone(),
            plan: self.plan.clone(),
            plan_stopped: self.plan_stopped.clone(),
            attack_plan: self.attack_plan.clone(),
        }
    }

    /// Put back what a save carried, after `init_conversation`: the next
    /// prompt is the one the seat would have been sent had the game never
    /// stopped — the same notes, the events since the same point, the same
    /// pass-until — and no recap.
    pub fn restore_memory(&mut self, memory: SeatMemory) {
        self.notes = memory.notes;
        self.last_log_index = memory.log_cursor;
        self.pass_until = memory.pass_until;
        self.pass_until_stopped = memory.pass_until_stopped;
        self.plan = memory.plan;
        self.plan_stopped = memory.plan_stopped;
        self.attack_plan = memory.attack_plan;
        self.pending_recap = None;
        self.sync_memory_mirror();
    }

    /// Put back the notes a save carried, after `resume_from_log`: the
    /// first prompt after a resume carries the plan made before the save.
    /// For a save written before [`SeatMemory`] was; a newer one is given
    /// back whole with [`restore_memory`](Self::restore_memory).
    pub fn restore_notes(&mut self, notes: Option<String>) {
        self.set_notes(notes);
    }

    fn set_notes(&mut self, notes: Option<String>) {
        self.notes = notes;
        self.sync_memory_mirror();
    }

    /// Bring the save's copy of this seat's memory up to date.
    fn sync_memory_mirror(&self) {
        if let Some(cell) = &self.memory_mirror {
            if let Ok(mut c) = cell.lock() { *c = Some(self.memory()); }
        }
    }

    /// The notes the next decision prompt will carry, for tests.
    #[must_use]
    pub fn notes_for_test(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// Drive the backend's structured call directly, for backend tests.
    pub fn backend_send_with_schema_for_test(&mut self, message: &str, schema: &serde_json::Value) -> serde_json::Value {
        self.backend.set_seat(&self.name);
        self.backend.send_with_schema(message, schema)
    }

    /// Take the backend's reasoning for the last decision, for backend tests.
    pub fn backend_take_thinking_for_test(&mut self) -> Option<String> {
        self.backend.take_thinking()
    }

    /// The backend's model label, for backend tests.
    #[must_use]
    pub fn model_name_for_test(&self) -> &str {
        self.backend.model_name()
    }

    /// Expose `last_log_index` for testing.
    #[must_use]
    pub fn last_log_index_for_test(&self) -> usize {
        self.last_log_index
    }

    /// Get the model identifier (e.g. "claude-sonnet-4-6", "gemini-2.5-flash").
    #[must_use]
    pub fn model_name(&self) -> &str {
        self.backend.model_name()
    }

    /// The seat answered, but the answer could not be used, so the harness
    /// substituted a fallback. Logged exactly as before and also counted, so
    /// the run summary can say it happened: the transport failure is loud on
    /// stderr, but this — the answer-was-garbage case — was recorded only in
    /// an optional `--log` file under a label a reader had to know to grep
    /// for, and a game in which every single decision was made by the
    /// fallback looked, on screen, like a seat that had played normally
    /// (issue #211).
    ///
    /// **Every substitution goes through here.** It was wired to four of the
    /// ten structured prompts, and the other six each put a legal NO-OP in
    /// the seat's place — no targets marked, no attackers, no blockers,
    /// everything in one pile, the order as listed, a cancelled concede.
    /// That is exactly the answer a seat would give if it had decided to do
    /// nothing, so the log and the tally both read it as a decision: four
    /// mute target-set prompts in a row produced `CHOSE 0 target(s)`, no
    /// `MALFORMED` line, and a run summary with no rejection suffix at all
    /// (issue #399). A fallback nobody can count is a fallback nobody
    /// knows happened.
    #[track_caller]
    fn log_rejected(&self, content: &str) {
        /// The first line, clipped: what a terminal line can carry.
        fn first_line(s: &str) -> String {
            s.lines().next().unwrap_or("").chars().take(200).collect()
        }
        // A backend that never answered is not a seat that answered badly.
        // The fallback is the same; what the operator should do about it is
        // the opposite — fix the CLI, or fix the model — and the log said
        // the seat sent `{}`, quoting an answer it never gave (#587).
        // Each rejection is also one line on stderr as it happens. The log
        // holds a worker's records until its scope ends, so the log is in
        // order and the same on every run of one seed, and stderr is where
        // a seat going wrong is seen live — as the backends' `API_*` lines
        // already are (#658).
        if let Some(why) = &self.last_call_failure {
            crate::stderr_line!("[{}] NO_ANSWER: {why}; {}", self.name, first_line(content));
            self.log_at(crate::game_log::LogLevel::Error, "NO_ANSWER",
                &format!("{why}; {content}"));
            record_llm_unanswered(self.backend.model_name(), self.name());
            return;
        }
        crate::stderr_line!("[{}] MALFORMED: {}", self.name, first_line(content));
        // `LogLevel::Error` is documented as being for exactly this —
        // "malformed LLM responses, API retries ..., fallback activations" —
        // and this was written at Info, so `grep ERROR` over a game log
        // found nothing even when a seat had been mute for eight minutes
        // (#399).
        self.log_at(crate::game_log::LogLevel::Error, "MALFORMED", content);
        record_llm_rejected(self.backend.model_name(), self.name());
    }

    #[track_caller]
    fn log(&self, label: &str, content: &str) {
        self.log_at(crate::game_log::LogLevel::Info, label, content);
    }

    /// Write down a conversation's id the first time it is seen.
    ///
    /// A `claude -p` seat mints a fresh uuid per conversation, passes it as
    /// `--session-id` and `--resume`s it after — and a 2-seat best-of-3 run
    /// minted eight of them without one appearing in the `--log`, the
    /// `--save` snapshot, or on stderr. That id is the only handle to the
    /// CLI's own stored transcript of the conversation: after the run the
    /// transcript exists and is unfindable. It is also the only way to
    /// check, from a run that already happened, that a seat's calls really
    /// were one session — the property #481 was about, which until now
    /// could only be established by re-running the whole thing under a
    /// wrapper (issue #542).
    fn log_session(&mut self) {
        let Some(sid) = self.backend.session_id() else { return };
        if self.session_logged.as_deref() == Some(sid) {
            return;
        }
        let sid = sid.to_string();
        self.log("SESSION", &sid);
        self.session_logged = Some(sid);
    }

    #[track_caller]
    fn log_debug(&self, label: &str, content: &str) {
        self.log_at(crate::game_log::LogLevel::Debug, label, content);
    }

    #[allow(dead_code)]
    #[track_caller]
    fn log_error(&self, label: &str, content: &str) {
        self.log_at(crate::game_log::LogLevel::Error, label, content);
    }

    /// #[`track_caller`] propagates the source location from the caller of
    /// `log`/`log_debug`/`log_error`, not from inside this function — so
    /// `Location::caller()` reports the original call site.
    #[track_caller]
    fn log_at(&self, level: crate::game_log::LogLevel, label: &str, content: &str) {
        let loc = std::panic::Location::caller();
        let full_label = format!("{} [{}]", label, self.name);
        crate::game_log::write_at(level, loc.file(), loc.line(), &full_label, content);
    }

    /// Check if the AI should auto-pass (nothing interesting to do).
    fn should_auto_pass(_view: &GameView, actions: &[Action]) -> bool {
        let has_pass = actions.iter().any(|a| matches!(a, Action::PassPriority));
        if !has_pass {
            return false;
        }
        // Auto-pass when the only options are Pass, Concede, and/or mana abilities.
        // Tapping mana with nothing to cast is pointless.
        actions.iter().all(|a| matches!(a,
            Action::PassPriority | Action::Concede | Action::ActivateManaAbility { .. }
        ))
    }

    /// Rewrite a single engine log entry so it reads as "you"/"opp"
    /// relative to the viewing player, instead of the engine-global
    /// `p0`/`p1` labels. Handles the turn banner, the game-started
    /// wrapper, possessives (`p0's` → `your` / `opp's`), and the
    /// present-tense verb after a subject that becomes "you" (`base_form`).
    fn rewrite_log_entry(entry: &str, you: mtg_engine::ids::PlayerId) -> String {
        if let Some(rewritten) = Self::rewrite_turn_banner(entry, you) {
            return rewritten;
        }
        if let Some(rewritten) = Self::rewrite_game_started(entry, you) {
            return rewritten;
        }
        Self::generic_player_rewrite(entry, you)
    }

    /// Rewrite `── Turn N (pX) ──` → `── Turn N (your turn) ──` /
    /// `── Turn N (opp's turn) ──`. Returns None if the entry doesn't
    /// match the banner format.
    fn rewrite_turn_banner(entry: &str, you: mtg_engine::ids::PlayerId) -> Option<String> {
        let stripped = entry.strip_prefix("── Turn ")?.strip_suffix(" ──")?;
        let (num_str, rest) = stripped.split_once(' ')?;
        let rest = rest.strip_prefix('(')?.strip_suffix(')')?;
        let id: u8 = rest.strip_prefix('p')?.parse().ok()?;
        let whose = if id == you.0 { "your turn" } else { "opp's turn" };
        Some(format!("── Turn {num_str} ({whose}) ──"))
    }

    /// Rewrite `Game started (pX on the play)` →
    /// `Game started (you are on the play)` /
    /// `Game started (opp is on the play)`.
    fn rewrite_game_started(entry: &str, you: mtg_engine::ids::PlayerId) -> Option<String> {
        let stripped = entry
            .strip_prefix("Game started (p")?
            .strip_suffix(" on the play)")?;
        let id: u8 = stripped.parse().ok()?;
        let phrase = if id == you.0 { "you are on the play" } else { "opp is on the play" };
        Some(format!("Game started ({phrase})"))
    }

    /// Scan an entry for `p\d+` tokens (word-boundary aware) and rewrite
    /// each to the appropriate "You"/"Opp" form, handling possessives
    /// and verb conjugation when the subject becomes the viewing player.
    fn generic_player_rewrite(entry: &str, you: mtg_engine::ids::PlayerId) -> String {
        let mut out = String::with_capacity(entry.len() + 16);
        let mut remaining = entry;
        while !remaining.is_empty() {
            let prev_is_word = out.as_bytes().last()
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
            if !prev_is_word && remaining.starts_with('p') {
                let after_preposition = follows_preposition(&out);
                if let Some((rewritten, consumed)) = Self::try_rewrite_player_token(remaining, you, after_preposition) {
                    // A line that opens with a possessive opens with a
                    // capital, as one that opens with "You" does (#737).
                    if out.is_empty() {
                        let mut cs = rewritten.chars();
                        out.extend(cs.next().map(|c| c.to_ascii_uppercase()));
                        out.push_str(cs.as_str());
                    } else {
                        out.push_str(&rewritten);
                    }
                    remaining = &remaining[consumed..];
                    continue;
                }
            }
            let c = remaining.chars().next().unwrap();
            out.push(c);
            remaining = &remaining[c.len_utf8()..];
        }
        out
    }

    /// Try to match `p<digits>` at the start of `s` and return the
    /// rewritten text plus the number of input bytes to skip. Handles
    /// possessive (`p0's`) and the present-tense verb after a token that
    /// becomes "You", unless the token is a preposition's object.
    fn try_rewrite_player_token(s: &str, you: mtg_engine::ids::PlayerId, after_preposition: bool) -> Option<(String, usize)> {
        let bytes = s.as_bytes();
        if bytes.len() < 2 || bytes[0] != b'p' || !bytes[1].is_ascii_digit() {
            return None;
        }
        let mut end = 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        let id: u8 = s[1..end].parse().ok()?;
        let is_you = id == you.0;
        let rest = &s[end..];

        // Possessive: `p{N}'s` → `your` / `opp's`
        if rest.starts_with("'s") {
            let tag = if is_you { "your" } else { "opp's" };
            return Some((tag.to_string(), end + 2));
        }

        // Present-tense verb conjugation when subject becomes "You": any
        // third-person verb, not a list of them. The list named six, so
        // every other player-subject line read "You reveals hand", "You
        // has an empty library", "each creature You controls" (#737).
        if is_you && !after_preposition {
            if let Some(word) = rest.strip_prefix(' ') {
                let len = word.bytes().take_while(u8::is_ascii_lowercase).count();
                let ends = word.as_bytes().get(len).is_none_or(|c| !c.is_ascii_alphanumeric() && *c != b'\'');
                if len > 0 && ends {
                    if let Some(base) = base_form(&word[..len]) {
                        return Some((format!("You {base}"), end + 1 + len));
                    }
                }
            }
        }

        // Default substitution.
        let tag = if is_you { "You" } else { "Opp" };
        Some((tag.to_string(), end))
    }

    /// Turn/step header line for the top of every prompt. When a pre-game
    /// phase override is given (mulligan/bottoming), it's used verbatim.
    /// The step name the header line shows.
    ///
    /// The system prompt tells the model which names to expect, so this is
    /// the single place they are spelled — `game_rules_documents_every_step`
    /// checks the list in GAME_RULES against what this returns, which is how
    /// the two used to drift apart (issue #201).
    fn step_name(step: Step, first_strike_damage_step: bool) -> &'static str {
        match step {
            Step::PrecombatMain => "Main Phase 1",
            Step::PostcombatMain => "Main Phase 2",
            Step::BeginCombat => "Begin Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::CombatDamage if first_strike_damage_step => "First-Strike Combat Damage",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End Combat",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::EndStep => "End Step",
            Step::Untap => "Untap",
            Step::Cleanup => "Cleanup",
        }
    }

    /// The context marker and numbered option list that close every
    /// action prompt: one option per line, and copies of one permanent
    /// offering the same ability on one line with an index per copy.
    ///
    /// It used to be one comma-joined line with no cap and no grouping, and
    /// activated abilities were the one row class nothing collapsed: 79
    /// copies of a creature with one ability were 79 rows differing only in
    /// their `(#id)`, 8,898 characters on one unwrapped line, with the one
    /// row the seat wanted 7,900 characters in (issue #461).
    ///
    /// GAME_RULES quotes this shape back to the model, and
    /// `game_rules_shows_the_action_list_it_actually_sends` builds the
    /// documented examples through this function, so the two cannot drift
    /// (issue #201).
    ///
    /// Everything printed here is rewritten into the reader's own
    /// vocabulary. The engine labels players globally, and the harness
    /// rewrites that at each place it surfaces: log entries, the resolution
    /// description, the context line (#465). The rows were the one section
    /// nothing touched, so `Undead Alchemist (#3): instead p1 mills 4 cards`
    /// reached a seat whose system prompt never defines `p1` — while the
    /// line above it said the damage was "to you". A seat reading `p1` as
    /// its opponent picks the mill and empties its own library (#543).
    /// Doing it here rather than at each caller means a row cannot be added
    /// that skips it; the rewrite leaves text with no `p<N>` in it alone,
    /// so the already-rewritten context line passes through unchanged.
    fn format_action_prompt(context: Option<&str>, rows: &[ActionRow], you: mtg_engine::ids::PlayerId) -> String {
        let mut lines: Vec<String> = Vec::with_capacity(rows.len());
        let mut index = 0usize;
        for row in rows {
            let label = Self::generic_player_rewrite(row.label(), you);
            match row {
                ActionRow::One(_) => {
                    lines.push(format!("{index}: {label}"));
                    index += 1;
                }
                ActionRow::Copies { ids, .. } => {
                    let first = index;
                    let last = index + ids.len() - 1;
                    let per_copy: Vec<String> = ids.iter().enumerate()
                        .map(|(k, id)| format!("{}=#{}", first + k, id.0))
                        .collect();
                    lines.push(format!(
                        "{first}-{last}: {label} — one per copy: {}",
                        per_copy.join(", ")
                    ));
                    index = last + 1;
                }
            }
        }
        let context_line = context
            .map(|c| format!("[{}]\n", Self::generic_player_rewrite(c, you)))
            .unwrap_or_default();
        format!("{context_line}Available actions:\n{}\n", lines.join("\n"))
    }

    fn format_turn_header(view: &GameView, header_override: Option<&str>) -> String {
        if let Some(h) = header_override {
            return format!("{h}\n");
        }
        let step_name = Self::step_name(view.step, view.first_strike_damage_step);
        let whose_turn = if view.active_player == view.you { "your turn" } else { "opp's turn" };
        format!("Turn {} - {} ({})\n", view.turn_number, step_name, whose_turn)
    }

    /// Rest of the game-state body below the turn header and "Recent events"
    /// section: life totals, mana pool, boards, stack, hand, graveyards,
    /// flashback. Does NOT include the turn header itself.
    fn format_state_body(view: &GameView) -> String {
        let mut s = String::new();

        // Zone counts
        let your_gy_count: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards.len()).sum();
        let your_exile_count = view.exile.iter().filter(|c| c.owner == view.you).count();
        let opp_gy_count: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid != view.you)
            .map(|(_, cards)| cards.len()).sum();
        let opp_exile_count = view.exile.iter().filter(|c| c.owner != view.you).count();

        writeln!(s, "You: {}hp, {}cards, {}lib, {}gy, {}exile",
            view.your_life, view.your_hand.len(), view.your_library_size,
            your_gy_count, your_exile_count).unwrap();
        for opp in &view.opponents {
            writeln!(s, "Opp: {}hp, {}cards, {}lib, {}gy, {}exile",
                opp.life, opp.hand_size, opp.library_size,
                opp_gy_count, opp_exile_count).unwrap();
        }

        // What this turn has done that no zone shows: morbid reads whether
        // a creature died, a werewolf flips at the next upkeep on the spell
        // counts. They reached the seat only as a "Recent events" line in
        // the one prompt after the event, and the prompt after that had no
        // trace of them (#832).
        let cast = |mine: bool| -> u32 {
            view.spells_cast_this_turn.iter()
                .filter(|(p, _)| (*p == view.you) == mine)
                .map(|(_, n)| n).sum()
        };
        writeln!(s, "This turn: spells cast — you {}, opp {}{}",
            cast(true), cast(false),
            if view.creature_died_this_turn { "; a creature died" } else { "" }).unwrap();

        if !view.your_mana_pool.is_empty() {
            let pool_parts: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{t:?}:{v}"))
                .collect();
            writeln!(s, "Mana pool: {}", pool_parts.join(", ")).unwrap();
        }
        // The opponent's floating mana is what it can still cast with this
        // step. The "tapped … for mana" event used to be the only place it
        // was said, and those events are not sent any more (#865).
        for opp in view.opponents.iter().filter(|o| !o.mana_pool.is_empty()) {
            let pool_parts: Vec<String> = opp.mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{t:?}:{v}"))
                .collect();
            writeln!(s, "Opp mana pool: {}", pool_parts.join(", ")).unwrap();
        }

        // Battlefield — compact
        let your_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller == view.you).collect();
        let opp_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller != view.you).collect();
        let all_perms: Vec<_> = view.battlefield.iter().collect();

        if !your_perms.is_empty() {
            s.push_str("Your board:\n  ");
            s.push_str(&Self::format_perms_compact(&your_perms, &all_perms, view.you));
            s.push('\n');
        }
        if !opp_perms.is_empty() {
            s.push_str("Opp board:\n  ");
            s.push_str(&Self::format_perms_compact(&opp_perms, &all_perms, view.you));
            s.push('\n');
        }

        // Stack
        if !view.stack.is_empty() {
            s.push_str("Stack:\n");
            let mut entries: Vec<String> = Vec::new();
            for i in &view.stack {
                let who = if i.controller == view.you { "your" } else { "opponent's" };
                let targets_str = if i.targets.is_empty() {
                    String::new()
                } else {
                    let target_names: Vec<String> = i.targets.iter()
                        .map(|t| match t {
                            mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                            mtg_engine::actions::Target::Player(pid) => {
                                if *pid == view.you { "you".into() } else { "opponent".into() }
                            }
                            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                        })
                        .collect();
                    format!(" targeting {}", target_names.join(", "))
                };
                // The announced X is public (CR 601.2b, 400.2) — without it
                // a Devil's Play for 12 and one for 0 read identically here
                // too, so the seat could not tell what it was responding to
                // (issue #259).
                let x = i.x_value.map_or_else(String::new, |x| format!(" (X={x})"));
                // Which permanent or card the entry is for (#555). The
                // seat's `obj_name` carries the id everywhere else, and its
                // targets line already did; the entry itself did not, so N
                // triggers from N same-named sources were N identical lines.
                let id = i.source_id.map(|s| format!(" (#{})", s.0)).unwrap_or_default();
                // The entry's controller sits by the entry, not after the
                // targets: there it read as one phrase with the target's own
                // tag — "Grizzly Bears (#228) (opponent's) (your)" (#671).
                // The spell or ability resolving now is on the stack too
                // (#766, #833): the seat answering its question saw no
                // stack at all.
                let resolving = if i.resolving { " — resolving" } else { "" };
                entries.push(format!("  {}{id}{x} ({who}){}{resolving}", i.name, targets_str));
            }
            // Identical entries next to each other — one source's
            // triggers — are one line with their count.
            for entry in Self::collapse_repeats(entries) {
                s.push_str(&entry);
                s.push('\n');
            }
        }

        // Hand
        if !view.your_hand.is_empty() {
            s.push_str("Hand:\n");
            for c in &view.your_hand {
                let cost = c.cost.as_ref().map(|co| format!(" {co}")).unwrap_or_default();
                let pt = match (c.power, c.toughness) {
                    (Some(p), Some(t)) => format!(" {p}/{t}"),
                    _ => String::new(),
                };
                writeln!(s, "  {}{}{}", c.name, cost, pt).unwrap();
            }
        }

        // Graveyard contents (both players)
        for (pid, cards) in &view.graveyards {
            if !cards.is_empty() {
                let whose = if *pid == view.you { "Your" } else { "Opp" };
                writeln!(s, "{whose} graveyard:").unwrap();
                s.push_str(&Self::grouped_cards(cards));
            }
        }

        // Exile, both players', the way the graveyards are listed: the one
        // public zone the prompt only counted. A seat holding Runic
        // Repetition could not see which of its flashback cards it could
        // return, and the opponent's exiled cards were rules text under
        // "cards in view" alone (#828). The CLI has its exile view and the
        // page its exile overlay.
        for (whose, mine) in [("Your", true), ("Opp", false)] {
            let cards: Vec<&mtg_engine::view::CardView> = view.exile.iter()
                .filter(|c| (c.owner == view.you) == mine)
                .collect();
            if !cards.is_empty() {
                writeln!(s, "{whose} exile:").unwrap();
                s.push_str(&Self::grouped_cards(cards));
            }
        }

        // Show flashback-eligible cards in your graveyard.
        //
        // Every cost the card carries, not just the printed one: CR 702.33
        // lets a card have several instances of flashback at once, and this
        // section is the only place in the prompt that names a flashback cost
        // at all. Naming the printed one alone stated the cost of a row the
        // seat was not offered while saying nothing about the one it was
        // (issue #611).
        let your_gy = view.graveyards.iter()
            .find(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards);
        if let Some(gy_cards) = your_gy {
            let fb_cards: Vec<&mtg_engine::view::CardView> = gy_cards.iter()
                .filter(|c| !c.flashback_costs.is_empty())
                .collect();
            if !fb_cards.is_empty() {
                s.push_str("Flashback available:\n");
                for c in &fb_cards {
                    let costs: Vec<String> = c.flashback_costs.iter()
                        .map(ToString::to_string).collect();
                    writeln!(s, "  {} (flashback {})", c.name, costs.join(" or ")).unwrap();
                }
            }
        }

        s
    }

    /// Compact a card's `oracle_text` into a short inline effect summary for the
    /// board display. Drops the leading "Enchant <type>" targeting line (not
    /// useful once the aura is attached), strips reminder text in parentheses,
    /// collapses whitespace, and joins remaining lines with "; ". Returns an
    /// empty string for cards with no oracle text.
    fn short_effect_summary(oracle_text: &str) -> String {
        // Hard cap so a single permanent can't blow up the board line.
        const MAX: usize = 200;

        if oracle_text.is_empty() {
            return String::new();
        }
        // Strip parenthesized reminder text: "(You can't ...)".
        let mut stripped = String::with_capacity(oracle_text.len());
        let mut depth = 0i32;
        for ch in oracle_text.chars() {
            match ch {
                '(' => depth += 1,
                ')' => { if depth > 0 { depth -= 1; } }
                _ => { if depth == 0 { stripped.push(ch); } }
            }
        }

        let lines: Vec<String> = stripped
            .split('\n')
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .filter(|l| {
                // Drop the "Enchant <something>" targeting line — once attached,
                // what it enchants is implicit from the display grouping.
                let lower = l.to_lowercase();
                !lower.starts_with("enchant ")
            })
            .collect();

        let joined = lines.join("; ");
        if joined.chars().count() > MAX {
            let mut out: String = joined.chars().take(MAX - 3).collect();
            out.push_str("...");
            out
        } else {
            joined
        }
    }

    fn format_perms_compact(
        perms: &[&mtg_engine::view::PermanentView],
        all_perms: &[&mtg_engine::view::PermanentView],
        view_you: mtg_engine::ids::PlayerId,
    ) -> String {
        // Group lands by name with tapped count.
        let lands: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Creature)).collect();
        let other: Vec<_> = perms.iter().filter(|p|
            !p.card_types.contains(&CardType::Land) && !p.card_types.contains(&CardType::Creature)
        ).collect();

        let mut parts = Vec::new();

        if !lands.is_empty() {
            // By name, and a land another player owns apart from its
            // controller's own: it goes back when the control effect ends
            // (#740).
            let mut land_groups: Vec<(String, Option<&str>, usize, usize)> = Vec::new();
            for land in &lands {
                let owner = Self::owner_flag(land, view_you);
                if let Some(entry) = land_groups.iter_mut().find(|(n, o, _, _)| *n == land.name && *o == owner) {
                    if land.tapped { entry.3 += 1; } else { entry.2 += 1; }
                } else {
                    let (u, t) = if land.tapped { (0, 1) } else { (1, 0) };
                    land_groups.push((land.name.clone(), owner, u, t));
                }
            }
            for (name, owner, untapped, tapped) in &land_groups {
                let total = untapped + tapped;
                let owner = owner.map(|o| format!(" [{o}]")).unwrap_or_default();
                if *tapped == 0 {
                    parts.push(format!("{total}x {name}{owner}"));
                } else if *untapped == 0 {
                    parts.push(format!("{total}x {name} (tapped){owner}"));
                } else {
                    parts.push(format!("{total}x {name} ({tapped} tapped){owner}"));
                }
            }
        }

        // Collect attached aura/equipment descriptions by what they're attached to
        // — search ALL permanents so we find attachments that cross controller
        // boundaries (e.g., opponent's Pacifism on your creature).
        let mut aura_map: std::collections::HashMap<mtg_engine::ids::ObjectId, Vec<String>> = std::collections::HashMap::new();
        for o in all_perms {
            if o.attached_to.is_some() && !o.card_types.contains(&CardType::Land) && !o.card_types.contains(&CardType::Creature) {
                if let Some(target_id) = o.attached_to {
                    let desc = Self::short_effect_summary(&o.oracle_text);
                    let entry = if desc.is_empty() {
                        o.name.clone()
                    } else {
                        format!("{}: {}", o.name, desc)
                    };
                    aura_map.entry(target_id).or_default().push(entry);
                }
            }
        }

        for c in &creatures {
            let power = c.effective_power.or(c.power).unwrap_or(0);
            let toughness = c.effective_toughness.or(c.toughness).unwrap_or(0);
            // "legendary" leads the ability words: it is the supertype that
            // arms the legend rule (CR 704.5j), and the board text never
            // carried it (issue #333).
            let mut words: Vec<String> = Vec::new();
            if Self::is_legendary(c) { words.push("legendary".into()); }
            // Color (CR 105.2), which intimidate is decided entirely by
            // (CR 702.13a) and which no seat was ever told — the CLI could
            // not show it either, and for a face with no mana cost it was
            // unobtainable from anything on screen (issue #357).
            // "colorless" is stated, not left out: it is what makes a
            // Galvanic Juggernaut blockable by artifact creatures alone.
            words.push(Self::format_colors(&c.colors));
            let kw = Self::format_abilities(c);
            if !kw.is_empty() { words.push(kw); }
            let kw_str = if words.is_empty() { String::new() } else { format!(" {}", words.join(", ")) };

            let mut flag_parts: Vec<String> = Vec::new();
            // A token's name is its subtypes alone (CR 111.4), so the board
            // says "token" here rather than in the name (issues #331, #334).
            if c.is_token { flag_parts.push("token".into()); }
            // CR 707.2: a copy is a different permanent from what it copied
            // — an Evil Twin clone can never transform (CR 701.28c) whatever
            // face it shows, and the seat cannot tell from the name, the P/T
            // or its own decklist, all of which are the copied card's. The
            // granted ability below says it only when the copy effect had an
            // "except it has ..." clause to grant (#557).
            else if c.is_copy { flag_parts.push("copy".into()); }
            // Whose card it is, when not its controller's: what it goes back
            // to when the control effect ends (CR 611.2b), and whose
            // graveyard it goes to (CR 404.1). A stolen Vampire Interloper
            // read like the thief's own four (#740).
            if let Some(owner) = Self::owner_flag(c, view_you) { flag_parts.push(owner.into()); }
            if c.tapped { flag_parts.push("T".into()); }
            // `S` is the engine's answer to CR 302.6, not the raw
            // "entered this turn" field: a hasty creature carries that field
            // for its whole first turn, so the seat used to be handed
            // `haste [S]` and a legend defining `[S]` as "can't attack"
            // about a creature the very next section offered as a legal
            // attacker (#605, the harness's half of #139).
            if c.affected_by_summoning_sickness { flag_parts.push("S".into()); }
            // Combat roles (CR 506.4), which the CLI marks `[ATK]`/`[BLK]`
            // (#245) and the page badges. This board never said them, and
            // "Recent events" is a delta since the last prompt, so from the
            // second prompt of a combat step an attacker read as plain `[T]`
            // and a vigilant one or a blocker as idle — while the seat chose
            // pump and removal targets (#724).
            let named = |id: ObjectId| all_perms.iter().find(|p| p.object_id == id)
                .map_or_else(|| format!("#{}", id.0), |p| format!("{} (#{})", p.name, id.0));
            match &c.attacking {
                Some(mtg_engine::view::AttackTarget::Player(p)) =>
                    flag_parts.push(if *p == view_you { "attacking you".into() } else { "attacking Opp".into() }),
                Some(mtg_engine::view::AttackTarget::Planeswalker(pw)) =>
                    flag_parts.push(format!("attacking {}", named(*pw))),
                None => {}
            }
            if !c.blocking.is_empty() {
                flag_parts.push(format!("blocking {}",
                    c.blocking.iter().map(|id| named(*id)).collect::<Vec<_>>().join(" and ")));
            }
            if !c.blocked_by.is_empty() {
                flag_parts.push(format!("blocked by {}",
                    c.blocked_by.iter().map(|id| named(*id)).collect::<Vec<_>>().join(" and ")));
            } else if c.blocked {
                // Still blocked with every blocker gone (CR 509.1h): it
                // deals no combat damage, and read as unblocked (#725).
                flag_parts.push("blocked (no blockers left)".into());
            }
            if c.damage_marked > 0 { flag_parts.push(format!("{}dmg", c.damage_marked)); }
            // Which permanents dealt it damage this turn: Abattoir Ghoul's
            // trigger looks back over the whole turn, and the damage line
            // was in one prompt's recent events (#932). Its own fact, not
            // the marked damage's: a regenerated creature keeps it with no
            // damage marked. A spell that dealt damage is gone, and is not
            // named.
            let from: Vec<String> = c.damaged_by.iter()
                .filter(|id| all_perms.iter().any(|p| p.object_id == **id))
                .map(|id| named(*id)).collect();
            if !from.is_empty() {
                flag_parts.push(format!("damaged this turn by {}", from.join(" and ")));
            }
            // A live regeneration shield (CR 701.15a), which decides whether
            // removal is worth casting and whether an attack trades. The seat
            // could not find it anywhere: not here, not in the log, which
            // mentions a shield only when it is spent (issue #468).
            match c.regeneration_shields {
                0 => {}
                1 => flag_parts.push("regen shield".into()),
                n => flag_parts.push(format!("{n} regen shields")),
            }
            if let Some(suffix) = Self::format_counters(&c.counters) {
                flag_parts.push(suffix);
            }
            let flags_str = if flag_parts.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flag_parts.join(","))
            };
            let auras = aura_map.get(&c.object_id)
                .map(|entries| format!(" ({})", entries.join("; ")))
                .unwrap_or_default();
            // CR 706.2: an ability the copy effect added ("except it has
            // ..."). The row is built from the copied card's name and
            // characteristics, and the seat reads ability text off its own
            // decklist — which is the copied card's text, so this ability is
            // in neither. The CLI's #501 half of this, on the board the
            // model actually sees.
            let granted = if c.granted_abilities.is_empty() {
                String::new()
            } else {
                format!(" (also has: {})", c.granted_abilities.join("; "))
            };
            parts.push(format!("{} (#{}) {}/{}{}{}{}{}",
                c.name, c.object_id.0, power, toughness, kw_str, flags_str, auras, granted));
        }

        // Show non-aura other permanents. For unattached equipment include
        // the short effect summary; planeswalkers surface their loyalty
        // counter count via format_counters.
        for o in &other {
            if o.attached_to.is_some() { continue; } // skip auras, shown with creature
            let mut flag_parts: Vec<String> = Vec::new();
            if Self::is_legendary(o) { flag_parts.push("legendary".into()); }
            if o.is_token { flag_parts.push("token".into()); }
            if let Some(owner) = Self::owner_flag(o, view_you) { flag_parts.push(owner.into()); }
            if o.tapped { flag_parts.push("T".into()); }
            // The chosen name is the permanent's whole identity (Nevermore's
            // ban) — without it a spell just vanishes from the menu (#130).
            if let Some(n) = &o.named_card {
                flag_parts.push(format!("names: {n}"));
            }
            if let Some(suffix) = Self::format_counters(&o.counters) {
                flag_parts.push(suffix);
            }
            // A Curse's entire identity is whom it enchants (CR 702.5c).
            // It attaches to a *player*, so it is not in `aura_map` (keyed on
            // attached_to, an object) and falls through to here — where the
            // line used to print "enchanted player" with no antecedent
            // anywhere in the prompt, since short_effect_summary drops the
            // "Enchant player" line. Two Curses of the same name on opposite
            // players then rendered identically, and the controller is no
            // proxy for the host: a seat can legally curse itself. This is
            // the CLI's #81 fix, which the prompt never got. One of the
            // flags, in the one bracket the legend defines (#736).
            match o.attached_to_player {
                Some(p) if p == view_you => flag_parts.push("enchanting you".into()),
                Some(_) => flag_parts.push("enchanting opponent".into()),
                None => {}
            }
            let flags_str = if flag_parts.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flag_parts.join(","))
            };
            let desc = Self::short_effect_summary(&o.oracle_text);
            if desc.is_empty() {
                parts.push(format!("{} (#{}){}", o.name, o.object_id.0, flags_str));
            } else {
                parts.push(format!("{} (#{}){} ({})", o.name, o.object_id.0, flags_str, desc));
            }
        }

        parts.join("\n  ")
    }

    /// The board's ownership flag: `owned by opponent` on a permanent you
    /// control that the opponent owns, `owned by you` on one of yours the
    /// opponent controls, nothing when owner and controller agree (#740).
    fn owner_flag(p: &mtg_engine::view::PermanentView, view_you: mtg_engine::ids::PlayerId) -> Option<&'static str> {
        if p.owner == p.controller {
            None
        } else if p.owner == view_you {
            Some("owned by you")
        } else {
            Some("owned by opponent")
        }
    }

    /// Format any +1/+1, -1/-1, or loyalty counters on a permanent into a
    /// compact suffix like `+1+1x2` or `LOYx3`. Returns `None` when the
    /// permanent has none of these counter types (so the caller can omit
    /// the flag entirely).
    fn format_counters(
        counters: &std::collections::HashMap<mtg_engine::types::CounterType, u32>,
    ) -> Option<String> {
        use mtg_engine::types::CounterType;
        let mut bits: Vec<String> = Vec::new();
        if let Some(&n) = counters.get(&CounterType::PlusOnePlusOne) {
            if n > 0 { bits.push(format!("+1+1x{n}")); }
        }
        if let Some(&n) = counters.get(&CounterType::MinusOneMinusOne) {
            if n > 0 { bits.push(format!("-1-1x{n}")); }
        }
        if let Some(&n) = counters.get(&CounterType::Loyalty) {
            if n > 0 { bits.push(format!("LOYx{n}")); }
        }
        for (ct, &n) in counters {
            if n > 0 && !matches!(ct, CounterType::PlusOnePlusOne | CounterType::MinusOneMinusOne | CounterType::Loyalty) {
                bits.push(format!("{ct:?}x{n}"));
            }
        }
        if bits.is_empty() { None } else { Some(bits.join(",")) }
    }

    /// ` targeting X, Y` — what an action row says it is aimed at, or
    /// nothing when the action has no targets.
    fn targets_suffix(view: &GameView, targets: &[mtg_engine::actions::Target]) -> String {
        if targets.is_empty() {
            return String::new();
        }
        format!(" targeting {}", Self::target_labels(view, targets).join(", "))
    }

    /// Format a single non-CastSpell action for the collapsed display.
    fn format_single_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass".into(),
            Action::PlayLand { object_id } => format!("Play {}", Self::own_land_name(view, *object_id)),
            // Name the mana this entry makes. The engine offers one action
            // per (object, ability_index), so dropping the description
            // rendered a dual land's two abilities as byte-identical rows
            // that are not the same action — and this seat is told by
            // GAME_RULES to tap manually "to preserve a specific land",
            // which is the one thing it could not then express. Same
            // lookup the CLI has had since #118 (issue #460).
            Action::ActivateManaAbility { object_id, ability_index } => {
                match view.mana_ability_description(*object_id, *ability_index) {
                    Some(d) => format!("Tap {}: {}", Self::own_land_name(view, *object_id), d),
                    None => format!("Tap {} for mana", Self::own_land_name(view, *object_id)),
                }
            }
            Action::ActivateAbility { object_id, .. } => format!("Activate {}", Self::obj_name(view, *object_id)),
            // Name the ability and say what it is aimed at, not just the
            // index. There was no arm here at all, so the row fell through
            // to the engine's `Display` — `Activate loyalty ability 2 on
            // obj#1` — which says neither what the ability costs in loyalty
            // nor what it does, and, because the engine enumerates one
            // action per target, made "-6 targeting you" and "-6 targeting
            // the opponent" byte-identical rows. A loyalty ability cannot
            // be taken back or retried that turn (CR 606.3). Same lookup
            // the CLI has had since #61 (issue #494).
            Action::ActivateLoyaltyAbility { object_id, ability_index, targets } => {
                let name = Self::obj_name(view, *object_id);
                let suffix = Self::targets_suffix(view, targets);
                match view.loyalty_ability_description(*object_id, *ability_index) {
                    Some(d) => format!("{name}: {d}{suffix}"),
                    None => format!("Activate loyalty ability {ability_index} on {name}{suffix}"),
                }
            }
            Action::Concede => "Concede".into(),
            Action::DiscardCards { cards } => {
                let names: Vec<String> = cards.iter().map(|id| Self::obj_name(view, *id)).collect();
                format!("Discard {}", names.join(", "))
            }
            Action::ResolveChoice { choice } => {
                use mtg_engine::actions::ResolvedChoice;
                match choice {
                    ResolvedChoice::PayDecision(true) => "Pay".into(),
                    ResolvedChoice::PayDecision(false) => "Don't pay".into(),
                    ResolvedChoice::YesNoDecision(true) => "Yes".into(),
                    ResolvedChoice::YesNoDecision(false) => "No".into(),
                    ResolvedChoice::ChosenTarget(Some(t)) => {
                        match t {
                            mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                            mtg_engine::actions::Target::Player(pid) => {
                                if *pid == view.you { "You".into() } else { "Opponent".into() }
                            }
                            mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                        }
                    }
                    ResolvedChoice::ChosenTarget(None) => "Decline".into(),
                    ResolvedChoice::ChosenCard(id) => Self::obj_name(view, *id),
                    ResolvedChoice::ChosenIndex(_, ref label) => {
                        label.clone()
                    }
                    ResolvedChoice::ChosenOrder(order) => format!("Order: {}",
                        order.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")),
                    ResolvedChoice::ChosenSubset(ids) => {
                        let names: Vec<String> = ids.iter()
                            .map(|id| Self::obj_name(view, *id))
                            .collect();
                        format!("Pile A: [{}]", if names.is_empty() { "empty".into() } else { names.join(", ") })
                    }
                    ResolvedChoice::XFunding(response) => format!("Fund X = {}", response.x_value()),
                    ResolvedChoice::ChosenTargetSet(ts) => {
                        if ts.is_empty() {
                            "Target: (none)".into()
                        } else {
                            let names: Vec<String> = ts.iter().map(|t| match t {
                                mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                                mtg_engine::actions::Target::Player(pid) =>
                                    if *pid == view.you { "You".into() } else { "Opponent".into() },
                                mtg_engine::actions::Target::Illegal => "(illegal)".into(),
                            }).collect();
                            format!("Target: {}", names.join(", "))
                        }
                    }
                    ResolvedChoice::ChosenObjectSet(ids) => {
                        if ids.is_empty() {
                            "Choose: (none)".to_string()
                        } else {
                            let names: Vec<String> = ids.iter()
                                .map(|id| Self::obj_name(view, *id))
                                .collect();
                            format!("Choose: [{}]", names.join(", "))
                        }
                    }
                    ResolvedChoice::ChosenExileSet(ids) => {
                        if ids.is_empty() {
                            "Exile: (none)".to_string()
                        } else {
                            let names: Vec<String> = ids.iter()
                                .map(|id| Self::obj_name(view, *id))
                                .collect();
                            format!("Exile: [{}]", names.join(", "))
                        }
                    }
                    ResolvedChoice::CancelCast => "Cancel cast".to_string(),
                }
            }
            other => format!("{other}"),
        }
    }

    /// Second API call: choose targets for a castable spell.
    fn choose_cast_targets(&mut self, view: &GameView, spell: &mtg_engine::actions::CastableSpell, legal_actions: &[Action]) -> Action {
        use mtg_engine::actions::{CastTargetSpec, Target};

        // For ExileXFromGraveyard spells (Harvest Pyre), find the expanded
        // CastSpell action in legal_actions that exiles the maximum number of
        // cards (matching `spell.exile_x_from_gy_max`). choose_cast_targets
        // only picks the target here — exile_count and exile_ids come from
        // the pre-enumerated expanded action so the LLM gets the damage it
        // was promised in the label.
        let pick_expanded = |targets: &[Target]| -> Option<Action> {
            let target_max = spell.exile_x_from_gy_max?;
            legal_actions.iter().find_map(|a| {
                if let Action::CastSpell { object_id, targets: t, exile_count, .. } = a {
                    if *object_id == spell.object_id && *exile_count == Some(target_max) && t.as_slice() == targets {
                        return Some(a.clone());
                    }
                }
                None
            })
        };

        // Step 1: Choose targets based on target_spec.
        let chosen_targets = match &spell.target_spec {
            CastTargetSpec::NoTargets => {
                if let Some(a) = pick_expanded(&[]) { return a; }
                vec![]
            }
            CastTargetSpec::SingleTarget(options) => {
                if options.len() == 1 {
                    if let Some(a) = pick_expanded(std::slice::from_ref(&options[0])) { return a; }
                    vec![options[0].clone()]
                } else {
                    let target = self.prompt_target_selection(view, &format!("{}: select a target", spell.name), options);
                    if let Some(a) = pick_expanded(std::slice::from_ref(&target)) { return a; }
                    vec![target]
                }
            }
            CastTargetSpec::TwoTargets { first, second, second_min, second_max } => {
                // The one pair left here is Memory's Journey: its card slot
                // cannot be described until a player is named, so the player
                // is chosen now and the cards on the prompt the cast then
                // raises (`choose_target_set`). Two slots that can both be
                // described up front are `ChosenAtCast`.
                let _ = (second, second_min, second_max);
                vec![self.prompt_target_selection(
                    view, &format!("{}: select first of two targets", spell.name), first)]
            }
            // The cast asks for these itself (CR 601.2c): submit it bare and
            // answer the `ChooseTargetSet` prompt it raises, which this seat
            // handles in `choose_target_set`.
            CastTargetSpec::ChosenAtCast => vec![],
        };

        // Step 2: Choose sacrifice if the spell has a sacrifice additional cost.
        // NOTE: If you change this, also update the "Sacrifice costs"
        // bullet in GAME_RULES so the agent's system prompt stays accurate.
        let chosen_sacrifice = match spell.sacrifice_options.len() {
            0 => None,
            1 => Some(spell.sacrifice_options[0]),
            _ => {
                let labels: Vec<String> = spell.sacrifice_options.iter()
                    .map(|id| Self::obj_name(view, *id))
                    .collect();
                let prompt = format!(
                    "{}: choose a creature to sacrifice as additional cost\n{}",
                    spell.name,
                    labels.iter().enumerate().map(|(i, l)| format!("{i}: {l}")).collect::<Vec<_>>().join("\n"),
                );
                let idx = self.pick_action_index(view, &prompt, spell.sacrifice_options.len());
                Some(spell.sacrifice_options[idx.min(spell.sacrifice_options.len() - 1)])
            }
        };

        Action::CastSpell {
            object_id: spell.object_id,
            targets: chosen_targets,
            sacrifice: chosen_sacrifice,
            exile_count: None,
            exile_ids: vec![],
            // The cost this entry was offered for (issue #128).
            alternative_cost: spell.alternative_cost.clone(),
            tap_plan: spell.tap_plan.clone(),
        }
    }

    /// Choose targets and sacrifice for an activated ability via sequential prompts.
    /// Instead of presenting every (target × sacrifice) combo as a flat list,
    /// we ask the model to pick each dimension separately.
    fn choose_ability_targets(&mut self, view: &GameView, ab: &mtg_engine::actions::ActivatableAbility, _legal_actions: &[Action]) -> Action {
        if ab.option_combos.is_empty() {
            return Action::PassPriority;
        }
        if ab.option_combos.len() == 1 {
            let chosen = &ab.option_combos[0];
            return Action::ActivateAbility {
                object_id: ab.object_id,
                ability_index: ab.ability_index,
                targets: chosen.targets.clone(),
                tap_plan: ab.tap_plan.clone(),
                sacrifice: chosen.sacrifice,
                x_value: None,
                source_card_id: ab.source_card_id,
            };
        }

        // Collect unique targets and unique sacrifices from all combos.
        let (unique_target_sets, _) = Self::ability_slots(ab);

        // Step 1: Pick targets (if there are multiple target options)
        let chosen_targets = if unique_target_sets.len() <= 1 {
            unique_target_sets.first().map(|t| (*t).clone()).unwrap_or_default()
        } else {
            let labels: Vec<String> = unique_target_sets.iter().map(|targets| {
                if targets.is_empty() {
                    return String::new(); // shouldn't happen if >1 unique set
                }
                targets.iter().map(|t| match t {
                    mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                    mtg_engine::actions::Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                    mtg_engine::actions::Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
                }).collect::<Vec<_>>().join(", ")
            }).collect();
            let prompt = format!(
                "{}: choose a target for {}\n{}",
                ab.name,
                ab.description,
                labels.iter().enumerate().map(|(i, l)| format!("{i}: {l}")).collect::<Vec<_>>().join("\n"),
            );
            let idx = self.pick_action_index(view, &prompt, unique_target_sets.len());
            unique_target_sets[idx.min(unique_target_sets.len() - 1)].clone()
        };

        // Step 2: Pick sacrifice (if there are multiple sacrifice options)
        // Filter to sacrifices that are valid with the chosen targets.
        let valid_sacrifices: Vec<Option<ObjectId>> = ab.option_combos.iter()
            .filter(|opt| opt.targets == chosen_targets)
            .map(|opt| opt.sacrifice)
            .collect();
        let mut unique_valid_sacs: Vec<Option<ObjectId>> = Vec::new();
        for s in &valid_sacrifices {
            if !unique_valid_sacs.contains(s) {
                unique_valid_sacs.push(*s);
            }
        }

        let chosen_sacrifice = if unique_valid_sacs.len() <= 1 {
            unique_valid_sacs.first().copied().flatten()
        } else {
            let labels: Vec<String> = unique_valid_sacs.iter().map(|s| {
                match s {
                    Some(id) => Self::obj_name(view, *id),
                    None => "None".into(),
                }
            }).collect();
            let prompt = format!(
                "{}: choose a creature to sacrifice\n{}",
                ab.name,
                labels.iter().enumerate().map(|(i, l)| format!("{i}: {l}")).collect::<Vec<_>>().join("\n"),
            );
            let idx = self.pick_action_index(view, &prompt, unique_valid_sacs.len());
            unique_valid_sacs[idx.min(unique_valid_sacs.len() - 1)]
        };

        Action::ActivateAbility {
            object_id: ab.object_id,
            ability_index: ab.ability_index,
            targets: chosen_targets,
            tap_plan: ab.tap_plan.clone(),
            sacrifice: chosen_sacrifice,
            x_value: None,
            source_card_id: ab.source_card_id,
        }
    }

    /// Make a second API call to select one target from a list.
    fn prompt_target_selection(&mut self, view: &GameView, spell_name: &str, options: &[mtg_engine::actions::Target]) -> mtg_engine::actions::Target {
        assert!(!options.is_empty(), "prompt_target_selection called with no options for {spell_name}");
        // One row per line, through the same two helpers the marked-set
        // prompts use — this had its own copy of the labelling, which put
        // every option on one comma-joined line and named the players in a
        // different case from every other prompt.
        let target_list = Self::numbered_listing(&Self::target_labels(view, options));
        let prompt = format!(
            "{spell_name}:\n{target_list}",
        );
        let idx = self.pick_action_index(view, &prompt, options.len());
        options[idx.min(options.len() - 1)].clone()
    }

    /// Format a tap plan as a compact string like "2x Plains, Hinterland Harbor".
    ///
    /// A plan taps only the seat's own sources and the seat does not pick
    /// them, so they are named bare, as the `Tap <land>` rows name them.
    /// Since #668 gave every battlefield name its id and owner, no two names
    /// in a plan were equal and the grouping never happened: "(tap Island
    /// (#1) (your), Island (#2) (your))", about 46 bytes a menu over the
    /// grouped form, under a GAME_RULES that showed "(tap 2x Island)" (#746).
    fn format_tap_plan(view: &GameView, tap_plan: &[(ObjectId, usize)]) -> String {
        if tap_plan.is_empty() { return String::new(); }
        // Collect names, count duplicates.
        let mut name_counts: Vec<(String, usize)> = Vec::new();
        for &(source_id, _) in tap_plan {
            let name = Self::own_land_name(view, source_id);
            if let Some(entry) = name_counts.iter_mut().find(|(n, _)| *n == name) {
                entry.1 += 1;
            } else {
                name_counts.push((name, 1));
            }
        }
        name_counts.iter()
            .map(|(name, count)| {
                if *count > 1 { format!("{count}x {name}") } else { name.clone() }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// An ability's distinct target sets and distinct sacrifices, in offer
    /// order: the slots `choose_ability_targets` asks one at a time, and a
    /// slot with one value it fills without asking.
    fn ability_slots(ab: &mtg_engine::actions::ActivatableAbility)
        -> (Vec<&Vec<mtg_engine::actions::Target>>, Vec<Option<ObjectId>>)
    {
        let mut targets: Vec<&Vec<mtg_engine::actions::Target>> = Vec::new();
        let mut sacrifices: Vec<Option<ObjectId>> = Vec::new();
        for opt in &ab.option_combos {
            if !targets.contains(&&opt.targets) { targets.push(&opt.targets); }
            if !sacrifices.contains(&opt.sacrifice) { sacrifices.push(opt.sacrifice); }
        }
        (targets, sacrifices)
    }

    /// What a cast or activation row will commit to WITHOUT asking: the
    /// target when there is exactly one (CR 601.2c, 602.2b) and the
    /// creature sacrificed when there is exactly one (CR 601.2h) — `""`
    /// when every slot is still a choice. The seat takes a forced option
    /// without a follow-up prompt, so the row is the only place it can
    /// read where the spell goes: a Brimstone Volley whose one legal
    /// target was its own caster read exactly like one aimed at the
    /// opponent (#827, the CLI's #254). `source` is the activating
    /// permanent, so a cost that sacrifices it reads "itself".
    fn forced_suffix(view: &GameView, targets: &[mtg_engine::actions::Target],
        sacrifice: Option<ObjectId>, source: Option<ObjectId>) -> String
    {
        use mtg_engine::actions::Target;
        let mut s = String::new();
        if !targets.is_empty() {
            let names: Vec<String> = targets.iter().map(|t| match t {
                Target::Object(id) => Self::obj_name(view, *id),
                Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                Target::Illegal => unreachable!("Target::Illegal is substituted at resolution; it is never offered to a player"),
            }).collect();
            write!(s, " targeting {}", names.join(", ")).unwrap();
        }
        match sacrifice {
            Some(id) if Some(id) == source => s.push_str(", sacrificing itself"),
            Some(id) => write!(s, ", sacrificing {}", Self::obj_name(view, id)).unwrap(),
            None => {}
        }
        s
    }

    /// How the seat names an object: in a public zone two objects can share
    /// a name and belong to different players, so the name carries its id
    /// and whose it is — a permanent and a stack object by controller, a
    /// graveyard card by whose graveyard. Only lands on the battlefield used
    /// to go without, and nothing on the stack or in a graveyard had either:
    /// "Destroy target land" listed your Mountain and the opponent's as two
    /// identical rows, and a seat countered its own Dissipate (#668). Hand
    /// and library cards are yours, and copies of one are interchangeable,
    /// so they stay bare.
    fn obj_name(view: &GameView, id: ObjectId) -> String {
        let whose = |p: mtg_engine::ids::PlayerId| if p == view.you { "your" } else { "opponent's" };
        if let Some(p) = view.battlefield.iter().find(|p| p.object_id == id) {
            // A Curse is told apart by whom it enchants, which its controller
            // does not say: two same-named Curses as targets read alike (#736).
            let host = match p.attached_to_player {
                Some(h) if h == view.you => ", enchanting you",
                Some(_) => ", enchanting opponent",
                None => "",
            };
            return format!("{} (#{}) ({}{host})", p.name, id.0, whose(p.controller));
        }
        if let Some(s) = view.stack.iter().find(|s| s.object_id == id) {
            return format!("{} (#{}) ({})", s.name, id.0, whose(s.controller));
        }
        if let Some((owner, c)) = view.graveyards.iter()
            .find_map(|(owner, cards)| cards.iter().find(|c| c.object_id == id).map(|c| (*owner, c)))
        {
            return format!("{} (#{}) (in {} graveyard)", c.name, id.0, whose(owner));
        }
        if let Some(c) = view.exile.iter().find(|c| c.object_id == id) {
            return format!("{} (#{}) (in {} exile)", c.name, id.0, whose(c.owner));
        }
        view.your_hand.iter()
            .find(|c| c.object_id == id)
            .map(|c| c.name.clone())
            .or_else(|| view.your_library_cards.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.revealed_names.get(&id).cloned())
            .unwrap_or_else(|| format!("{id}"))
    }

    /// The bare name of a land you play or a source you tap for mana. Both
    /// are yours, and the engine offers one row per kind of land or per
    /// mana ability, so neither the id nor the owner tells two rows apart.
    fn own_land_name(view: &GameView, id: ObjectId) -> String {
        view.battlefield.iter().find(|p| p.object_id == id).map(|p| p.name.clone())
            .unwrap_or_else(|| Self::obj_name(view, id))
    }

    /// `obj_name` with the id always present, for rows that must tell any
    /// two objects apart wherever they are.
    fn obj_label(view: &GameView, id: ObjectId) -> String {
        let name = Self::obj_name(view, id);
        if name.contains(&format!("(#{})", id.0)) { name } else { format!("{name} (#{})", id.0) }
    }

    /// Log thinking from the last backend call, if any, at info level, and
    /// keep its tail as the seat's notes for the next prompt.
    fn log_thinking(&mut self) {
        if let Some(thinking) = self.backend.take_thinking() {
            self.log("THOUGHT", &thinking);
            if let Some(notes) = Self::notes_from(&thinking) {
                self.set_notes(Some(notes));
            }
        }
    }

    /// The most characters of reasoning carried into the next prompt: two
    /// sentences, which is what `thoughts` asks for. At 600 the notes were a
    /// sixth of every prompt and as long again in the output that wrote
    /// them (reports/llm-cost.md, phase 3).
    const NOTES_MAX_CHARS: usize = 300;

    /// The tail of a decision's reasoning, for the next prompt's notes.
    ///
    /// The tail, because reasoning ends with the decision and the plan
    /// behind it ("…so I pass and block with the Bears next turn") where
    /// it opens with a reading of the board the next prompt restates
    /// anyway. Cut at a word boundary and marked as cut. `None` when there
    /// is nothing to carry — a 5.x model whose thinking is not displayed
    /// hands back an empty block, which is not a note.
    fn notes_from(thinking: &str) -> Option<String> {
        let text = thinking.trim();
        if text.is_empty() {
            return None;
        }
        let chars = text.chars().count();
        if chars <= Self::NOTES_MAX_CHARS {
            return Some(text.to_string());
        }
        let tail: String = text.chars().skip(chars - Self::NOTES_MAX_CHARS).collect();
        let from_word = tail.find(char::is_whitespace).map_or(0, |i| i + 1);
        Some(format!("…{}", tail[from_word..].trim_start()))
    }

    /// Send a message with a custom JSON response schema, returning parsed JSON.
    fn send_message_structured(&mut self, user_message: &str, schema: &serde_json::Value) -> serde_json::Value {
        // Every structured request goes through here, so this is the one
        // place that can see them all. A top-level key the API's pattern
        // refuses is a 400 before the model reads anything, and the seat
        // cannot tell that apart from having declined (#398) — so fail
        // loudly where a developer will see it, and let a live game take
        // the retry path rather than crash mid-tournament.
        debug_assert!(
            schema.get("properties").and_then(serde_json::Value::as_object)
                .is_none_or(|p| p.keys().all(|k| schema_key_is_legal(k))),
            "a top-level schema key is one the API will refuse (#398): {:?}",
            schema.get("properties").and_then(serde_json::Value::as_object)
                .map(|p| p.keys().filter(|k| !schema_key_is_legal(k))
                    .cloned().collect::<Vec<_>>()));
        self.log("PROMPT", user_message);
        self.backend.set_seat(&self.name);
        let result = self.ask_structured(user_message, schema);
        // The request carried no enums or counts (`cache_stable_schema`), so
        // the answer is held to them here, and one that breaks them is asked
        // once more with the reason rather than read as whatever its
        // caller's fallback makes of it. A call that got no answer at all is
        // not asked again: its retries are behind it (#587).
        if self.last_call_failure.is_some() {
            return result;
        }
        let repaired = repair_enum_strings(schema, &result);
        if repaired != result {
            self.log("REPAIRED", &format!("{result} read as {repaired}"));
        }
        let result = with_empty_lists(schema, repaired);
        let Some(why) = schema_violation(schema, &result) else {
            return result;
        };
        // Not a rejection: nothing is substituted, the seat is asked. A
        // malformed answer is an `Error` record, and this is one.
        crate::stderr_line!("[{}] REASK: {why}", self.name);
        self.log_at(crate::game_log::LogLevel::Error, "REASK", &format!("{why}: {result}"));
        let again = format!(
            "{user_message}\n\nYour answer {result} is not one this question allows: {why}. Answer again.");
        self.log("PROMPT", &again);
        let again = self.ask_structured(&again, schema);
        with_empty_lists(schema, again)
    }

    /// One structured call and the logging every call gets.
    fn ask_structured(&mut self, user_message: &str, schema: &serde_json::Value) -> serde_json::Value {
        let result = self.backend.send_with_schema(user_message, schema);
        // Whether this decision got an answer at all, before anything
        // downstream reads the value it was handed (#587).
        self.last_call_failure = self.backend.take_call_failure();
        self.log_session();
        self.log_thinking();
        // Raw backend JSON is verbose and duplicates the THOUGHT line for
        // backends that put thoughts in the JSON — log it at debug level.
        self.log_debug("RESPONSE", &result.to_string());
        result
    }

    /// Build a prompt that includes new log entries + board state + the action prompt.
    fn build_prompt(&mut self, view: &GameView, action_prompt: &str) -> String {
        self.build_prompt_with_header(view, action_prompt, None)
    }

    /// The most log entries one recap carries. Nothing bounded the block,
    /// and it is largest exactly when the most has happened: one prompt
    /// carried 307 entries covering turns 1-98, three quarters of its
    /// length (issue #464). The oldest are dropped, with a marker saying so.
    const MAX_RECENT_EVENTS: usize = 80;

    /// `p1 tapped Plains (#45) for mana (pool: White:1)`, the engine's line
    /// for a mana ability.
    fn is_mana_tap_entry(entry: &str) -> bool {
        (entry.contains(" tapped ") && entry.contains(" for mana")) || entry.contains("'s mana pool empties (")
    }

    /// Consecutive identical lines as one, with how many: eight triggers
    /// from one death, each the same line, said once with `(×8)`. Nothing
    /// is lost but the repetition.
    fn collapse_repeats(lines: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut out: Vec<(String, usize)> = Vec::new();
        for line in lines {
            match out.last_mut() {
                Some((last, n)) if *last == line => *n += 1,
                _ => out.push((line, 1)),
            }
        }
        out.into_iter().map(|(line, n)| if n == 1 { line } else { format!("{line} (×{n})") }).collect()
    }

    /// A graveyard's or an exile's cards, one line per name with how many:
    /// `4x Island`, `2x Walking Corpse 2/2`, in the order each name first
    /// appears. No card in the pool cares about a graveyard's order.
    fn grouped_cards<'a>(cards: impl IntoIterator<Item = &'a mtg_engine::view::CardView>) -> String {
        let mut rows: Vec<(String, usize)> = Vec::new();
        for c in cards {
            let pt = match (c.power, c.toughness) {
                (Some(p), Some(t)) => format!(" {p}/{t}"),
                _ => String::new(),
            };
            let row = format!("{}{pt}", c.name);
            match rows.iter_mut().find(|(r, _)| *r == row) {
                Some((_, n)) => *n += 1,
                None => rows.push((row, 1)),
            }
        }
        rows.into_iter().map(|(row, n)| if n == 1 { format!("  {row}\n") } else { format!("  {n}x {row}\n") }).collect()
    }

    /// The turn number a `── Turn N (...) ──` log banner announces.
    fn turn_banner_number(entry: &str) -> Option<&str> {
        entry.strip_prefix("── Turn ")?.split_once(' ').map(|(n, _)| n)
    }

    /// The notes block: the seat's own plan from its last decision, the
    /// one thing the bounded conversation would otherwise lose (see
    /// `notes`), or `""` before the first decision. Every prompt carries it,
    /// including the ones built outside `build_prompt_with_header`: the
    /// keep-then-bottom pair is the one two-step plan every game has, and
    /// the bottom prompt was asked without the "keep and bottom X" the keep
    /// had just said (#830).
    fn notes_section(&self) -> String {
        self.notes.as_ref().map_or_else(String::new,
            |notes| format!("Your notes from your last decision:\n{notes}\n\n"))
    }

    /// `prompt` with the notes block after its first line, the context
    /// marker (`[MULLIGAN DECISION]`) that leads every prompt.
    fn with_notes_after_marker(&self, prompt: &str) -> String {
        let notes = self.notes_section();
        if notes.is_empty() {
            return prompt.to_string();
        }
        match prompt.split_once('\n') {
            Some((marker, rest)) => format!("{marker}\n\n{notes}{rest}"),
            None => format!("{prompt}\n\n{notes}"),
        }
    }

    fn build_prompt_with_header(
        &mut self,
        view: &GameView,
        action_prompt: &str,
        header_override: Option<&str>,
    ) -> String {
        // Use display_log (Info level and above) to skip debug noise like
        // "passes priority" and "Step: Draw" entries that add no information.
        // Rewrite each new entry so player references read "you"/"opp"
        // instead of the engine-global `p0`/`p1` labels.
        let unread: &[String] = view.display_log
            .get(self.last_log_index..)
            .unwrap_or_default();
        self.last_log_index = view.display_log.len();
        // A land tapped for mana is not news: the cast it paid for is its
        // own line, and the board says what is tapped. It was one line in
        // ten of the events a seat was sent (reports/llm-cost.md, phase 3).
        let pending: Vec<String> = unread.iter()
            .filter(|e| !Self::is_mana_tap_entry(e))
            .cloned()
            .collect();
        let omitted = pending.len().saturating_sub(Self::MAX_RECENT_EVENTS);
        let (elided, shown) = pending.split_at(omitted);
        let new_logs: Vec<String> = Self::collapse_repeats(shown.iter()
            .map(|e| Self::rewrite_log_entry(e, view.you)));

        let mut prompt = String::new();
        // Turn/phase header comes first so the model immediately knows
        // what decision it's being asked to make.
        prompt.push_str(&Self::format_turn_header(view, header_override));
        prompt.push('\n');

        // A resumed seat reads the game so far once, here, in its first
        // prompt; after that the board below is the whole position.
        if let Some(recap) = self.pending_recap.take() {
            prompt.push_str(&recap);
            prompt.push_str("\n\n");
        }
        // The seat's own plan, from its last decision: the one thing the
        // bounded conversation would otherwise lose (see `notes`).
        prompt.push_str(&self.notes_section());
        // Why the seat is being asked at all, when its last answer was
        // "pass until something happens": what happened. The events it
        // passed through are in the recap below, none of them dropped by
        // the passing itself (`last_log_index` moves only here).
        if let Some((passes, why)) = self.pass_until_stopped.take() {
            prompt.push_str(&Self::pass_until_stopped_line(passes, &why));
            prompt.push_str("\n\n");
        }
        if let Some(why) = self.plan_stopped.take() {
            prompt.push_str(&why);
            prompt.push_str("\n\n");
        }

        if !new_logs.is_empty() {
            prompt.push_str("Recent events:\n");
            if omitted > 0 {
                // The marker names the turn the dropped stretch reached, so
                // a reader knows what the kept entries are the tail of.
                let through = elided.iter().rev().find_map(|e| Self::turn_banner_number(e));
                prompt.push_str(&Self::omitted_events_marker(omitted, through));
                prompt.push('\n');
            }
            for entry in &new_logs {
                prompt.push_str(entry);
                prompt.push('\n');
            }
            prompt.push('\n');
        }

        prompt.push_str(&Self::format_state_body(view));
        prompt.push_str(&self.format_cards_in_view(view, action_prompt));
        prompt.push('\n');

        prompt.push_str(action_prompt);
        prompt
    }

    /// The line that opens a capped recap. GAME_RULES quotes this shape,
    /// and a test builds the documented example through it.
    fn omitted_events_marker(omitted: usize, through_turn: Option<&str>) -> String {
        match through_turn {
            Some(t) => format!("… {omitted} earlier entries omitted, through turn {t} …"),
            None => format!("… {omitted} earlier entries omitted …"),
        }
    }

    /// Divide permanents into two piles by asking for a pile, A or B, for each listed permanent.
    /// Used for effects like Liliana of the Veil -6 where a player divides
    /// permanents and the opponent chooses which pile to sacrifice.
    fn choose_pile_division(
        &mut self,
        view: &GameView,
        permanents: &[mtg_engine::ids::ObjectId],
        description: &str,
        target_player: mtg_engine::ids::PlayerId,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;

        let all_ids: Vec<mtg_engine::ids::ObjectId> = permanents.to_vec();
        if all_ids.is_empty() {
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(vec![]) };
        }

        // Build prompt with permanent names
        let context_desc = Self::generic_player_rewrite(description, view.you);
        let mut perm_list = String::new();
        let labels = Self::format_combat_creature_list(view, &all_ids);
        for (i, label) in labels.iter().enumerate() {
            writeln!(perm_list, "  {i}: {label}").unwrap();
        }

        // The one fact that decides the answer: who picks the pile that
        // dies. Dividing the opponent's board you want both piles equally
        // painful, because they take the cheaper one; dividing your own you
        // want one pile empty, and you sacrifice that one. The prompt used
        // to say neither whose permanents these were nor that anything was
        // sacrificed — `legal.context` is only "<source>: divide into
        // piles" — so a seat had to supply the card from memory and split
        // 12/12 down the middle (#495).
        let who_sacrifices = if target_player == view.you {
            "You then choose one of the two piles and sacrifice every permanent in it."
        } else {
            "Your opponent then chooses one of the two piles and sacrifices every permanent in it."
        };

        let action_text = format!(
            "{context_desc}\n{who_sacrifices}\n\
             Answer `piles` with {} entries, \"A\" or \"B\" for each permanent in the order listed.\n\n\
             Permanents:\n{perm_list}",
            labels.len()
        );
        let prompt = self.build_prompt(view, &action_text);

        let schema = Self::pile_division_schema(labels.len());
        let response = self.send_message_structured(&prompt, &schema);

        // Parse response: collect IDs the model put in pile A.
        let mut pile_1_ids: Vec<mtg_engine::ids::ObjectId> = Vec::new();
        if let Some(piles) = response["piles"].as_array() {
            // Every permanent is named, "A" or "B": the schema requires the
            // length, and an answer that skips one anyway is counted. The
            // default for a missing entry (pile B) is a strategic decision
            // made for the seat, and it used to be made in silence (#662).
            let mut unnamed: Vec<&str> = Vec::new();
            for (i, label) in labels.iter().enumerate() {
                match piles.get(i).and_then(serde_json::Value::as_str) {
                    Some("A") => pile_1_ids.push(all_ids[i]),
                    Some("B") => {}
                    _ => unnamed.push(label),
                }
            }
            if !unnamed.is_empty() {
                self.log_rejected(&format!(
                    "'piles' gave no A or B for {} of {} permanents ({}); putting them in pile B",
                    unnamed.len(), labels.len(), unnamed.join(", ")));
            }
        } else {
            // Everything into pile B is a legal division, and it is also
            // what an unanswered prompt produces (#399).
            self.log_rejected(&format!(
                "no usable 'piles' list ({}); putting all {} permanents in pile B",
                response["piles"], all_ids.len()));
        }

        self.log("CHOSE", &format!("pile division: {} in pile A, {} in pile B",
            pile_1_ids.len(), all_ids.len() - pile_1_ids.len()));

        Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(pile_1_ids) }
    }

    /// Ask for a whole ordering of `rows` (issue #325): the response is the
    /// list of indices, each exactly once, first to last. A response that is
    /// not a permutation falls back to the order as listed, which is what
    /// the flat action list would have produced from the same seat.
    fn choose_ordering(&mut self, view: &GameView, description: &str, rows: &[String], rule: &str) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let n = rows.len();
        let action_text = Self::ordering_text(description, rows, rule, view.you);
        let prompt = self.build_prompt(view, &action_text);
        let schema = ordering_schema(n);
        let response = self.send_message_structured(&prompt, &schema);
        let order = match Self::parse_order_response(&response["order"], n) {
            Some(order) => order,
            None => {
                // Keeping the listed order is a legal answer, and it is what
                // a seat that said nothing gets (#399).
                self.log_rejected(&format!(
                    "order response was not a permutation of 0..{n}: {}; keeping the listed order",
                    response["order"]));
                (0..n).collect()
            }
        };
        self.log("CHOSE", &format!("order: {order:?}"));
        Action::ResolveChoice { choice: ResolvedChoice::ChosenOrder(order) }
    }

    /// An ordering question as the seat reads it. The rows and the
    /// description are engine text — a trigger's "triggered by: Rakish Heir
    /// (#39) dealt 2 combat damage to p1" — and `p0`/`p1` are never defined
    /// to the seat, so they are rewritten to "you"/"Opp" as every other
    /// row is (#465, #543; issue #834).
    fn ordering_text(description: &str, rows: &[String], rule: &str, you: mtg_engine::ids::PlayerId) -> String {
        let n = rows.len();
        let description = Self::generic_player_rewrite(description, you);
        let listed: String = rows.iter().enumerate()
            .map(|(i, r)| format!("  {i}: {}", Self::generic_player_rewrite(r, you)))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{description}\n{rule}\n\nEntries to order:\n{listed}\n\n\
             Respond with `order`: every index from 0 to {} exactly once, in the order you choose.",
            n.saturating_sub(1))
    }

    /// Divide combat damage: how much of what is left goes to one blocker
    /// (CR 510.1c-d). Answered as the amount, mapped back onto the
    /// prompt's `ChosenIndex` (index `i` is `min + i`).
    fn choose_damage_amount(&mut self, view: &GameView, description: &str, min: u32, max: u32,
                            options: &[String]) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let description = Self::generic_player_rewrite(description, view.you);
        let action_text = format!(
            "{description}\n\
             Each blocker must be assigned lethal damage before the next is assigned any, \
             but you may assign it more (CR 510.1c). Whatever you do not assign here goes on.\n\n\
             Respond with `amount`: an integer from {min} to {max}.");
        let prompt = self.build_prompt(view, &action_text);
        let schema = damage_amount_schema(min, max);
        let response = self.send_message_structured(&prompt, &schema);
        let amount = match Self::parse_damage_amount(&response["amount"], min, max) {
            Some(a) => a,
            None => {
                // Lethal is a legal answer, and it is the division the
                // engine makes when nobody is asked (#399).
                self.log_rejected(&format!(
                    "amount response was not an integer in {min}..={max}: {}; assigning lethal ({min})",
                    response["amount"]));
                min
            }
        };
        self.log("CHOSE", &format!("amount {amount}"));
        let index = (amount - min) as usize;
        Action::ResolveChoice {
            choice: ResolvedChoice::ChosenIndex(index, options.get(index).cloned().unwrap_or_default()),
        }
    }

    /// The `amount` of a damage-division response, if it is in range.
    fn parse_damage_amount(value: &serde_json::Value, min: u32, max: u32) -> Option<u32> {
        value.as_u64()
            .and_then(|a| u32::try_from(a).ok())
            .filter(|a| (min..=max).contains(a))
    }

    /// The `order` array of an ordering response as a permutation of `0..n`,
    /// or `None` when it is not one.
    fn parse_order_response(value: &serde_json::Value, n: usize) -> Option<Vec<usize>> {
        let arr = value.as_array()?;
        let order: Vec<usize> = arr.iter()
            .map(|v| v.as_u64().and_then(|x| usize::try_from(x).ok()))
            .collect::<Option<Vec<_>>>()?;
        let mut seen = vec![false; n];
        let ok = order.len() == n && order.iter().all(|&i| i < n && !std::mem::replace(&mut seen[i], true));
        ok.then_some(order)
    }

    /// Handle a `ChooseExileFromGraveyard` resolution prompt.

    /// Ask the seat to mark a subset of a numbered list, and get back the
    /// positions it marked.
    ///
    /// The answer is an array of indices under one fixed key, NOT one
    /// boolean per option keyed by the option's name. A tool schema's
    /// top-level property keys must match `^[a-zA-Z0-9_.-]{1,64}$`, and a
    /// card's display name — `Spectral Rider (#62)`, or `0: Grizzly Bears`
    /// — has spaces and parentheses in it. Keying by name gets the whole
    /// request rejected with a 400 before the model ever sees it, which is
    /// issue #398: a `cc` seat could not cast Skaab Goliath at all, six
    /// times in one game, because the question was never put to it.
    ///
    /// `choose_card_set` already answered this with an index array; this is
    /// the same answer for every other "mark some of these".
    /// The `Options:` block of a "mark some of these" prompt.
    ///
    /// The numbering happens here and only here. A caller that numbers its
    /// own labels gets it twice — `0: 0: Abbey Griffin` on every target
    /// prompt, in the one prompt whose unusable answers are the most
    /// expensive (issue #490).
    fn numbered_listing(labels: &[String]) -> String {
        let mut listing = String::new();
        for (i, label) in labels.iter().enumerate() {
            writeln!(listing, "{i}: {label}").unwrap();
        }
        listing
    }

    /// One label per target: what the thing is called, and nothing else.
    fn target_labels(view: &GameView, options: &[mtg_engine::actions::Target]) -> Vec<String> {
        use mtg_engine::actions::Target;
        options
            .iter()
            .map(|t| match t {
                Target::Object(id) => Self::obj_name(view, *id),
                Target::Player(pid) => {
                    if *pid == view.you { "You".to_string() } else { "Opponent".to_string() }
                }
                Target::Illegal => "(illegal)".to_string(),
            })
            .collect()
    }

    /// How many to pick, in words. Also the `indices` description in the
    /// schema, so the prompt and the schema cannot say different numbers.
    fn marked_count_note(min: usize, max: usize, noun: &str) -> String {
        if min == max {
            format!("Pick exactly {min} {noun}{}.", if min == 1 { "" } else { "s" })
        } else {
            format!("Pick anywhere from {min} to {max} {noun}s.")
        }
    }

    /// The body of a "choose some of these" prompt. GAME_RULES quotes this
    /// shape and a test builds the documented example through it, because
    /// the const said this prompt answered with a boolean per card long
    /// after it became an index array (#492).
    fn marked_list_body(
        labels: &[String],
        description: &str,
        count_note: &str,
        instruction: &str,
    ) -> String {
        let listing = Self::numbered_listing(labels);
        format!("{description}\n\n{count_note} {instruction}\n\nOptions:\n{listing}")
    }

    /// Mark a subset of a numbered list. `cancel` names what the prompt can
    /// back out of ("the cast"), where the engine accepts `CancelCast`;
    /// `None` is returned when the seat chose to (#749).
    #[allow(clippy::too_many_arguments)]
    fn mark_indices(
        &mut self,
        view: &GameView,
        labels: &[String],
        min: usize,
        max: usize,
        description: &str,
        instruction: &str,
        noun: &str,
        cancel: Option<&str>,
    ) -> Option<Vec<usize>> {
        let count_note = Self::marked_count_note(min, max, noun);
        let action_text = Self::marked_list_body(labels, description, &count_note, instruction);
        let prompt = self.build_prompt(view, &action_text);

        let valid: Vec<usize> = (0..labels.len()).collect();
        let mut schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {
                    "type": "string",
                    "description": THOUGHTS_DESCRIPTION,
                },
                "indices": {
                    "type": "array",
                    "items": {"type": "integer", "enum": valid},
                    "minItems": min,
                    "maxItems": max,
                    // The count is in the prompt (`count_note`), not here:
                    // the schema is part of the cached prefix.
                    "description": "The 0-indexed positions to pick, from the numbered list, \
                                    as many as the prompt says.",
                },
            },
            "required": ["thoughts", "indices"],
            "additionalProperties": false,
        });
        if cancel.is_some() {
            Self::offer_cancel(&mut schema);
        }

        let response = self.send_message_structured(&prompt, &schema);
        if cancel.is_some() && self.chose_cancel(&response, description) {
            return None;
        }
        let answered = response["indices"].is_array();
        let mut chosen: Vec<usize> = Vec::new();
        if let Some(arr) = response["indices"].as_array() {
            for v in arr {
                let Some(i) = v.as_u64().and_then(|i| usize::try_from(i).ok()) else { continue };
                // A repeat or an out-of-range index is not a second choice;
                // both are answers the engine would refuse for a reason the
                // seat did not intend.
                if i < labels.len() && !chosen.contains(&i) {
                    chosen.push(i);
                }
            }
        }
        // An empty set is a real answer when none was required; it is a
        // SUBSTITUTED one when the seat gave no usable array or too few
        // indices, and the two used to be the same line in the log (#399).
        if !answered || chosen.len() < min {
            self.log_rejected(&format!(
                "no usable 'indices' for {description} ({}); marking {} of {} \
                 instead of the {min} asked for",
                response["indices"], chosen.len(), labels.len()));
        }
        Some(chosen)
    }

    /// Add the `cancel` field to a prompt's schema: the CLI's "c = cancel
    /// the cast", the page's Cancel button and the random seat's
    /// `CANCEL_CHANCE`, which this seat alone did not have — an X spell it
    /// could no longer afford was cast for X = 0 (#749).
    ///
    /// One description for a cast and an activation alike: the prompt says
    /// which it is, and the schema is part of the cached prefix
    /// (`cache_stable_schema`).
    fn offer_cancel(schema: &mut serde_json::Value) {
        schema["properties"]["cancel"] = serde_json::json!({
            "type": "boolean",
            "description": "true to cancel the cast or activation this question belongs to: it is undone and nothing is paid. false to answer.",
        });
        if let Some(required) = schema["required"].as_array_mut() {
            required.push(serde_json::json!("cancel"));
        }
    }

    /// Whether the seat answered `cancel: true`, logged when it did.
    fn chose_cancel(&mut self, response: &serde_json::Value, description: &str) -> bool {
        let cancel = response["cancel"].as_bool() == Some(true);
        if cancel {
            self.log("CHOSE", &format!("cancel ({description})"));
        }
        cancel
    }

    ///
    /// The engine surfaces eligible graveyard cards (filtered per the
    /// spell's additional cost: creatures only for Stitched Drake et al.,
    /// all cards for Harvest Pyre) plus a count range `[min, max]`.
    /// For variable count (Harvest Pyre: `min=0, max=gy_size`) any
    /// response is legal. For fixed count (Stitched Drake: `min=max=1`)
    /// the count must match exactly — the engine validates and cancels
    /// the cast if it doesn't.
    ///
    /// Pick the targets for an "up to N" slot (CR 601.2c). The answer is an
    /// index array under one fixed key — see `mark_indices`, which also
    /// numbers the list.
    fn choose_target_set(
        &mut self,
        view: &GameView,
        options: &[mtg_engine::actions::Target],
        min: usize,
        max: usize,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::{ResolvedChoice, Target};
        if options.is_empty() || max == 0 {
            return Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(vec![]) };
        }

        // No index prefix on the label: it used to *be* the schema key,
        // where two copies of one card needed distinguishing, and the schema
        // is an index array now — so `mark_indices`, which numbers the list
        // itself, printed `0: 0: Abbey Griffin` on every target prompt
        // (issue #490).
        let labels = Self::target_labels(view, options);
        let Some(picked) = self.mark_indices(
            view, &labels, min, max, description,
            "Name the targets you want; leaving a slot empty is allowed where the count says so.",
            "target", Some("the cast")) else {
            return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
        };
        let mut chosen: Vec<Target> = picked.into_iter()
            .filter_map(|i| options.get(i).cloned())
            .collect();
        // Too many is a cast the engine would cancel, so trim to what the
        // slot holds rather than throwing the cast away; too few is only
        // possible where the slot demands more than the seat marked, and
        // there the engine's refusal is the right answer.
        if chosen.len() > max {
            self.log("VALIDATION", &format!(
                "target-set: chose {} of at most {max}; keeping the first {max}", chosen.len()));
            chosen.truncate(max);
        }
        self.log("CHOSE", &format!("{} target(s)", chosen.len()));
        Action::ResolveChoice { choice: ResolvedChoice::ChosenTargetSet(chosen) }
    }

    /// Mark a subset of objects: an index array under one fixed key, which
    /// is the shape every "choose some of these" question takes for this
    /// seat (`mark_indices`, #398).
    ///
    /// `verb` is what picking an index does — "exile", "choose" — and
    /// completes the instruction line "Name the cards to {verb}.", so it is
    /// a bare verb phrase and not a whole clause: "exile this card" made
    /// that read "Name the cards to exile this card." (#492).
    fn choose_object_subset(
        &mut self,
        view: &GameView,
        options: &[mtg_engine::ids::ObjectId],
        min: usize,
        max: usize,
        description: &str,
        verb: &str,
        cancel: Option<&str>,
    ) -> Option<Vec<mtg_engine::ids::ObjectId>> {
        if options.is_empty() {
            return Some(vec![]);
        }

        let labels = Self::format_combat_creature_list(view, options);
        let instruction = format!("Name the cards to {verb}.");
        let picked = self.mark_indices(view, &labels, min, max, description, &instruction, "card", cancel)?;
        Some(picked.into_iter().filter_map(|i| options.get(i).copied()).collect())
    }

    /// Exile-from-graveyard additional cost: mark the cards to exile.
    fn choose_exile_from_graveyard(
        &mut self,
        view: &GameView,
        options: &[mtg_engine::ids::ObjectId],
        min: usize,
        max: usize,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let Some(chosen) = self.choose_object_subset(
            view, options, min, max, description, "exile", Some("the cast")) else {
            return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
        };

        // For fixed-count costs, the engine validates and cancels on
        // mismatch. We log here so the diagnostic trail is clear.
        if min == max && chosen.len() != min {
            self.log("VALIDATION", &format!(
                "exile-choice: chose {} but required exactly {min}; engine will cancel cast",
                chosen.len()
            ));
        } else {
            self.log("CHOSE", &format!("exile {} from graveyard", chosen.len()));
        }

        Action::ResolveChoice { choice: ResolvedChoice::ChosenExileSet(chosen) }
    }

    /// A set of objects chosen while an effect resolves — Curse of
    /// Oblivion's two cards out of a graveyard.
    ///
    /// Unlike the exile cost above there is no cast to cancel: a wrong count
    /// leaves the question unanswered and the effect waiting, so the
    /// shortfall is filled in from the options rather than sent as-is.
    fn choose_object_set(
        &mut self,
        view: &GameView,
        options: &[mtg_engine::ids::ObjectId],
        min: usize,
        max: usize,
        description: &str,
        verb: &str,
        cancel: Option<&str>,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        let Some(mut chosen) = self.choose_object_subset(
            view, options, min, max, description, verb, cancel) else {
            return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
        };
        chosen.truncate(max);
        if chosen.len() < min {
            self.log("VALIDATION", &format!(
                "object-set: chose {} but required at least {min}; filling from the options",
                chosen.len()));
            for id in options {
                if chosen.len() >= min {
                    break;
                }
                if !chosen.contains(id) {
                    chosen.push(*id);
                }
            }
        }
        self.log("CHOSE", &format!("{} object(s)", chosen.len()));
        Action::ResolveChoice { choice: ResolvedChoice::ChosenObjectSet(chosen) }
    }

    /// Handle a `ChooseXFunding` resolution prompt.
    ///
    /// Builds a dynamic JSON schema from the engine-provided options (pool
    /// mana per color + tap groups per category) and asks the model to
    /// allocate amounts. The sum of allocations becomes X.
    fn choose_x_funding(
        &mut self,
        view: &GameView,
        options: &mtg_engine::funding::FundingOptions,
        _source_id: mtg_engine::ids::ObjectId,
        description: &str,
    ) -> Action {
        use mtg_engine::actions::ResolvedChoice;
        use mtg_engine::funding::{FundingCategory, FundingResponse};
        use mtg_engine::types::ManaType;

        // The schema is the same for every X spell (#857): `floating` has one
        // field per color whatever the pool holds, and `sources` is one
        // amount per source group in the order the prompt lists them. What
        // changes from call to call — which groups are untapped, how many,
        // how much each makes, what is floating — is in the prompt and in
        // the per-call constraints (`enum`, `prefixItems`, the length) that
        // `cache_stable_schema` leaves out of the request and
        // `schema_violation` holds the answer to. The groups used to be
        // property names and their counts descriptions, so nearly every X
        // spell sent a schema of its own and wrote the cached prefix again.
        let mt_key = |mt: ManaType| -> &'static str {
            match mt {
                ManaType::White => "white",
                ManaType::Blue => "blue",
                ManaType::Black => "black",
                ManaType::Red => "red",
                ManaType::Green => "green",
                ManaType::Colorless => "colorless",
            }
        };

        // Integer-range fields use string-enum so the schema is portable across
        // providers: Anthropic rejects `minimum`/`maximum` on integer fields,
        // Gemini rejects `enum` on integer fields, and only `enum` on string
        // fields is both accepted and enforced by both. Response is parsed
        // back to u32 below. The rule is now enforced rather than only
        // stated — `AnthropicBackend::sanitize_schema` on one side and
        // `sanitize_schema_for_gemini` on the other (#546) — so a schema that
        // does not use this workaround is at worst bounded by a range on
        // Gemini rather than rejected. A set constraint is still tighter than
        // a range, which is why this one keeps it.
        let int_enum_str = |legal_values: Vec<u32>| -> serde_json::Value {
            let enum_vals: Vec<serde_json::Value> = legal_values.into_iter()
                .map(|n| serde_json::json!(n.to_string()))
                .collect();
            serde_json::json!({"type": "string", "enum": enum_vals})
        };

        // Floating: per-color legal values are 0..=available.
        let mut floating_props = serde_json::Map::new();
        let colors = [
            ManaType::White, ManaType::Blue, ManaType::Black,
            ManaType::Red, ManaType::Green, ManaType::Colorless,
        ];
        let mut floating_required = Vec::new();
        let mut pool_lines = Vec::new();
        for mt in colors {
            let available = options.pool.get(&mt).copied().unwrap_or(0);
            let mut amounts = int_enum_str((0..=available).collect());
            if available == 0 {
                // Nothing of this colour floats, so its amount can only be
                // 0: an answer may leave it out, and the format line does
                // not ask for it (#930). The field stays in the schema.
                amounts[ONLY_VALUE] = serde_json::json!(true);
            }
            floating_props.insert(mt_key(mt).to_string(), amounts);
            floating_required.push(mt_key(mt).to_string());
            if available > 0 {
                pool_lines.push(format!("  {}: 0 to {available}", mt_key(mt)));
            }
        }

        // Legal allocations are 0, mana_per_tap, 2*mana_per_tap, ...,
        // count*mana_per_tap. For 1-mana sources this degenerates to a simple
        // 0..=count range; for multi-mana sources (Sol Ring) the enum excludes
        // fractional activations.
        let mut source_rows = Vec::new();
        let mut source_slots = Vec::new();
        for (i, g) in options.groups.iter().enumerate() {
            let legal: Vec<u32> = (0..=g.source_ids.len())
                .map(|n| u32::try_from(n).unwrap_or(u32::MAX).saturating_mul(g.mana_per_tap))
                .collect();
            let colors_str: Vec<String> = g.colors_produced.iter().map(|c| format!("{c:?}")).collect();
            let color_hint = if colors_str.is_empty() {
                String::new()
            } else {
                format!(" (produces {{{}}})", colors_str.join(","))
            };
            let amounts = if g.mana_per_tap == 1 {
                format!("0 to {}", g.max_contribution())
            } else {
                let shown: Vec<String> = legal.iter().map(ToString::to_string).collect();
                format!("one of {}", shown.join(", "))
            };
            let dork = if g.category == FundingCategory::Dorks {
                "; a creature tapped for this can't attack or block this turn"
            } else {
                ""
            };
            source_rows.push(format!(
                "  {i}: {}{color_hint}, {} untapped, {} mana each: {amounts}{dork}",
                g.name, g.source_ids.len(), g.mana_per_tap));
            source_slots.push(int_enum_str(legal));
        }
        let n_sources = source_slots.len();

        let mut schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "floating": {
                    "type": "object",
                    "properties": floating_props,
                    "required": floating_required,
                    "additionalProperties": false,
                    "description": "Mana to drain from your pool, per color"
                },
                "sources": {
                    "type": "array",
                    "items": {"type": "string"},
                    "prefixItems": source_slots,
                    "minItems": n_sources,
                    "maxItems": n_sources,
                    "description": "Mana to take from each listed source, one entry per source in the order listed"
                }
            },
            "required": ["thoughts", "floating", "sources"],
            "additionalProperties": false,
        });

        let discount_note = if options.x_discount > 0 {
            format!(
                " A cost reduction already pays for {} of X, so X = {} + everything you allocate, \
                 up to {}.",
                options.x_discount, options.x_discount, options.max_announceable_x(),
            )
        } else {
            String::new()
        };
        let sources_text = if source_rows.is_empty() {
            "Sources: none untapped (`sources` is [])".to_string()
        } else {
            let plural = if n_sources == 1 { "" } else { "s" };
            format!("Sources (`sources` is {n_sources} amount{plural}, one per row, in this order):\n{}",
                source_rows.join("\n"))
        };
        let pool_text = if pool_lines.is_empty() {
            String::new()
        } else {
            format!("\nFloating in your pool (`floating`, per color):\n{}", pool_lines.join("\n"))
        };
        let prompt_text = format!(
            "{description}\n\
             X = sum of all allocated amounts. Legal X values: 0 to {}.{discount_note}\n\
             {sources_text}{pool_text}\n\
             Sources with variable output or cost-bearing activation (e.g. pain lands) aren't listed — tap those manually first to float the mana.",
            options.max_announceable_x(),
        );
        Self::offer_cancel(&mut schema);
        let full_prompt = self.build_prompt(view, &prompt_text);
        let response = self.send_message_structured(&full_prompt, &schema);
        if self.chose_cancel(&response, description) {
            return Action::ResolveChoice { choice: ResolvedChoice::CancelCast };
        }

        // Parse response. The schema uses string-enum for integer-range
        // fields (see `int_enum_str` above), so values arrive as strings
        // like "2" and we parse back to u32.
        //
        // A JSON *number* carrying the same value is the same allocation and
        // is taken as one: `as_str()` alone is `None` for it, so
        // `{"rocks":{"Sol Ring":4}}` (the shape then) read as 0, every group read as 0, the
        // response became `FundingResponse::default()` — which `validate`
        // accepts, because an empty response is a legal X = 0 — and the seat
        // logged `X funding sum = 0`, byte-identical to a model that chose
        // zero. A seat cast every X spell for nothing and the counter built
        // to surface exactly that read clean (#596). The other readers in
        // this file take `as_u64()`; this one was the odd one out.
        //
        // What is still not an allocation — "two", "", "0x2", "1e1", 2.5,
        // -1, an object — is substituted with 0 *and counted*, the way every
        // other client-side validator here does it (H14). Substituting is
        // right; substituting silently is what made this invisible.
        let parse_int_str = |v: &serde_json::Value| -> Option<u32> {
            if let Some(s) = v.as_str() {
                return s.parse::<u32>().ok();
            }
            v.as_u64().and_then(|n| u32::try_from(n).ok())
        };
        let mut funding = FundingResponse::default();
        let mut unreadable: Vec<String> = Vec::new();
        for mt in colors {
            let Some(val) = response.get("floating").and_then(|f| f.get(mt_key(mt))) else {
                continue;
            };
            match parse_int_str(val) {
                Some(amount) => {
                    if amount > 0 {
                        funding.pool.insert(mt, amount);
                    }
                }
                None => unreadable.push(format!("floating.{} = {val}", mt_key(mt))),
            }
        }
        let answered = response.get("sources").and_then(serde_json::Value::as_array);
        for (i, g) in options.groups.iter().enumerate() {
            let Some(val) = answered.and_then(|a| a.get(i)) else {
                continue;
            };
            match parse_int_str(val) {
                Some(amount) => {
                    if amount > 0 {
                        funding.taps.insert(g.name.clone(), amount);
                    }
                }
                None => unreadable.push(format!("sources[{i}] ({}) = {val}", g.name)),
            }
        }
        if !unreadable.is_empty() {
            self.log_rejected(&format!(
                "X funding allocations the harness could not read ({}), taken as 0",
                unreadable.join(", ")));
        }

        // Best-effort validation: if the model produced something invalid,
        // clamp to empty (X = 0) so the cast still completes rather than
        // crashing. Log the issue for investigation.
        if let Err(e) = mtg_engine::funding::validate(&funding, options) {
            self.log_rejected(&format!("invalid X funding response ({e}), defaulting to X=0"));
            funding = FundingResponse::default();
        }
        self.log("CHOSE", &format!("X funding sum = {}", funding.x_value()));
        Action::ResolveChoice { choice: ResolvedChoice::XFunding(funding) }
    }

    /// Confirm concede via structured output. Returns true if the AI confirms.
    ///
    /// Asked through `build_prompt` (#831): since every decision is its own
    /// call, the bare question was all the seat saw — no board, and not the
    /// reasoning that had just chosen Concede, which the notes block carries.
    fn confirm_concede(&mut self, view: &GameView) -> bool {
        self.log("CONCEDE-CHECK", "AI chose Concede, confirming...");

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "confirm": {"type": "boolean", "description": "true to concede, false to cancel"}
            },
            "required": ["thoughts", "confirm"]
        });

        let prompt = self.build_prompt(view,
            "[CONFIRM CONCEDE]\nYou chose to CONCEDE the game. Are you sure? Before you confirm, count it out: the \
             opponent's damage next turn through your best blocks (a chump stops a creature without trample), your \
             life, and the turns until either library runs out. If any line keeps you alive, cancel. Confirm true to \
             concede, false to cancel.");
        let response = self.send_message_structured(&prompt, &schema);
        let confirmed = match response["confirm"].as_bool() {
            Some(c) => c,
            None => {
                // Cancelling is what a seat that changed its mind answers,
                // and it was also what a seat that said nothing got (#399).
                self.log_rejected(&format!(
                    "no usable 'confirm' bool ({}); cancelling the concede",
                    response["confirm"]));
                false
            }
        };
        if confirmed {
            self.log("CONCEDE-CHECK", "Concede confirmed");
        } else {
            self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
        }
        confirmed
    }

    /// Build a JSON schema that constrains a single integer "action" field
    /// to one of `0..count`. The model can only return a valid index.
    /// Used for action-pick and target-pick prompts.
    fn enum_action_schema(count: usize, key: &str, description: &str) -> serde_json::Value {
        let valid: Vec<serde_json::Value> = (0..count).map(|i| serde_json::json!(i)).collect();
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                key: {
                    "type": "integer",
                    "enum": valid,
                    "description": description,
                }
            },
            "required": ["thoughts", key]
        })
    }

    /// Pick an action index from a bounded set using structured output.
    /// The schema constrains the response to a valid integer in `0..max`,
    /// so the model cannot return an out-of-range index. Falls back to 0
    /// only if the response is somehow missing the field entirely.
    /// If the chosen action is Concede, runs the confirmation dialog
    /// before returning.
    /// Ask for one index into a list of options the caller has shown,
    /// against the same board every other in-game decision is made against.
    ///
    /// The returned index belongs to whatever list the prompt displayed —
    /// this function has no idea what the options mean. It used to also
    /// screen the choice for Concede against `legal_actions`, which is a
    /// different list whenever the display collapsed duplicate casts or
    /// abilities: Concede's display index was then smaller than its legal
    /// index, `actions.get(idx)` was some unrelated action, and the
    /// confirmation silently did not happen (issue #209). The guard now
    /// lives with the caller that knows which action an index means.
    fn pick_action_index(&mut self, view: &GameView, action_text: &str, max: usize) -> usize {
        self.pick_action_index_described(view, action_text, max, "Index of the chosen action")
    }

    /// [`pick_action_index`](Self::pick_action_index) with the schema's
    /// description of the index spelled out by the caller — the priority
    /// menu uses it to say which index is the pass-until row.
    fn pick_action_index_described(
        &mut self, view: &GameView, action_text: &str, max: usize, description: &str,
    ) -> usize {
        let mut schema = Self::enum_action_schema(max, "action", description);
        self.add_turn_fields(&mut schema, &[], &[]);
        self.pick_action_index_with_schema(view, action_text, max, &schema).0
    }

    /// [`pick_action_index`](Self::pick_action_index) against a schema the
    /// caller built (an `action` index and whatever else it asks), handing
    /// back the whole response with the index.
    fn pick_action_index_with_schema(
        &mut self, view: &GameView, action_text: &str, max: usize, schema: &serde_json::Value,
    ) -> (usize, serde_json::Value) {
        assert!(max > 0, "pick_action_index requires at least one option");
        // The state body is built HERE rather than by the caller, so that no
        // caller can ask for an index without it. Four of the five callers
        // used to hand over a bare `format!` string: the seat chose a target
        // from 48 characters with no turn, no life totals, no boards and no
        // stack, which is what #463 fixed for the cleanup discard and #491
        // found still true of "select a target", the two sacrifice prompts
        // and the ability-target prompt. The CLI renders the whole board at
        // all four (`run_target_chooser`, #122).
        let prompt = self.build_prompt(view, action_text);
        let response = self.send_message_structured(&prompt, schema);
        let idx = response["action"].as_u64().map(|n| usize::try_from(n).unwrap_or(usize::MAX))
            .filter(|n| *n < max)
            .unwrap_or_else(|| {
                self.log_rejected(&format!("response missing valid 'action' field ({response}), defaulting to 0"));
                0
            });
        self.log("CHOSE", &format!("action {idx}"));
        (idx, response)
    }
}

impl Player for LlmPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn gave_up(&self) -> Option<String> {
        self.backend.gave_up()
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let action = self.decide(view, legal, true);
        if std::mem::take(&mut self.plan_done_pending) && self.pass_until.is_none() {
            self.pass_until = Some(PassUntil::done_at(view));
        }
        // The save written before the next decision reads this.
        self.sync_memory_mirror();
        action
    }
}

impl LlmPlayer {
    /// [`Player::choose_action`]'s body: one decision, by whichever prompt
    /// fits it. `may_reask`: a plan started by a pass that stops at once
    /// asks the seat again, once (#860).
    fn decide(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions, may_reask: bool) -> Action {
        let legal_actions = &legal.actions;
        // A prompt that is not a plain priority pass — a target, a set, X,
        // a "may pay" — is asked with the full prompt, as it would be
        // anyway. It ends a pass-until or a plan (and the attack named with
        // it) only when the opponent has put something on the stack since
        // they began: one the seat's own planned cast or its own trigger
        // raises is part of the plan, and dropping the plan there asked the
        // same attack again at combat (#862).
        // The engine labels the other seat `p1`, and the context line is the
        // one line of the prompt that used to pass that through: every log
        // entry is rewritten to you/opp, and the line that names the decision
        // — `[RESPOND TO p1's ...]` — was not, so the prompt used a vocabulary
        // its own system prompt says it never uses (issue #465). The stop
        // lines below quote it too, into the next prompt (#834's review).
        let context: Option<String> = legal.context.as_deref()
            .map(|c| Self::generic_player_rewrite(c, view.you));
        if !Self::is_plain_priority_offer(legal) {
            let what = || context.clone()
                .unwrap_or_else(|| "a prompt that is not a priority pass".to_string());
            let opp_acted = |since: u64| view.opponent_stack_puts > since;
            if self.pass_until.as_ref().is_some_and(|m| opp_acted(m.opp_puts)) {
                self.stop_pass_until(&format!("{OPPONENT_ACTED}, and you are asked something else: {}", what()));
            }
            if self.plan.as_ref().is_some_and(|p| opp_acted(p.opp_puts)) {
                self.stop_plan(&format!("{OPPONENT_ACTED}, and you are asked something else: {}", what()));
            }
        }
        // X-cost funding: structured-prompt choice that can't be pre-enumerated.
        // The engine surfaces the `FundingOptions` via `resolution_prompt`; we
        // build a dynamic JSON schema + explanation and parse the response.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding {
            options, source_id, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return self.choose_x_funding(view, options, *source_id, description);
        }

        // An "up to N" target slot: an index array, the same shape as
        // the exile cost below. The engine stopped enumerating one cast per
        // subset (issue #360), which for Memory's Journey over a
        // fifteen-card graveyard was about 1,150 rows of menu.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseTargetSet {
            options, min, max, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            let (options, min, max, description) =
                (options.clone(), *min, *max, description.clone());
            return self.choose_target_set(view, &options, min, max, &description);
        }

        // A set of objects chosen while an effect resolves: the same
        // index-array shape. Curse of Oblivion used to ask twice.
        // The same shape answers "tap two untapped creatures you control",
        // an activation's cost (#670): the verb says which it is.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseObjectSet {
            options, min, max, description, effect,
        }) = legal.resolution_prompt.as_ref()
        {
            // Only the cost can be backed out of; an effect resolving has
            // no cast to cancel.
            let (verb, cancel) = if matches!(effect, mtg_engine::state::PendingEffect::PayActivationTaps { .. }) {
                ("tap to pay the ability's cost", Some("the activation"))
            } else {
                ("choose", None)
            };
            let (options, min, max, description) =
                (options.clone(), *min, *max, description.clone());
            return self.choose_object_set(view, &options, min, max, &description, verb, cancel);
        }

        // Exile-from-graveyard additional cost: which cards to exile.
        // The engine surfaces eligible graveyard cards via `resolution_prompt`;
        // the answer is an index array under `indices` (`mark_indices`), not
        // a boolean per card -- a card name is not a legal top-level schema
        // key (#398).
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseExileFromGraveyard {
            options, min, max, description, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return self.choose_exile_from_graveyard(view, options, *min, *max, description);
        }

        // London mulligan keep/mull decision.
        if legal_actions.iter().any(|a| matches!(a, Action::MulliganKeep)) {
            return self.choose_mulligan(view, legal_actions);
        }
        // A set of cards out of a list — the mulligan bottoming and the
        // cleanup discard. The engine stopped enumerating the C(hand, n)
        // subsets (issue #360), so the seat picks indices out of its hand
        // rather than one row out of a list of every way of picking.
        if let Some(prompt) = legal.set_prompt.as_ref() {
            let prompt = prompt.clone();
            return self.choose_card_set(view, &prompt);
        }

        // Pile division (e.g. Liliana of the Veil -6): structured prompt —
        // the engine no longer enumerates the 2^N subsets (issue #142).
        // Ask for a pile per permanent and build the subset directly.
        if let Some(mtg_engine::state::ResolutionChoiceKind::DividePermanentsIntoPiles {
            permanents, description, target_player, ..
        }) = legal.resolution_prompt.as_ref()
        {
            let (permanents, description, target_player) =
                (permanents.clone(), description.clone(), *target_player);
            return self.choose_pile_division(view, &permanents, &description, target_player);
        }

        // An ordering is one decision (issue #325): the seat lists every
        // index once, first to last, instead of answering one prompt per
        // place — twelve simultaneous triggers were twelve round trips.
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseTriggerOrder {
            description, options, details, ..
        }) = legal.resolution_prompt.as_ref()
        {
            let rows: Vec<String> = options.iter().enumerate().map(|(k, o)| match details.get(k) {
                Some(d) => {
                    let pt = d.power_toughness.map(|(p, t)| format!(" {p}/{t}")).unwrap_or_default();
                    let what = if d.ability.is_empty() { d.kind.clone() } else { format!("{}: {}", d.kind, d.ability) };
                    format!("{} (#{}){pt} — {what} — triggered by: {}", d.source_name, d.source.0, d.cause)
                }
                None => o.clone(),
            }).collect();
            return self.choose_ordering(view, description, &rows,
                "The first index you list goes on the stack FIRST and so resolves LAST; the last you list resolves FIRST (CR 603.3b). \
                 Put the trigger you want to resolve first at the END of the list.");
        }
        if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseDamageAssignmentOrder {
            description, options, ..
        }) = legal.resolution_prompt.as_ref()
        {
            return self.choose_ordering(view, description, options,
                "The first index you list is assigned damage FIRST and must be assigned lethal damage before the next gets any (CR 510.1c). \
                 Put the blocker you most want dead first.");
        }

        // How much of an attacker's damage goes to one blocker (CR
        // 510.1c-d): one integer, asked as the amount it is (#637).
        if let Some(mtg_engine::state::ResolutionChoiceKind::AssignCombatDamage {
            description, min, max, options, ..
        }) = legal.resolution_prompt.as_ref()
        {
            let (description, min, max, options) = (description.clone(), *min, *max, options.clone());
            return self.choose_damage_amount(view, &description, min, max, &options);
        }

        // Nobody answers their own spell: while everything on the stack is
        // the seat's, the offer is to respond to itself, and a person who
        // just cast a creature does not deliberate over letting it resolve.
        // Each of these was a model call — about one per spell the seat
        // cast — that came back "pass" (reports/llm-cost.md, phase 3).
        if self.offer_pass_until && Self::is_plain_priority_offer(legal)
            && !view.stack.is_empty() && view.stack.iter().all(|s| s.controller == view.you)
        {
            // The view's stack is top first (#870).
            let top = view.stack.first().map_or_else(String::new, |s| s.name.clone());
            self.log("AUTO_PASS", &format!("only your own spells and abilities are on the stack ({top} on top): passing so they resolve"));
            return Action::PassPriority;
        }

        // The rest of a main phase the seat laid out: its next card, or
        // the end of the plan — which passes until something happens — or
        // a stop that asks it. Only at a priority offer: any other prompt
        // that reaches here (a "may pay", an ETB trigger's target, a yes or
        // no) is asked, and the plan waits it out (#862).
        let plain_offer = Self::is_plain_priority_offer(legal);
        if let Some(plan) = self.plan.clone().filter(|_| plain_offer) {
            match self.plan_next(view, legal, &plan) {
                PlanMove::Act(action) => return action,
                PlanMove::Done => {
                    self.plan = None;
                    self.log("PLAN_DONE", &format!("{} planned play{} made; passing until something happens",
                        plan.taken, if plan.taken == 1 { "" } else { "s" }));
                    if self.pass_until.is_none() {
                        self.pass_until = Some(PassUntil::engaged_at(view));
                    }
                }
                PlanMove::Stop(why) => self.stop_plan(&why),
            }
        }

        // Auto-pass when there's nothing interesting to do. Logged at
        // debug level — it can fire many steps in a row. After the plan, so
        // a plan whose next card cannot be cast says so and ends here.
        if Self::should_auto_pass(view, legal_actions) {
            self.log_debug("AUTO-PASS", &format!("Step: {:?}, active: p#{}", view.step, view.active_player.0));
            return Action::PassPriority;
        }

        // The seat said "go" at an earlier offer: pass this one unasked
        // unless something it would want to see has happened since.
        if let Some(mode) = self.pass_until.clone().filter(|_| plain_offer) {
            if let Some(why) = Self::pass_until_stop(view, legal, &mode) {
                self.stop_pass_until(&why);
            } else {
                let passes = mode.passes + 1;
                self.log("AUTO_PASS", &format!(
                    "until: {} — turn {} {} ({}), {}; passing since {} ({passes} offer{} passed unasked)",
                    Self::pass_until_still_passing(view, &mode),
                    view.turn_number, Self::step_name(view.step, view.first_strike_damage_step),
                    if view.active_player == view.you { "your turn" } else { "opp's turn" },
                    if view.stack.is_empty() { "stack empty" } else { "stack unchanged" },
                    mode.since(), if passes == 1 { "" } else { "s" }));
                self.pass_until = Some(PassUntil { passes, ..mode });
                return Action::PassPriority;
            }
        }

        let (menu, rows) = Self::priority_menu(view, legal, self.offer_pass_until);

        let mut action_prompt = Self::format_action_prompt(context.as_deref(), &rows, view.you);
        // The one prompt whose rule the board does not make plain is told it
        // here, where it is asked, rather than in every system prompt.
        if matches!(legal.resolution_prompt, Some(mtg_engine::state::ResolutionChoiceKind::ChooseDamageEffect { .. })) {
            action_prompt = format!("{DAMAGE_EFFECT_RULE}\n{action_prompt}");
        }

        let engine_entries = menu.iter().filter(|e| matches!(e, MenuEntry::Engine(_))).count();
        if engine_entries != legal_actions.len() {
            self.log_debug("COLLAPSED", &format!("{} actions → {} options", legal_actions.len(), engine_entries));
        }
        // The one description every index question has: the schema is part
        // of the cached prefix (`cache_stable_schema`), so a menu that
        // described its own rows would be a schema of its own. The row
        // names itself, and the system prompt says what ends it.
        let description = "Index of the chosen action";
        let plannable = self.plannable(view, legal);
        let ready = self.ready_attackers(view, legal);
        let mut schema = Self::enum_action_schema(menu.len(), "action", description);
        self.add_turn_fields(&mut schema, &plannable, &ready);
        let (idx, response) = self.pick_action_index_with_schema(view, &action_prompt, menu.len(), &schema);
        let then: Vec<String> = response["then"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        // Where no plan was offered there is no field, and the seat is asked
        // at the next offer as it always was.
        let ask_again = response["ask_again"].as_bool().unwrap_or(true);
        if !ready.is_empty() {
            let attackers: Vec<ObjectId> = response["attack"].as_array().into_iter().flatten()
                .filter_map(serde_json::Value::as_u64)
                .map(ObjectId)
                .filter(|o| ready.contains(o))
                .collect();
            self.attack_plan = (!attackers.is_empty()).then(|| {
                self.log("ATTACK_PLAN", &format!("attack with {}", Self::creature_names(view, &attackers)));
                AttackPlan { turn: view.turn_number, attackers, opp_puts: view.opponent_stack_puts }
            });
        }

        if idx >= menu.len() {
            return Action::PassPriority;
        }
        // A pass with cards planned is read as the plan starting now: in a
        // main phase with an empty stack a pass ends the phase, so there is
        // no "after" for the plan to wait for. A model answering
        // `{"action": 0, "then": ["Swamp"]}` meant "play the Swamp" — read
        // as a pass it skipped its land drop, which a replay of real
        // positions caught on one decision in six (reports/llm-cost.md,
        // phase 3).
        let passes = match menu[idx] {
            MenuEntry::PassUntil => true,
            MenuEntry::Engine(DisplayEntry::Direct(i)) => matches!(legal_actions[i], Action::PassPriority),
            MenuEntry::Engine(_) => false,
        };
        if passes && !plannable.is_empty() && !then.is_empty() {
            self.log("PLAN", &format!("then: {} (answered with a pass: the plan starts now)", then.join(", ")));
            let plan = Plan { turn: view.turn_number, step: view.step, steps: then.clone(), taken: 0, ask_again,
                opp_puts: view.opponent_stack_puts };
            self.plan = Some(plan.clone());
            match self.plan_next(view, legal, &plan) {
                PlanMove::Act(action) => return action,
                PlanMove::Done => self.plan = None,
                // The seat is asked, as a plan that stops after a play asks
                // it at the next offer: carrying out the pass instead ended
                // the main phase with the rest of the plan — a land after
                // the spell it was to pay for — never played, and nobody
                // asked (#860). Asked once: a second answer that stops the
                // same way passes.
                PlanMove::Stop(why) => {
                    self.stop_plan(&why);
                    if may_reask {
                        return self.decide(view, legal, false);
                    }
                }
            }
        }
        let entry = match menu[idx] {
            MenuEntry::PassUntil => {
                self.engage_pass_until(view);
                return Action::PassPriority;
            }
            MenuEntry::Engine(entry) => entry,
        };

        // Confirm a concede here, where the index has been resolved back to
        // the action it stands for. A priority offer always carries exactly
        // one Concede and exactly one PassPriority (the engine's legal
        // -actions invariant), so cancelling means passing.
        if let DisplayEntry::Direct(action_idx) = &entry {
            if matches!(legal_actions[*action_idx], Action::Concede) && !self.confirm_concede(view) {
                return Action::PassPriority;
            }
        }

        // A plan rides on a play: after Pass or a mana tap there is no
        // "then", there is the next offer.
        let plays = match &entry {
            DisplayEntry::Direct(i) => matches!(legal_actions[*i], Action::PlayLand { .. }),
            DisplayEntry::Cast(_) | DisplayEntry::Ability(_) => true,
        };
        if !plannable.is_empty() && plays && !then.is_empty() {
            self.log("PLAN", &format!("then: {}", then.join(", ")));
            self.plan = Some(Plan { turn: view.turn_number, step: view.step, steps: then, taken: 0, ask_again,
                opp_puts: view.opponent_stack_puts });
        } else if !plannable.is_empty() && plays && !ask_again {
            // Nothing after this play: the main phase is done, as it is at
            // the end of a plan.
            self.log("PLAN", "nothing after this; passing until something happens");
            self.plan_done_pending = true;
        }

        match &entry {
            DisplayEntry::Direct(action_idx) => {
                legal_actions[*action_idx].clone()
            }
            DisplayEntry::Cast(cs_idx) => {
                let cs = &legal.castable_spells[*cs_idx];
                self.choose_cast_targets(view, cs, legal_actions)
            }
            DisplayEntry::Ability(ab_idx) => {
                let ab = &legal.activatable_abilities[*ab_idx];
                self.choose_ability_targets(view, ab, legal_actions)
            }
        }
    }

    /// The names a main-phase answer may plan with, or none where no plan
    /// can be made: the seat's own main phase, an empty stack, a plain
    /// priority offer, the plan offered at all. Every card in hand, lands
    /// and spells alike — a spell the seat cannot afford yet may be
    /// castable once the land it plays first is down, which is the plan a
    /// person makes most.
    fn plannable(&self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Vec<String> {
        if !self.offer_pass_until || view.active_player != view.you || !view.stack.is_empty()
            || !matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
            || !Self::is_plain_priority_offer(legal)
        {
            return Vec::new();
        }
        let mut names: Vec<String> = Vec::new();
        for c in &view.your_hand {
            if !names.contains(&c.name) {
                names.push(c.name.clone());
            }
        }
        names
    }

    /// The plan fields (`then`, `ask_again`, `attack`) on an index question,
    /// read where `plannable` and `ready` are not empty and [`UNREAD`]
    /// elsewhere. Every index question carries them, so that a priority
    /// menu in the main phase, a response on the opponent's turn and a
    /// target are one schema and one cached prefix: each kind of question
    /// is a write of the whole prefix the first time a game asks it, and
    /// these three were three (reports/llm-cost.md, phase 3). A seat with
    /// the plan turned off (`MTG_LLM_PASS_UNTIL=off`) gets none.
    fn add_turn_fields(&self, schema: &mut serde_json::Value, plannable: &[String], ready: &[ObjectId]) {
        if !self.offer_pass_until {
            return;
        }
        let mut field = |key: &str, mut spec: serde_json::Value, read: bool| {
            if !read {
                spec[UNREAD] = serde_json::json!(true);
            }
            schema["properties"][key] = spec;
            if let Some(required) = schema["required"].as_array_mut() {
                required.push(serde_json::json!(key));
            }
        };
        let mut then_items = serde_json::json!({"type": "string"});
        if !plannable.is_empty() {
            then_items["enum"] = serde_json::json!(plannable);
        }
        field("then", serde_json::json!({"type": "array", "items": then_items, "description": PLAN_DESCRIPTION}),
            !plannable.is_empty());
        field("ask_again", serde_json::json!({"type": "boolean", "description": ASK_AGAIN_DESCRIPTION}),
            !plannable.is_empty());
        let mut attack_items = serde_json::json!({"type": "integer"});
        if !ready.is_empty() {
            attack_items["enum"] = serde_json::json!(ready.iter().map(|o| o.0).collect::<Vec<_>>());
        }
        field("attack", serde_json::json!({"type": "array", "items": attack_items, "description": ATTACK_DESCRIPTION}),
            !ready.is_empty());
    }

    /// The creatures a first-main-phase answer may name to attack with, or
    /// none where no attack can be planned: where a plan could be made at
    /// all (`plannable`'s conditions, a hand not needed), in the first main
    /// phase, untapped, not summoning-sick and under no "can't attack" now.
    /// What the view cannot see — a cost to attack, an attack the
    /// opponent's board forbids — is the declaration's to refuse, and the
    /// seat is then asked.
    fn ready_attackers(&self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Vec<ObjectId> {
        if !self.offer_pass_until || view.active_player != view.you || !view.stack.is_empty()
            || view.step != Step::PrecombatMain || !Self::is_plain_priority_offer(legal)
        {
            return Vec::new();
        }
        view.battlefield.iter()
            .filter(|p| p.controller == view.you && p.card_types.contains(&CardType::Creature)
                && !p.tapped && !p.affected_by_summoning_sickness
                && !p.restrictions.iter().any(|r| r.contains("can't attack")))
            .map(|p| p.object_id)
            .collect()
    }

    /// `Kalonian Tusker (#30), Grizzly Bears (#31)`: the seat's own
    /// creatures as its board names them.
    fn creature_names(view: &GameView, ids: &[ObjectId]) -> String {
        ids.iter().map(|&id| {
            let name = view.battlefield.iter().find(|p| p.object_id == id).map_or("a creature", |p| p.name.as_str());
            format!("{name} (#{})", id.0)
        }).collect::<Vec<_>>().join(", ")
    }

    /// The plan's move at this offer. Called only at a plain priority
    /// offer once the seat's own stack has been passed, so anything still
    /// on the stack is the opponent's.
    fn plan_next(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions, plan: &Plan) -> PlanMove {
        if view.active_player != view.you || view.turn_number != plan.turn || view.step != plan.step {
            // The engine passes for a seat with nothing to do, so a plan
            // whose next card could not be cast is not consulted again
            // until a later step.
            let next = plan.steps.first().map_or("it", String::as_str);
            return PlanMove::Stop(format!("{next} could not be played before the main phase ended"));
        }
        // The view's stack is top first: `last()` was the bottom, and the
        // seat was told its own spell had stopped its plan when the
        // opponent's response on top of it had (#870).
        if let Some(top) = view.stack.first() {
            return PlanMove::Stop(format!("{} ({}) is on the stack", top.name,
                if top.controller == view.you { "yours" } else { "the opponent's" }));
        }
        // Gone from the stack again by now: the engine passed for this seat
        // while it had nothing to respond with (#861).
        if view.opponent_stack_puts > plan.opp_puts {
            return PlanMove::Stop(OPPONENT_ACTED.to_string());
        }
        let Some(name) = plan.steps.first().cloned() else { return PlanMove::Done };
        let in_hand: Vec<ObjectId> = view.your_hand.iter().filter(|c| c.name == name).map(|c| c.object_id).collect();
        if in_hand.is_empty() {
            return PlanMove::Stop(format!("{name} is not in your hand"));
        }
        let land = legal.actions.iter().find(|a| matches!(a, Action::PlayLand { object_id } if in_hand.contains(object_id)));
        let cast = legal.castable_spells.iter().find(|cs| in_hand.contains(&cs.object_id)
            && !cs.from_graveyard && !cs.is_flashback && cs.alternative_cost.is_none());
        let action = if let Some(land) = land {
            land.clone()
        } else if let Some(cs) = cast {
            let cs = cs.clone();
            self.log("PLAN_STEP", &format!("cast {name} (planned, {} of {})", plan.taken + 1, plan.taken as usize + plan.steps.len()));
            self.advance_plan();
            return PlanMove::Act(self.choose_cast_targets(view, &cs, &legal.actions));
        } else {
            return PlanMove::Stop(format!("{name} cannot be played or cast now"));
        };
        self.log("PLAN_STEP", &format!("play {name} (planned, {} of {})", plan.taken + 1, plan.taken as usize + plan.steps.len()));
        self.advance_plan();
        PlanMove::Act(action)
    }

    /// The plan's next card is taken. After its last, the plan is done
    /// there and then — the seat passes until something happens — rather
    /// than at the next offer, which the engine skips when nothing is left
    /// to do.
    fn advance_plan(&mut self) {
        let Some(plan) = self.plan.as_mut() else { return };
        plan.steps.remove(0);
        plan.taken += 1;
        if plan.steps.is_empty() {
            let (taken, ask_again) = (plan.taken, plan.ask_again);
            self.plan = None;
            self.log("PLAN_DONE", &format!("{taken} planned play{} made; {}",
                if taken == 1 { "" } else { "s" },
                if ask_again { "asking again" } else { "passing until something happens" }));
            self.plan_done_pending = !ask_again;
        }
    }

    /// A plan ends short: say why in the log now and in the next prompt.
    fn stop_plan(&mut self, why: &str) {
        self.drop_attack_plan(why);
        if let Some(plan) = self.plan.take() {
            self.log("PLAN_STOP", &format!("{why} (after {} of {} planned)", plan.taken, plan.taken as usize + plan.steps.len()));
            self.plan_stopped = Some(format!(
                "Your plan stopped before {}: {why}.", plan.steps.join(", ")));
        }
    }
}

impl LlmPlayer {
    /// The rows of a priority offer, and what each row stands for.
    ///
    /// Pure — no backend, no log — so the rows can be held against the
    /// CLI's for the same `LegalActions` (`surface_parity.rs`). They were
    /// built inline in `choose_action` above the call to the model, which
    /// is where a key that dropped a legal row (#589, #610) could ship
    /// without any test seeing the menu it produced.
    pub(crate) fn build_action_rows(view: &GameView, legal: &mtg_engine::engine::LegalActions)
        -> (Vec<DisplayEntry>, Vec<ActionRow>)
    {
        /// What a display entry is shown as, before copies are grouped.
        enum Seed {
            One(String),
            /// An activated ability of a permanent, keyed by everything but
            /// which copy: the granting card and the label without the
            /// `(#id)`, so copies of one card offering the same ability the
            /// same way share a row.
            ///
            /// The key carries `source_card_id` and the row does not. An
            /// Aura can grant an ability that reads and costs exactly like
            /// the host's own, and grouping those together would put one
            /// copy in the range twice and claim the unenchanted copies
            /// offer it too (issue #589).
            AbilityCopy { key: (Option<mtg_engine::ids::CardId>, String), label: String, id: ObjectId },
        }

        let legal_actions = &legal.actions;
        // Build collapsed display: non-CastSpell/ActivateAbility actions + one per
        // castable spell + one per activatable ability.
        let mut seeds: Vec<(DisplayEntry, Seed)> = Vec::new();
        // Keyed by (object, alternative cost) — one row per way to cast
        // (issue #128), matching the CLI. Keyed by what the cost *is*, not
        // by whether there is one: two different alternative costs on one
        // object are two ways to cast it, and the engine's own key
        // (`invariants/legal.rs`, `collapsed_views`) says so. Same shape as
        // the activation key below, found in the same reading (#589). The
        // key is the engine's own, shared with the CLI
        // (`crate::cast_offer_key`).
        let mut seen_spell_objects: Vec<crate::CastOfferKey> = Vec::new();
        // The triple the engine keys an activation offer on
        // (`crate::ability_offer_key`). Keying on the pair instead dropped
        // every Aura- or Equipment-granted ability whose index collided
        // with one the host already had natively — always the granted one,
        // since natives are collected first — and did it silently: the two
        // halves of `LegalActions` agree, so no invariant could see it
        // (issue #589).
        let mut seen_ability_keys: Vec<crate::AbilityOfferKey> = Vec::new();

        let mut seen_cast_labels: Vec<String> = Vec::new();
        for (i, action) in legal_actions.iter().enumerate() {
            match action {
                Action::CastSpell { object_id, alternative_cost, .. } => {
                    let key = crate::cast_offer_key(*object_id, alternative_cost.as_ref());
                    if !seen_spell_objects.contains(&key) {
                        if let Some(cs_idx) = legal.castable_spells.iter()
                            .position(|cs| crate::cast_offer_key(cs.object_id, cs.alternative_cost.as_ref()) == key)
                        {
                            seen_spell_objects.push(key);
                            let cs = &legal.castable_spells[cs_idx];
                            let verb = if cs.is_flashback { "Flashback" } else { "Cast" };
                            let tap_str = Self::format_tap_plan(view, &cs.tap_plan);
                            // For ExileXFromGraveyard spells (Harvest Pyre), show the
                            // effective X *range* — the agent will pick X
                            // explicitly via the ChooseExileFromGraveyard prompt
                            // after picking the cast action. Shown as "X=0..N
                            // (0..N damage)" so the agent knows the range it
                            // can fund and the damage that scales with it.
                            let x_suffix = cs.exile_x_from_gy_max
                                .map(|n| format!(" X=0..{n} (0..{n} damage)"))
                                .unwrap_or_default();
                            let cost_note = cs.additional_cost_label.as_deref().unwrap_or("");
                            // A graveyard cast pays the printed cost (CR
                            // 601.3a); saying which zone is what tells it
                            // apart from the copy in hand (issue #300).
                            let zone_note = if cs.from_graveyard { " from graveyard" } else { "" };
                            let mut extras = Vec::new();
                            // What the row charges, in the one wording every
                            // surface uses (`crate::cast_cost_note`, #611).
                            if let Some(note) = crate::cast_cost_note(cs) { extras.push(note); }
                            if !cost_note.is_empty() { extras.push(cost_note.to_string()); }
                            if !tap_str.is_empty() { extras.push(format!("tap {tap_str}")); }
                            // The target and sacrifice taken without a
                            // prompt (`choose_cast_targets`), named (#827).
                            let forced_targets: &[mtg_engine::actions::Target] = match &cs.target_spec {
                                mtg_engine::actions::CastTargetSpec::SingleTarget(o) if o.len() == 1 => o,
                                _ => &[],
                            };
                            let forced_sac = match cs.sacrifice_options.as_slice() {
                                [only] => Some(*only),
                                _ => None,
                            };
                            let forced = Self::forced_suffix(view, forced_targets, forced_sac, None);
                            let label = if extras.is_empty() {
                                format!("{} {}{zone_note}{}{forced}", verb, cs.name, x_suffix)
                            } else {
                                format!("{} {}{zone_note}{} ({}){forced}", verb, cs.name, x_suffix, extras.join(", "))
                            };
                            // Deduplicate identical cast labels (e.g. two copies of same spell).
                            if seen_cast_labels.contains(&label) { continue; }
                            seen_cast_labels.push(label.clone());
                            seeds.push((DisplayEntry::Cast(cs_idx), Seed::One(label)));
                        }
                    }
                }
                Action::ActivateAbility { object_id, ability_index, source_card_id, .. } => {
                    let key = crate::ability_offer_key(*object_id, *ability_index, *source_card_id);
                    if !seen_ability_keys.contains(&key) {
                        if let Some(ab_idx) = legal.activatable_abilities.iter()
                            .position(|ab| crate::ability_offer_key(ab.object_id, ab.ability_index, ab.source_card_id) == key)
                        {
                            seen_ability_keys.push(key);
                            let ab = &legal.activatable_abilities[ab_idx];
                            let tap_str = Self::format_tap_plan(view, &ab.tap_plan);
                            let mut tail = if tap_str.is_empty() {
                                format!(" ({})", ab.description)
                            } else {
                                format!(" ({}) (tap {})", ab.description, tap_str)
                            };
                            // The slots `choose_ability_targets` fills
                            // without a prompt, named (#827). Part of the
                            // grouping key too: copies aimed at different
                            // forced targets are not one row.
                            let (targets, sacrifices) = Self::ability_slots(ab);
                            let forced_targets: &[mtg_engine::actions::Target] = match targets.as_slice() {
                                [only] => only,
                                _ => &[],
                            };
                            let forced_sac = match sacrifices.as_slice() {
                                [only] => *only,
                                _ => None,
                            };
                            tail.push_str(&Self::forced_suffix(view, forced_targets, forced_sac, Some(ab.object_id)));
                            // The engine names the permanent `Name (#id)`;
                            // the row shared by its copies names the card.
                            let id_suffix = format!(" (#{})", ab.object_id.0);
                            let card = ab.name.strip_suffix(id_suffix.as_str()).unwrap_or(&ab.name);
                            seeds.push((DisplayEntry::Ability(ab_idx), Seed::AbilityCopy {
                                key: (ab.source_card_id, format!("Activate {card}{tail}")),
                                label: format!("Activate {}{tail}", ab.name),
                                id: ab.object_id,
                            }));
                        }
                    }
                }
                _ => {
                    seeds.push((DisplayEntry::Direct(i), Seed::One(Self::format_single_action(view, action))));
                }
            }
        }

        // Copies of one permanent offering the same ability the same way
        // are one row with an index per copy (issue #461). A group's
        // members are made contiguous so its indices are a range; the
        // entries are reordered with the rows, so an index still names the
        // option it was shown as.
        let mut rows: Vec<ActionRow> = Vec::new();
        let mut display_entries: Vec<DisplayEntry> = Vec::new();
        let mut grouped: Vec<&(Option<mtg_engine::ids::CardId>, String)> = Vec::new();
        for (entry, seed) in &seeds {
            match seed {
                Seed::One(label) => {
                    rows.push(ActionRow::One(label.clone()));
                    display_entries.push(*entry);
                }
                Seed::AbilityCopy { key, label, .. } => {
                    if grouped.contains(&key) { continue; }
                    let members: Vec<(DisplayEntry, ObjectId)> = seeds.iter()
                        .filter_map(|(e, s)| match s {
                            Seed::AbilityCopy { key: k, id, .. } if k == key => Some((*e, *id)),
                            _ => None,
                        })
                        .collect();
                    // "One per copy" is a claim about the board: n copies of
                    // one permanent, one index each, each told apart by its
                    // `#id`. Two entries on the SAME object are not copies —
                    // they are two ways of paying one ability's cost that
                    // happen to render alike — and the row then asserted a
                    // second permanent that is not there: "one per copy:
                    // 4=#37, 5=#37" with one #37 on the board, and no way to
                    // tell index 4 from index 5 (issue #612). A row that
                    // repeats an id says nothing true, so each entry keeps its
                    // own row instead; the engine-side invariant is what stops
                    // two rows of one permanent reading alike in the first
                    // place.
                    let mut ids: Vec<ObjectId> = Vec::new();
                    for (_, id) in &members {
                        if !ids.contains(id) { ids.push(*id); }
                    }
                    if members.len() == 1 {
                        rows.push(ActionRow::One(label.clone()));
                        display_entries.push(*entry);
                    } else if ids.len() < members.len() {
                        grouped.push(key);
                        for (e, _) in &members {
                            rows.push(ActionRow::One(label.clone()));
                            display_entries.push(*e);
                        }
                    } else {
                        grouped.push(key);
                        rows.push(ActionRow::Copies { label: key.1.clone(), ids });
                        display_entries.extend(members.iter().map(|(e, _)| *e));
                    }
                }
            }
        }

        (display_entries, rows)
    }

    /// The priority menu as the seat is sent it: the engine's rows from
    /// [`build_action_rows`](Self::build_action_rows), plus
    /// [`PASS_UNTIL_ROW`] right after `Pass` when it is offered. The row
    /// is the seat's own, not an engine action: the random seat, the CLI
    /// and the page do not get it (the page's `f` key and the CLI's
    /// auto-pass are the same thing as a keystroke), and the surface-parity
    /// check compares the engine's rows only — `surface_parity.rs` says
    /// why. One row, at a fixed place, never one per phase.
    pub(crate) fn priority_menu(
        view: &GameView, legal: &mtg_engine::engine::LegalActions, offer_pass_until: bool,
    ) -> (Vec<MenuEntry>, Vec<ActionRow>) {
        let (entries, mut rows) = Self::build_action_rows(view, legal);
        let mut menu: Vec<MenuEntry> = entries.into_iter().map(MenuEntry::Engine).collect();
        if !offer_pass_until {
            return (menu, rows);
        }
        // The row goes after Pass, which is a single-entry row; walk the
        // rows with their index widths so a copies row ahead of it (there
        // is none — Pass is the engine's first action) would not shift it.
        let mut entry_at = 0usize;
        for (row_pos, row) in rows.iter().enumerate() {
            let width = match row {
                ActionRow::One(_) => 1,
                ActionRow::Copies { ids, .. } => ids.len(),
            };
            if let (ActionRow::One(_), Some(MenuEntry::Engine(DisplayEntry::Direct(i)))) = (row, menu.get(entry_at)) {
                if matches!(legal.actions.get(*i), Some(Action::PassPriority)) {
                    rows.insert(row_pos + 1, ActionRow::One(PASS_UNTIL_ROW.to_string()));
                    menu.insert(entry_at + 1, MenuEntry::PassUntil);
                    return (menu, rows);
                }
            }
            entry_at += width;
        }
        // No Pass row: nothing to pass with, so no row either.
        (menu, rows)
    }

    /// A plain priority offer: a menu with Pass on it and no prompt of
    /// another kind. Everything else is a question the seat is asked
    /// whatever it said earlier.
    fn is_plain_priority_offer(legal: &mtg_engine::engine::LegalActions) -> bool {
        legal.combat_prompt.is_none()
            && legal.resolution_prompt.is_none()
            && legal.set_prompt.is_none()
            && !legal.actions.iter().any(|a| matches!(a, Action::MulliganKeep))
            && legal.actions.iter().any(|a| matches!(a, Action::PassPriority))
    }

    /// The seat chose [`PASS_UNTIL_ROW`]: pass now, and remember to.
    fn engage_pass_until(&mut self, view: &GameView) {
        let mode = PassUntil::engaged_at(view);
        self.log("AUTO_PASS", &format!("engaged at {}: passing until something happens", mode.since()));
        self.pass_until = Some(mode);
    }

    /// A pass-until ends: say why in the log now and in the next prompt.
    fn stop_pass_until(&mut self, why: &str) {
        // Something happened that the seat is to be asked about: the attack
        // it named before is its to make again.
        self.drop_attack_plan(why);
        if let Some(mode) = self.pass_until.take() {
            self.log("AUTO_PASS_STOP", &format!("{why} ({} offer{} passed unasked since {})",
                mode.passes, if mode.passes == 1 { "" } else { "s" }, mode.since()));
            self.pass_until_stopped = Some((mode.passes, why.to_string()));
        }
    }

    /// An attack plan that will not be used, said in the log.
    fn drop_attack_plan(&mut self, why: &str) {
        if let Some(plan) = self.attack_plan.take() {
            self.log("ATTACK_PLAN_DROPPED", &format!("{why} (turn {} attack with {} creature{})",
                plan.turn, plan.attackers.len(), if plan.attackers.len() == 1 { "" } else { "s" }));
        }
    }

    /// The line the first prompt after a pass-until opens with.
    /// `GAME_RULES` quotes its shape.
    fn pass_until_stopped_line(passes: u32, why: &str) -> String {
        format!("After {passes} priority offer{} passed for you unasked, the pass stopped: {why}.",
            if passes == 1 { "" } else { "s" })
    }

    /// Why a plain priority offer ends a pass-until, or `None` to pass it
    /// unasked. Conservative where a decision may be owed, and not where
    /// none is:
    ///
    /// - anything on the stack that was not there, below the top, when the
    ///   seat said "go" (an entry resolving off the top is expected), or
    ///   anything put on it since — which, the seat's own being passed
    ///   before this is consulted, means something of the opponent's;
    /// - a blocker declared against the seat's attacker;
    /// - the opponent attacking the seat, once blocks are in: the window a
    ///   trick or a removal spell is cast in;
    /// - the seat's own first main phase, and a later main phase of its own
    ///   — or the one it said "go" in, unless it said it was done with it
    ///   (`done_here`) — when a land drop or a sorcery-speed cast or
    ///   activation is on offer. A second main phase with nothing
    ///   to do at sorcery speed is not a decision: an instant can be cast
    ///   at the next stop as well;
    /// - no Pass on the menu, which is not a priority offer at all.
    ///
    /// The seat's own turn beginning used to stop it too, and was a model
    /// call every turn that came back "pass" (reports/llm-cost.md, phase 3):
    /// the upkeep and draw step hold nothing the first main phase does not.
    /// Each stop at a combat step fires once, not again at the step the
    /// seat said "go" in.
    fn pass_until_stop(
        view: &GameView, legal: &mtg_engine::engine::LegalActions, mode: &PassUntil,
    ) -> Option<String> {
        if !legal.actions.iter().any(|a| matches!(a, Action::PassPriority)) {
            return Some("this is not a priority pass".to_string());
        }
        let our_turn = view.active_player == view.you;
        let same_point = view.turn_number == mode.turn && view.step == mode.step
            && our_turn == mode.our_turn;

        if !view.stack.is_empty() {
            let now = PassUntil::stack_identity(view);
            // Top first, as the view lists it (#870): what resolved since
            // came off the front, so what is left is the end of what was.
            let unchanged = same_point && view.stack_puts == mode.stack_puts
                && now.len() <= mode.stack.len() && mode.stack[mode.stack.len() - now.len()..] == now[..];
            if !unchanged {
                let newest = view.stack.iter()
                    .find(|s| !mode.stack.contains(&(s.object_id, s.card_id, s.name.clone())))
                    .or_else(|| view.stack.first())
                    .map_or_else(|| "something".to_string(), |s| format!("{} ({})", s.name,
                        if s.controller == view.you { "yours" } else { "the opponent's" }));
                return Some(format!("{newest} is on the stack"));
            }
        }
        // What the opponent cast and resolved while the engine passed for
        // the seat (#861) is not a stop: there is no window left to answer
        // it in, and the board shows what it did. It does stop a plan and
        // a planned attack, which were made before it (`plan_next`,
        // `planned_attack`) — a stop here was a call in every third
        // opponent's turn, measured.

        if !our_turn && view.step == Step::DeclareBlockers && !same_point {
            let attackers: Vec<String> = view.battlefield.iter()
                .filter(|p| p.controller != view.you && p.attacking.is_some())
                .map(|p| Self::obj_label(view, p.object_id))
                .collect();
            if !attackers.is_empty() {
                return Some(format!("the opponent is attacking you with {} and blocks are in", attackers.join(", ")));
            }
        }

        if our_turn && view.step == Step::DeclareBlockers && !same_point {
            let blocked: Vec<String> = view.battlefield.iter()
                .filter(|p| p.controller == view.you && p.attacking.is_some() && p.blocked)
                .map(|p| Self::obj_label(view, p.object_id))
                .collect();
            if !blocked.is_empty() {
                return Some(format!("your attacker{} blocked: {}",
                    if blocked.len() == 1 { " is" } else { "s are" }, blocked.join(", ")));
            }
        }

        if our_turn && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
            && !(same_point && mode.done_here)
        {
            if !same_point && view.step == Step::PrecombatMain {
                return Some(format!("your {}", Self::step_name(view.step, false)));
            }
            if legal.actions.iter().any(|a| matches!(a, Action::PlayLand { .. })) {
                return Some("you can play a land".to_string());
            }
            if let Some(cs) = legal.castable_spells.iter().find(|cs| !Self::is_instant_speed_cast(view, cs)) {
                return Some(format!("you could cast {} at sorcery speed", cs.name));
            }
            if let Some(ab) = legal.activatable_abilities.iter().find(|ab| ab.sorcery_speed) {
                return Some(format!("you could activate {} ({}) at sorcery speed", ab.name, ab.description));
            }
        }
        None
    }

    /// What a pass-until is still waiting for, for the `AUTO_PASS` line.
    fn pass_until_still_passing(view: &GameView, mode: &PassUntil) -> String {
        let our_turn = view.active_player == view.you;
        if !our_turn {
            if mode.our_turn {
                "your next turn, or something of the opponent's".to_string()
            } else {
                "your turn, or something of the opponent's".to_string()
            }
        } else if matches!(view.step, Step::Untap | Step::Upkeep | Step::Draw | Step::PrecombatMain) {
            "your main phase".to_string()
        } else if matches!(view.step, Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers
            | Step::CombatDamage | Step::EndCombat) {
            "your postcombat main, or a block".to_string()
        } else {
            "your next turn, or something of the opponent's".to_string()
        }
    }

    /// Whether a castable spell could be cast at instant speed: an
    /// instant, or a card with flash. A card the view cannot show (not in
    /// hand or a graveyard) is taken as sorcery-speed, which is the side
    /// that asks.
    fn is_instant_speed_cast(view: &GameView, cs: &mtg_engine::actions::CastableSpell) -> bool {
        let card = view.your_hand.iter().find(|c| c.object_id == cs.object_id)
            .or_else(|| view.graveyards.iter().flat_map(|(_, cards)| cards.iter())
                .find(|c| c.object_id == cs.object_id));
        card.is_some_and(|c| c.card_types.contains(&CardType::Instant)
            || c.oracle_text.lines().any(|l| {
                let l = l.trim();
                l.eq_ignore_ascii_case("flash") || l.starts_with("Flash ") || l.starts_with("Flash,")
            }))
    }

    /// Format a card for the mulligan prompt: `Name {cost}[ P/T]`.
    fn format_hand_card(c: &mtg_engine::view::CardView) -> String {
        let cost = c.cost.as_ref().map(|co| format!(" {co}")).unwrap_or_default();
        let pt = match (c.power, c.toughness) {
            (Some(p), Some(t)) => format!(" {p}/{t}"),
            _ => String::new(),
        };
        format!("{}{}{}", c.name, cost, pt)
    }

    /// Render the player's hand as a numbered list for the mulligan /
    /// bottom prompts.
    fn format_numbered_hand(view: &GameView) -> String {
        if view.your_hand.is_empty() {
            return "  <empty>".to_string();
        }
        view.your_hand.iter().enumerate()
            .map(|(i, c)| format!("  {}: {}", i, Self::format_hand_card(c)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// One-line summary of each opponent's mulligan count, for the
    /// pre-game mulligan prompts.
    fn format_opponent_mulls(view: &GameView) -> String {
        match view.opponents.len() {
            0 => String::new(),
            1 => {
                let n = view.opponents[0].mulligan_count;
                format!("Opponent has taken {} mulligan{} so far.",
                    n, if n == 1 { "" } else { "s" })
            }
            _ => {
                let parts: Vec<String> = view.opponents.iter().enumerate()
                    .map(|(i, o)| format!("opp{}: {}", i, o.mulligan_count))
                    .collect();
                format!("Opponents' mulligans so far — {}.", parts.join(", "))
            }
        }
    }

    /// Decide keep or mulligan for the London opening-hand phase.
    /// Sends a structured-JSON prompt with the current hand and the
    /// mulligan count. Falls back to `MulliganKeep` on malformed responses.
    /// When a further mulligan cannot change the kept hand and keep is the only sensible
    /// action, returns `MulliganKeep` directly without round-tripping the LLM.
    /// The keep-or-mulligan prompt.
    ///
    /// It leads with the `[MULLIGAN DECISION]` context marker GAME_RULES
    /// documents and the `[CONTEXT]` section tells the model to key off. It
    /// used to carry no bracketed context line at all, unlike every other
    /// prompt the harness sends (issue #201).
    fn mulligan_prompt(
        play_draw: &str,
        mulls_taken: u32,
        keep_size: i32,
        opp_mulls_text: &str,
        hand_text: &str,
    ) -> String {
        let plural = if mulls_taken == 1 { "" } else { "s" };
        format!(
            "[MULLIGAN DECISION]\n\
             London mulligan decision — keep or mulligan?\n\
             \n\
             {play_draw}. You have taken {mulls_taken} mulligan{plural} so far. \
If you keep now you will bottom {mulls_taken} card{plural} and play with {keep_size} in hand. \
A mulligan draws seven again and costs one card more. {MULLIGAN_ADVICE}\n\
             {opp_mulls_text}\n\
             \n\
             Your opening hand:\n\
             {hand_text}"
        )
    }

    /// The bottom-N-after-mulligan prompt, led by the
    /// `[BOTTOM N CARD(S) AFTER MULLIGAN]` marker GAME_RULES documents.
    fn mulligan_bottom_prompt(
        play_draw: &str,
        n: usize,
        opp_mulls_text: &str,
        hand_text: &str,
    ) -> String {
        let plural = if n == 1 { "" } else { "s" };
        format!(
            "[BOTTOM {n} CARD{} AFTER MULLIGAN]\n\
             Bottom {n} card{plural} after mulligan.\n\
             \n\
             {play_draw}. You took {n} mulligan{plural} and have kept — pick {n} card{plural} \
from your hand to put on the bottom of your library.\n\
             {opp_mulls_text}\n\
             \n\
             Your opening hand:\n\
             {hand_text}",
            if n == 1 { "" } else { "(S)" }
        )
    }

    fn choose_mulligan(&mut self, view: &GameView, legal_actions: &[Action]) -> Action {
        let mull_allowed = legal_actions.iter().any(|a| matches!(a, Action::MulliganMull));
        // CR 103.4 caps nothing, and the engine offers the mulligan at every
        // count. Past seven, though, every remaining choice is the same
        // choice: the kept hand is empty either way, so a seat that keeps
        // answering "mull" would loop forever without ever changing the
        // game. Stop asking once the answer cannot matter — this is a policy
        // floor for an automated seat, not a rule.
        if !mull_allowed || mulligan_is_dominated(view) {
            self.log(
                "AUTO-KEEP",
                if mull_allowed {
                    "seven mulligans taken: any further mulligan keeps the same empty hand"
                } else {
                    "no mulligan on offer, forced to keep"
                },
            );
            return Action::MulliganKeep;
        }

        let hand_text = Self::format_numbered_hand(view);
        let mulls_taken = view.your_mulligan_count;
        let keep_size = (7_i32 - i32::try_from(mulls_taken).unwrap_or(i32::MAX)).max(0);
        let opp_mulls_text = Self::format_opponent_mulls(view);
        let play_draw = if view.active_player == view.you {
            "You are on the play"
        } else {
            "You are on the draw"
        };

        let full_prompt = self.with_notes_after_marker(&Self::mulligan_prompt(
            play_draw,
            mulls_taken,
            keep_size,
            &opp_mulls_text,
            &hand_text,
        ));

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "mull": {"type": "boolean", "description": "true = mulligan, false = keep"}
            },
            "required": ["thoughts", "mull"]
        });

        let response = self.send_message_structured(&full_prompt, &schema);
        let choice = response["mull"].as_bool();
        match choice {
            Some(true) if mull_allowed => {
                self.log("CHOSE", "mulligan");
                Action::MulliganMull
            }
            Some(true) => {
                // The engine did not offer the mulligan (it always does at
                // this prompt, so this is a malformed legal-action list).
                self.log_rejected("Requested a mulligan that was not on offer — forcing keep");
                Action::MulliganKeep
            }
            Some(false) => {
                self.log("CHOSE", "keep");
                Action::MulliganKeep
            }
            None => {
                self.log_rejected(&format!("Mulligan response missing 'mull' bool ({response}), defaulting to keep"));
                Action::MulliganKeep
            }
        }
    }

    /// Decide which cards to put on the bottom after all mulligans.
    /// Sends a structured-JSON prompt with the numbered hand and expected
    /// count. Falls back to the first enumerated legal `BottomCards` option
    /// if the response is malformed.
    /// Pick a set of cards out of the hand: which to bottom after a
    /// mulligan (CR 103.4), or which to discard down to hand size (CR
    /// 514.1). One question, answered with the indices.
    fn choose_card_set(&mut self, view: &GameView, prompt: &mtg_engine::actions::SetPrompt) -> Action {
        use mtg_engine::actions::SetPromptKind;
        let n = prompt.min;
        let hand_text = Self::format_numbered_hand(view);
        let full_prompt = match prompt.kind {
            SetPromptKind::BottomAfterMulligan => {
                let opp_mulls_text = Self::format_opponent_mulls(view);
                let play_draw = if view.active_player == view.you {
                    "You are on the play"
                } else {
                    "You are on the draw"
                };
                self.with_notes_after_marker(
                    &Self::mulligan_bottom_prompt(play_draw, n, &opp_mulls_text, &hand_text))
            }
            SetPromptKind::DiscardToHandSize => {
                // Through `build_prompt` like every other in-game decision:
                // this was the one prompt built from a format string alone,
                // so the seat chose what to throw away from eight card names
                // and nothing else — no turn, no life totals, no board, no
                // graveyard, when three of the eight cards' castability or
                // size was a function of the graveyard (issue #463). Going
                // through `build_prompt` also consumes the log, so the recap
                // stays a delta across a run of cleanup discards (#464).
                let plural = if n == 1 { "" } else { "s" };
                let action_text = format!(
                    "[DISCARD {n} CARD{}]\n\
                     Cleanup: your hand is over seven cards. Discard {n} card{plural} (CR 514.1).\n\
                     \n\
                     Your hand:\n\
                     {hand_text}",
                    if n == 1 { "" } else { "S" });
                self.build_prompt(view, &action_text)
            }
        };

        let valid_indices: Vec<serde_json::Value> = (0..prompt.options.len())
            .map(|i| serde_json::json!(i))
            .collect();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "card_indices": {
                    "type": "array",
                    "items": {"type": "integer", "enum": valid_indices},
                    "minItems": prompt.min,
                    "maxItems": prompt.max,
                    // The count and the purpose are the prompt's to say: the
                    // schema is part of the cached prefix
                    // (`cache_stable_schema`), and one schema serves both.
                    "description": "Distinct 0-indexed positions in your hand, as many as the prompt says"
                }
            },
            "required": ["thoughts", "card_indices"]
        });

        let response = self.send_message_structured(&full_prompt, &schema);

        let indices: Option<Vec<usize>> = response["card_indices"].as_array().map(|arr| {
            arr.iter()
                .filter_map(serde_json::Value::as_i64)
                .filter(|i| *i >= 0)
                .map(|i| usize::try_from(i).unwrap_or(0))
                .collect()
        });
        // The fallback is the first `min` cards of the hand, which is the
        // same answer RandomPlayer gives and is always legal — there is no
        // enumerated list to fall back into any more.
        let mut seen = std::collections::HashSet::new();
        let ok = indices.as_ref().is_some_and(|v| {
            v.len() >= prompt.min && v.len() <= prompt.max
                && v.iter().all(|&i| i < prompt.options.len() && seen.insert(i))
        });
        if !ok {
            self.log_rejected("Invalid card_indices — defaulting to the first cards in hand");
            return prompt.answer(prompt.options.iter().take(prompt.min).copied().collect());
        }
        let indices = indices.unwrap_or_default();
        let cards: Vec<ObjectId> = indices.iter().map(|&i| prompt.options[i]).collect();
        self.log("CHOSE", &format!("card set {indices:?}"));
        prompt.answer(cards)
    }

    /// A permanent's colors in the lowercase the board text uses, from the
    /// engine's one renderer.
    fn format_colors(colors: &[mtg_engine::types::Color]) -> String {
        mtg_engine::types::colors_line(colors).to_lowercase()
    }

    /// Everything a permanent's row says it can do and cannot: its keywords,
    /// its protections (CR 702.16) and its restrictions.
    ///
    /// `format_keywords` alone was what both board renderers used, and
    /// protection is not a `Keyword` in this engine — the field exists
    /// precisely because it cannot ride in `keywords` — so Elite
    /// Inquisitor's "protection from Vampires and from Werewolves" and Grave
    /// Bramble's "protection from Zombies" never reached a model seat, and
    /// Spare from Evil's timed protection was representable nowhere else at
    /// all. A seat choosing blocks and targets against an opponent's
    /// creature had a bare `Name (#id) P/T colour, keywords` row to do it
    /// from (issue #506). Restrictions are the same gap on the same field's
    /// sibling (issue #504).
    ///
    /// One function, because there are two board renderers and the last
    /// field to be added reached one of them. It replaces `format_keywords`,
    /// which is what both used to call; the keyword's printed word still
    /// comes from the engine beside the enum, which was one of five copies
    /// of that table before (#363).
    fn format_abilities(p: &mtg_engine::view::PermanentView) -> String {
        p.keywords.iter().map(|kw| kw.label().to_string())
            .chain(p.protections.iter().cloned())
            .chain(p.restrictions.iter().cloned())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn is_legendary(p: &mtg_engine::view::PermanentView) -> bool {
        p.supertypes.contains(&mtg_engine::types::Supertype::Legendary)
    }

    /// Format a permanent for combat/selection display: "Name (#id) P/T keywords".
    /// Always includes the object ID for unambiguous reference.
    fn format_combat_creature(view: &GameView, id: ObjectId) -> String {

        if let Some(p) = view.battlefield.iter().find(|p| p.object_id == id) {
            // A non-creature has no power or toughness, and printing it
            // `0/0` is a claim about the permanent, not a missing value:
            // handed a board of lands to divide, a seat read the rows back
            // as "all 24 Swamps are functionally identical (0/0, no other
            // stats matter)" (#495). Only the combat prompts this was
            // written for pass creatures, and their rows are unchanged.
            let pt = if p.card_types.contains(&mtg_engine::types::CardType::Creature) {
                let power = p.effective_power.or(p.power).unwrap_or(0);
                let toughness = p.effective_toughness.or(p.toughness).unwrap_or(0);
                format!(" {power}/{toughness}")
            } else {
                String::new()
            };
            // Color is not repeated here: it is a characteristic, and the
            // board section of the same prompt states every creature's
            // (#357). The combat rows stay the shape they have.
            let kw = Self::format_abilities(p);
            if kw.is_empty() {
                format!("{} (#{}){}", p.name, id.0, pt)
            } else {
                format!("{} (#{}){} {}", p.name, id.0, pt, kw)
            }
        } else {
            Self::obj_label(view, id)
        }
    }

    /// The pile-division answer's schema: `piles`, one "A" or "B" per
    /// permanent in the order the prompt lists them. Its length is required —
    /// the prompt asks for each permanent, and when `{}` was a schema-valid
    /// answer the whole board went to pile B unasked (#662). The permanents
    /// are not property names (#857): keyed by `Name (#id)`, every division
    /// sent a schema of its own and wrote the cached prefix again; the count
    /// is a per-call constraint `cache_stable_schema` leaves out.
    fn pile_division_schema(n: usize) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {
                    "type": "string",
                    "description": THOUGHTS_DESCRIPTION
                },
                "piles": {
                    "type": "array",
                    "items": {"type": "string", "enum": ["A", "B"]},
                    "minItems": n,
                    "maxItems": n,
                    "description": "The pile of each listed permanent, one entry per permanent in the order listed"
                }
            },
            "required": ["thoughts", "piles"]
        })
    }

    /// Build labels for a list of permanent IDs, each with its object ID for
    /// unambiguous reference. Also appends any attached aura/equipment context
    /// inline so the model can see which copy has which attachments.
    ///
    /// Returns a Vec<String> of the same length as `ids`, in the same order.
    fn format_combat_creature_list(view: &GameView, ids: &[ObjectId]) -> Vec<String> {
        let base: Vec<String> = ids.iter().map(|&id| Self::format_combat_creature(view, id)).collect();
        let attached: Vec<String> = ids.iter().map(|&id| {
            let bits: Vec<String> = view.battlefield.iter()
                .filter(|p| p.attached_to == Some(id))
                .map(|p| p.name.clone())
                .collect();
            // In words, not a bracket: the legend's one bracket is the
            // board's status flags, and `[+X]` read like a counter (#736).
            if bits.is_empty() { String::new() } else { format!(" (attached: {})", bits.join(", ")) }
        }).collect();

        let mut out = Vec::with_capacity(ids.len());
        for (i, label) in base.iter().enumerate() {
            out.push(format!("{}{}", label, attached[i]));
        }
        out
    }

    /// The declare-attackers answer's schema, bounded by what the prompt
    /// offers. It was the one index array in the seat with only a
    /// `minimum`, so an index past the end was schema-valid — the API passed
    /// it and the parser dropped it (issue #635). Every index here is an
    /// `enum` of the offered ones, as `mark_indices` and the blockers are;
    /// with no planeswalker to attack the array admits no entry at all.
    fn attackers_schema(attackers: usize, planeswalkers: usize) -> serde_json::Value {
        let attacker_enum: Vec<usize> = (0..attackers).collect();
        let walker_attack = if planeswalkers == 0 {
            serde_json::json!({"type": "array", "maxItems": 0, "items": {"type": "object"},
                "description": "No planeswalker can be attacked: leave empty"})
        } else {
            let walker_enum: Vec<usize> = (0..planeswalkers).collect();
            serde_json::json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "attacker": {"type": "integer", "enum": attacker_enum},
                        "planeswalker": {"type": "integer", "enum": walker_enum}
                    },
                    "required": ["attacker", "planeswalker"]
                },
                "description": "Attackers sent at a defending planeswalker instead of the player (attacker index + pw index); omit or empty if none"
            })
        };
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "attacker_indices": {
                    "type": "array",
                    "items": {"type": "integer", "enum": attacker_enum},
                    "description": "Indices of creatures to attack with (empty array for none)"
                },
                "planeswalker_attacks": walker_attack
            },
            "required": ["thoughts", "attacker_indices"]
        })
    }

    /// The attack the seat named in its first main phase, declared without
    /// a call — if it is this turn's, nothing has happened since (a stop
    /// drops it), no planeswalker is there to be attacked instead, and every
    /// creature it names can attack. Otherwise the seat is asked, told why
    /// its plan was not used.
    fn planned_attack(&mut self, view: &GameView, prompt: &CombatPrompt) -> Option<Action> {
        let CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player, defending_planeswalkers } = prompt else {
            return None;
        };
        let plan = self.attack_plan.take()?;
        if plan.turn != view.turn_number {
            return None;
        }
        let names = |ids: &[ObjectId]| Self::creature_names(view, ids);
        let unable: Vec<ObjectId> = plan.attackers.iter().copied().filter(|a| !eligible.contains(a)).collect();
        let why = if view.opponent_stack_puts > plan.opp_puts {
            // A flash blocker or a trick cast while the engine passed for
            // this seat: the attack was named before it (#861).
            Some(OPPONENT_ACTED.to_string())
        } else if !defending_planeswalkers.is_empty() {
            Some("the opponent has a planeswalker you could attack instead".to_string())
        } else if !unable.is_empty() {
            Some(format!("{} cannot attack now", names(&unable)))
        } else {
            None
        };
        if let Some(why) = why {
            self.log("ATTACK_PLAN_DROPPED", &why);
            self.plan_stopped = Some(format!("Your planned attack with {} was not declared: {why}.", names(&plan.attackers)));
            return None;
        }
        let mut attackers = plan.attackers;
        for &id in must_attack {
            if !attackers.contains(&id) {
                attackers.push(id);
            }
        }
        self.log("ATTACK_PLAN", &format!("declared as planned: {}", names(&attackers)));
        self.log("CHOSE", &format!("attack with {} (planned)", attackers.len()));
        Some(Action::DeclareAttackers {
            attackers: attackers.into_iter().map(|a| (a, *defending_player)).collect(),
            planeswalker_attacks: Vec::new(),
        })
    }

    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        let action = self.decide_combat(view, prompt);
        self.sync_memory_mirror();
        action
    }

    fn decide_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        // A combat prompt with one legal answer is not worth a round trip to
        // the model. One rule, shared with the other three seats (#517).
        if let Some(forced) = crate::forced_combat_answer(prompt) {
            return forced;
        }
        // A combat declaration is always asked, whatever the seat said
        // earlier — but for the attack it named in its first main phase
        // (`planned_attack`); a pass-until in progress carries on past it, to the stops
        // the declaration sets up (a block against the attacker, blocks in
        // against the seat). Ending it here, as this did, made the very next
        // priority offer — in the step the seat had just declared in — a
        // model call that came back "pass" (reports/llm-cost.md, phase 3).
        let action = self.decide_combat_prompt(view, prompt);
        // Attackers declared, the seat waits for blocks as a person does.
        if self.offer_pass_until && matches!(prompt, CombatPrompt::ChooseAttackers { .. })
            && self.pass_until.is_none()
        {
            let mode = PassUntil::engaged_at(view);
            self.log("AUTO_PASS", &format!("engaged at {} after declaring attackers: passing until a block or your second main phase", mode.since()));
            self.pass_until = Some(mode);
        }
        action
    }

    fn decide_combat_prompt(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        if let Some(planned) = self.planned_attack(view, prompt) {
            return planned;
        }
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player,
                                            defending_planeswalkers } => {
                // Build disambiguated labels so the model can tell apart two
                // creatures that would otherwise render with identical text.
                let labels = Self::format_combat_creature_list(view, eligible);

                let mut combat_text = String::new();
                if !must_attack.is_empty() {
                    combat_text.push_str("MUST ATTACK: ");
                    for &id in must_attack {
                        if let Some(idx) = eligible.iter().position(|&e| e == id) {
                            write!(combat_text, "{}:{} ", idx, labels[idx]).unwrap();
                        }
                    }
                    combat_text.push('\n');
                }
                combat_text.push_str("Choose attackers: ");
                for (i, &id) in eligible.iter().enumerate() {
                    let forced = if must_attack.contains(&id) { " [MUST]" } else { "" };
                    write!(combat_text, "{}:{}{} ", i, labels[i], forced).unwrap();
                }
                write!(combat_text,
                    "\nPick indices in 0-{} to attack with, or empty list for no attacks. Forced attackers are auto-included.",
                    eligible.len() - 1
                ).unwrap();
                if !defending_planeswalkers.is_empty() {
                    combat_text.push_str("\nDefending planeswalkers (attackable instead of the player): ");
                    for (i, &id) in defending_planeswalkers.iter().enumerate() {
                        let name = view.battlefield.iter().find(|p| p.object_id == id)
                            .map_or_else(|| format!("#{}", id.0), |p| p.name.clone());
                        write!(combat_text, "pw{i}:{name} ").unwrap();
                    }
                    combat_text.push_str(
                        "\nTo send an attacker at a planeswalker, list it in planeswalker_attacks \
                         as {\"attacker\": <attacker index>, \"planeswalker\": <pw index>} \
                         instead of putting its index in attacker_indices.");
                }

                let full_prompt = self.build_prompt(view, &combat_text);

                let schema = Self::attackers_schema(eligible.len(), defending_planeswalkers.len());

                let response = self.send_message_structured(&full_prompt, &schema);

                // Attacking with nobody is a real decision, and it is also
                // what a seat that gave no usable array gets (#399).
                if !response["attacker_indices"].is_array() {
                    self.log_rejected(&format!(
                        "no usable 'attacker_indices' ({}); declaring no attackers",
                        response["attacker_indices"]));
                }
                // An index the prompt did not offer is an answer the harness
                // cannot use, and is said so: dropping it silently turned
                // "attack with #4" into "#4 stays home" with no MALFORMED
                // line and no rejection counted (issue #635).
                let mut dropped: Vec<String> = Vec::new();
                let mut indices: Vec<usize> = response["attacker_indices"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| match v.as_u64().and_then(|n| usize::try_from(n).ok()) {
                            Some(i) if i < eligible.len() => Some(i),
                            _ => { dropped.push(v.to_string()); None }
                        })
                        .collect())
                    .unwrap_or_default();
                let answered = indices.clone();

                // Always include forced attackers.
                for &id in must_attack {
                    if let Some(idx) = eligible.iter().position(|&e| e == id) {
                        if !indices.contains(&idx) {
                            indices.push(idx);
                        }
                    }
                }

                // Deduplicate.
                let mut seen = std::collections::HashSet::new();
                indices.retain(|i| seen.insert(*i));

                // Planeswalker attacks: (attacker index, pw index) pairs. An
                // attacker named here must not also attack the player.
                let mut walker_attacks: Vec<(mtg_engine::ids::ObjectId, mtg_engine::ids::ObjectId)> =
                    response["planeswalker_attacks"].as_array().map(|arr| arr.iter()
                        .filter_map(|v| {
                            let index = |k: &str| v[k].as_u64().and_then(|n| usize::try_from(n).ok());
                            match (index("attacker"), index("planeswalker")) {
                                (Some(a), Some(w)) if a < eligible.len() && w < defending_planeswalkers.len() =>
                                    Some((eligible[a], defending_planeswalkers[w])),
                                _ => { dropped.push(v.to_string()); None }
                            }
                        })
                        .collect()).unwrap_or_default();
                if !dropped.is_empty() {
                    self.log_rejected(&format!(
                        "attack answer names {} outside the {} attacker(s) and {} planeswalker(s) \
offered; {} not declared",
                        dropped.join(", "), eligible.len(), defending_planeswalkers.len(),
                        if dropped.len() == 1 { "it is" } else { "they are" }));
                }
                // A self-contradictory answer is read one way and said so:
                // the walker entry is the specific one ("instead of putting
                // its index in attacker_indices"), and a creature sent at two
                // walkers attacks the first, as the engine would keep it.
                // Both used to be decided in silence (issue #663).
                let mut at_walker = std::collections::HashSet::new();
                let mut twice: Vec<String> = Vec::new();
                walker_attacks.retain(|&(a, _)| at_walker.insert(a) || {
                    twice.push(Self::obj_label(view, a));
                    false
                });
                let both: Vec<String> = answered.iter()
                    .map(|&i| eligible[i])
                    .filter(|a| at_walker.contains(a))
                    .map(|a| Self::obj_label(view, a))
                    .collect();
                if !both.is_empty() {
                    self.log_rejected(&format!(
                        "attack answer sends {} both at the player and at a planeswalker; \
attacking the planeswalker", both.join(", ")));
                }
                if !twice.is_empty() {
                    self.log_rejected(&format!(
                        "attack answer sends {} at more than one planeswalker; attacking the first",
                        twice.join(", ")));
                }
                indices.retain(|&i| !at_walker.contains(&eligible[i]));

                let attackers = indices.iter()
                    .map(|&i| (eligible[i], *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers, planeswalker_attacks: walker_attacks }
            }

            CombatPrompt::ChooseBlockers { eligible_blockers, attackers, legal_blocks, min_blockers } => {
                self.choose_blockers_structured(view, eligible_blockers, attackers, legal_blocks, min_blockers)
            }
        }
    }

    /// Validate blocker assignments and return a list of error messages.
    /// Returns an empty vec if assignments are valid.
    fn validate_blocker_assignments(
        view: &GameView,
        assignments: &[(ObjectId, ObjectId)],
        attackers: &[ObjectId],
        min_blockers: &std::collections::HashMap<ObjectId, u32>,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // CR 509.1b: an attacker that can't be blocked by fewer than N
        // creatures needs N+ blockers or none. The engine's `min_blockers`
        // map is authoritative — a keyword scan here missed the
        // `MinimumBlockers` continuous effects (Terror of Kruin Pass,
        // issue #72).
        let mut blocker_counts: std::collections::HashMap<ObjectId, Vec<ObjectId>> = std::collections::HashMap::new();
        for &(blocker, attacker) in assignments {
            blocker_counts.entry(attacker).or_default().push(blocker);
        }
        for (att_idx, &attacker_id) in attackers.iter().enumerate() {
            let Some(&min) = min_blockers.get(&attacker_id) else { continue };
            if let Some(blockers) = blocker_counts.get(&attacker_id) {
                if !blockers.is_empty() && blockers.len() < min as usize {
                    errors.push(format!(
                        "Attacker {} ({}) can't be blocked by fewer than {} creatures, but you only assigned {}. Either assign more blockers to it or leave those blockers out (don't block it at all).",
                        att_idx, Self::format_combat_creature(view, attacker_id),
                        min, blockers.len()
                    ));
                }
            }
        }

        errors
    }

    /// Declare blockers using structured output with per-blocker integer enum
    /// constraints and a validation retry loop.
    /// The declare-blockers answer's schema: one array of
    /// `{blocker, attacker}` index pairs, each index an `enum` of the
    /// offered ones.
    ///
    /// It used to be one required property per blocker, each an `enum` of
    /// the attackers that blocker could block — |blockers| × |attackers|
    /// values, 95,000 (0.48 MB, ~190k tokens) at 1,000 permanents and 2 MB
    /// at 2,000 (#642). This grows with the two lists, not their product;
    /// which pairs the board allows is said in the prompt, grouped, and
    /// checked on the way back.
    fn blockers_schema(blockers: usize, attackers: usize) -> serde_json::Value {
        let blocker_enum: Vec<usize> = (0..blockers).collect();
        let attacker_enum: Vec<usize> = (0..attackers).collect();
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
                "blocks": {
                    "type": "array",
                    "maxItems": blockers,
                    "items": {
                        "type": "object",
                        "properties": {
                            "blocker": {"type": "integer", "enum": blocker_enum},
                            "attacker": {"type": "integer", "enum": attacker_enum}
                        },
                        "required": ["blocker", "attacker"]
                    },
                    "description": "One entry per blocking creature (blocker index + attacker index); empty for no blocks"
                }
            },
            "required": ["thoughts", "blocks"]
        })
    }

    /// Which attackers each blocker may block, grouped by the set it may
    /// block so the text is one line per distinct set rather than one entry
    /// per pair. Nothing when every blocker may block every attacker.
    fn block_reach_text(
        blockers: &[ObjectId],
        attackers: &[ObjectId],
        legal_blocks: &std::collections::HashMap<ObjectId, Vec<ObjectId>>,
    ) -> String {
        let mut groups: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
        for (bi, b) in blockers.iter().enumerate() {
            let can: Vec<usize> = attackers.iter().enumerate()
                .filter(|(_, a)| legal_blocks.get(b).is_some_and(|l| l.contains(a)))
                .map(|(ai, _)| ai)
                .collect();
            match groups.iter_mut().find(|(set, _)| *set == can) {
                Some((_, members)) => members.push(bi),
                None => groups.push((can, vec![bi])),
            }
        }
        if groups.iter().all(|(set, _)| set.len() == attackers.len()) {
            return String::new();
        }
        let list = |v: &[usize]| v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
        let mut out = String::from("\nWhat each blocker can block:");
        for (set, members) in &groups {
            let what = if set.len() == attackers.len() {
                "any attacker".to_string()
            } else if set.is_empty() {
                "no attacker".to_string()
            } else {
                format!("attackers {}", list(set))
            };
            write!(out, "\n  blockers {}: {what}", list(members)).unwrap();
        }
        out
    }

    /// The `blocks` answer as assignments, and a message for every pair the
    /// board does not allow — an index out of range, a blocker that cannot
    /// block that attacker, or a blocker named twice.
    fn parse_blocks(
        response: &serde_json::Value,
        blockers: &[ObjectId],
        attackers: &[ObjectId],
        legal_blocks: &std::collections::HashMap<ObjectId, Vec<ObjectId>>,
    ) -> (Vec<(ObjectId, ObjectId)>, Vec<String>) {
        let mut assignments: Vec<(ObjectId, ObjectId)> = Vec::new();
        let mut errors = Vec::new();
        for entry in response["blocks"].as_array().into_iter().flatten() {
            let index = |k: &str| entry[k].as_u64().and_then(|n| usize::try_from(n).ok());
            let (Some(bi), Some(ai)) = (index("blocker"), index("attacker")) else {
                errors.push(format!("{entry} is not a {{blocker, attacker}} pair of indices"));
                continue;
            };
            let (Some(&b), Some(&a)) = (blockers.get(bi), attackers.get(ai)) else {
                errors.push(format!("blocker {bi} / attacker {ai}: no such creature in this prompt"));
                continue;
            };
            if !legal_blocks.get(&b).is_some_and(|l| l.contains(&a)) {
                errors.push(format!("blocker {bi} cannot block attacker {ai}"));
                continue;
            }
            if assignments.iter().any(|&(x, _)| x == b) {
                errors.push(format!("blocker {bi} is named twice; a creature blocks one attacker"));
                continue;
            }
            assignments.push((b, a));
        }
        (assignments, errors)
    }

    fn choose_blockers_structured(
        &mut self,
        view: &GameView,
        eligible_blockers: &[ObjectId],
        attackers: &[ObjectId],
        legal_blocks: &std::collections::HashMap<ObjectId, Vec<ObjectId>>,
        min_blockers: &std::collections::HashMap<ObjectId, u32>,
    ) -> Action {
        let schema = Self::blockers_schema(eligible_blockers.len(), attackers.len());

        // Build combat text for the prompt with disambiguated labels so the
        // model can tell apart two attackers/blockers that share a name.
        let attacker_labels = Self::format_combat_creature_list(view, attackers);
        let blocker_labels = Self::format_combat_creature_list(view, eligible_blockers);

        let mut combat_text = String::from("Attackers: ");
        for (i, &id) in attackers.iter().enumerate() {
            // The engine's `min_blockers` map, not a keyword scan: menace
            // and the MinimumBlockers effects (Terror of Kruin Pass) both
            // land here (issue #72).
            match min_blockers.get(&id) {
                Some(&min) => write!(combat_text,
                    "{}:{} (can't be blocked by fewer than {} creatures) ",
                    i, attacker_labels[i], min).unwrap(),
                None => write!(combat_text, "{}:{} ", i, attacker_labels[i]).unwrap(),
            }
        }
        combat_text.push_str("\nYour blockers: ");
        for (i, _id) in eligible_blockers.iter().enumerate() {
            write!(combat_text, "{}:{} ", i, blocker_labels[i]).unwrap();
        }
        combat_text.push_str(&Self::block_reach_text(eligible_blockers, attackers, legal_blocks));
        combat_text.push_str(
            "\nDeclare blocks as a list of {\"blocker\": <blocker index>, \"attacker\": <attacker index>} \
             pairs, at most one per blocker; a blocker you leave out does not block.");

        let base_prompt = self.build_prompt(view, &combat_text);

        // One corrective re-ask, and then the answer is repaired rather
        // than thrown away.
        //
        // This used to re-send the same prompt with the same schema twenty
        // times. Every attempt is a whole prompt (a real game's system
        // prompt is 28k-69k characters, re-sent verbatim, #464) and a new
        // turn on the same `--resume` session, so one declare-blockers
        // decision cost up to 20 billed calls and, at `CALL_TIMEOUT` x
        // `MAX_ATTEMPTS` apiece, hours of wall clock -- while both of the
        // program's other bounds, `max_actions` and the progress watchdog,
        // count DECISIONS and see all of it as one. Measured: 80 of one
        // game's 98 calls, and 85% of its prompt bytes, were retries of
        // four decisions (#496).
        //
        // And it could not converge. The constraint is a joint one over
        // several blockers -- "two or more on this attacker, or none" --
        // which a per-blocker `enum` cannot express, so attempt 20's schema
        // still offers the illegal value and is byte-identical to attempt
        // 1's. What the retry adds over the first answer is the error
        // message in the prompt; a seat that will act on that acts on it
        // once.
        const MAX_RETRIES: usize = 1;
        let mut retry_message: Option<String> = None;
        let mut last_assignments: Vec<(ObjectId, ObjectId)> = Vec::new();
        for attempt in 0..=MAX_RETRIES {
            let prompt = if let Some(ref msg) = retry_message {
                format!("{base_prompt}\n\nPREVIOUS RESPONSE WAS INVALID:\n{msg}\nPlease try again.")
            } else {
                base_prompt.clone()
            };

            let response = self.send_message_structured(&prompt, &schema);

            // Blocking with nobody is a real decision, and it is also what a
            // seat that gave no usable list gets — said once, as the attack
            // prompt says it, not re-asked (#399).
            if !response["blocks"].is_array() {
                self.log_rejected(&format!(
                    "no usable 'blocks' ({}); declaring no blockers", response["blocks"]));
                return Action::DeclareBlockers { assignments: Vec::new() };
            }

            // Parse response into assignments. A pair the board does not
            // allow is refused by name and kept out of the answer: the
            // schema bounds each index but, being O(blockers + attackers),
            // cannot say which pairs are legal (#642).
            let (assignments, mut errors) =
                Self::parse_blocks(&response, eligible_blockers, attackers, legal_blocks);

            // Validate.
            errors.extend(Self::validate_blocker_assignments(view, &assignments, attackers, min_blockers));
            if errors.is_empty() {
                return Action::DeclareBlockers { assignments };
            }

            self.log("BLOCKER_VALIDATION", &format!("attempt {} errors: {:?}", attempt + 1, errors));
            retry_message = Some(errors.join("\n"));
            last_assignments = assignments;
        }

        // Keep every block the rules allow and drop only the pairs CR
        // 509.1b refuses -- which is what the engine does when handed the
        // identical answer (#72), through the same partition. Declaring no
        // blocks at all discarded the legal blocks too, so a seat that
        // named four good blocks and one bad one ended up strictly worse
        // off for having answered than if it had said nothing.
        let (kept, dropped) = mtg_engine::combat::partition_under_minimum_blocks(
            &last_assignments,
            |attacker| min_blockers.get(&attacker).copied().unwrap_or(0));
        self.log_rejected(&format!(
            "blocker assignments still invalid after {} attempts; keeping {} legal \
             block(s) and dropping {} under-minimum one(s), as the engine would",
            MAX_RETRIES + 1, kept.len(), dropped.len()));
        Action::DeclareBlockers { assignments: kept }
    }
}

// Crate-visible for the fixtures `surface_parity.rs` replays against both
// surfaces.
/// The base form of `word` when it is a third-person-singular present
/// verb ("reveals" → "reveal", "has" → "have", "searches" → "search",
/// "copies" → "copy"), or None when it does not read as one. The engine
/// writes its player-subject lines as "p0 <verb>s"; a reader's own lines
/// say "you <verb>", whatever the verb (#737). The page's `baseForm` in
/// mtg-gui/src/prompts.ts is the same rule.
pub(crate) fn base_form(word: &str) -> Option<String> {
    const NOT_VERBS: &[&str] = &["its", "this", "his", "hers", "us", "plus", "thus", "yes", "always", "less"];
    match word {
        "has" => return Some("have".into()),
        "is" => return Some("are".into()),
        "was" => return Some("were".into()),
        "does" => return Some("do".into()),
        _ => {}
    }
    if word.len() < 3 || !word.ends_with('s') || word.ends_with("ss") || NOT_VERBS.contains(&word) {
        return None;
    }
    if word.len() > 4 && word.ends_with("ies") {
        return Some(format!("{}y", &word[..word.len() - 3]));
    }
    if ["sses", "shes", "ches", "xes", "zes", "oes"].iter().any(|e| word.ends_with(e)) {
        return Some(word[..word.len() - 2].to_string());
    }
    Some(word[..word.len() - 1].to_string())
}

/// Whether the text before a player token ends with a preposition, which
/// makes the token an object ("deals 3 damage to p0"), so the word after
/// it is not its verb.
pub(crate) fn follows_preposition(before: &str) -> bool {
    const PREPOSITIONS: &[&str] = &["to", "from", "at", "of", "by", "for", "with", "against", "than", "target"];
    let last = before.trim_end().rsplit(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or("");
    before.ends_with(' ') && PREPOSITIONS.contains(&last.to_ascii_lowercase().as_str())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// #911's review: an on-or-off setting is on only for `1`; anything
    /// else is off, and a value it does not know is said rather than
    /// silently taken as off (`true` or `yes` stripped what was asked for).
    #[test]
    fn an_on_or_off_setting_is_on_for_1_alone() {
        const VAR: &str = "MTG_TEST_ENV_FLAG_911";
        for (value, on) in [("1", true), (" 1 ", true), ("0", false), ("", false), ("yes", false), ("true", false)] {
            std::env::set_var(VAR, value);
            assert_eq!(env_flag(VAR), on, "{value:?}");
        }
        std::env::remove_var(VAR);
        assert!(!env_flag(VAR));
    }
    use mtg_engine::cards::CardRegistry;
    use mtg_engine::types::CounterType;
    use std::collections::HashMap;

    // ── The prompt-format contract (issue #201) ──────────────────────────
    //
    // GAME_RULES tells the model what a prompt looks like, and the system
    // prompt then demands it ground every claim in that prompt's text. Eight
    // documented facts had drifted away from the formatters — inverted
    // section order, obsolete step names, single-line boards and hands, a
    // space-separated action list that is comma-separated, mulligan context
    // markers that were never emitted. These build the documented shapes
    // through the same code that sends them, so the two cannot drift apart
    // again without a test failing.

    /// A two-player game far enough along to have a board, a hand and a
    /// graveyard, so `format_state_body` prints the sections the contract
    /// describes.
    fn view_for_contract_test() -> (mtg_engine::state::GameState, CardRegistry) {
        use mtg_engine::engine::{setup_game, Decklist, GameConfig};
        let registry = CardRegistry::with_all_cards();
        let deck = Decklist {
            entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)],
        };
        let config = GameConfig {
            player_names: vec!["you".into(), "opp".into()],
            decklists: vec![deck.clone(), deck],
            starting_life: 20,
            starting_player: Some(mtg_engine::ids::PlayerId(0)),
            rng_seed: Some(7),
        };
        let state = setup_game(&config, &registry);
        (state, registry)
    }

    /// #490: the target-set prompt numbered every option twice —
    /// `0: 0: Abbey Griffin` — because the labels still carried the index
    /// prefix they needed back when a label was a schema key, and
    /// `mark_indices` numbers the list itself.
    #[test]
    fn a_marked_list_is_numbered_once() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let opponent = mtg_engine::ids::PlayerId(1);
        let a_card = state
            .objects
            .values()
            .find(|o| o.owner == view.you)
            .map(|o| o.id)
            .expect("the game has objects");

        let options = vec![
            mtg_engine::actions::Target::Player(view.you),
            mtg_engine::actions::Target::Player(opponent),
            mtg_engine::actions::Target::Object(a_card),
        ];
        let labels = LlmPlayer::target_labels(&view, &options);
        assert_eq!(labels[0], "You");
        assert_eq!(labels[1], "Opponent");

        let listing = LlmPlayer::numbered_listing(&labels);
        let mut lines = listing.lines();
        assert_eq!(lines.next(), Some("0: You"));
        assert_eq!(lines.next(), Some("1: Opponent"));
        for line in listing.lines() {
            let (index, rest) = line.split_once(": ").expect("every row is numbered");
            assert!(index.chars().all(|c| c.is_ascii_digit()), "{line}");
            assert!(
                !rest.split_once(": ").is_some_and(|(second, _)| {
                    !second.is_empty() && second.chars().all(|c| c.is_ascii_digit())
                }),
                "row numbered twice: {line}"
            );
        }
    }

    /// Every step name the header can print is in the documented list.
    #[test]
    fn game_rules_documents_every_step_name_it_prints() {
        let all_steps = [
            Step::Untap, Step::Upkeep, Step::Draw, Step::PrecombatMain,
            Step::BeginCombat, Step::DeclareAttackers, Step::DeclareBlockers,
            Step::CombatDamage, Step::EndCombat, Step::PostcombatMain,
            Step::EndStep, Step::Cleanup,
        ];
        for step in all_steps {
            for first_strike in [false, true] {
                let name = LlmPlayer::step_name(step, first_strike);
                assert!(
                    GAME_RULES.contains(name),
                    "the header prints {name:?} for {step:?}, but GAME_RULES never mentions it"
                );
            }
        }
    }

    /// The action list in GAME_RULES is the one `choose_action` builds.
    #[test]
    fn game_rules_shows_the_action_list_it_actually_sends() {
        let labels: Vec<ActionRow> = [
            "Pass", "Tap Forest: Add {G}", "Play Forest",
            "Cast Kalonian Tusker (tap 2x Forest)", "Concede",
        ]
        .iter()
        .map(|s| ActionRow::One((*s).to_string()))
        .collect();
        // The rules quote one row of the list, the shared one; the list's
        // other rows are read off the prompt itself.

        // And the row copies of one permanent share (issue #461): six
        // single rows first, so the copies' indices start at 5 as documented.
        let mut rows = labels;
        rows.push(ActionRow::Copies {
            label: "Activate Ludevic's Test Subject ({1}{U}: Put a hatchling counter. At 5, transform.) (tap 2x Island)".to_string(),
            ids: vec![ObjectId(43), ObjectId(45), ObjectId(46)],
        });
        let with_copies = LlmPlayer::format_action_prompt(Some("MAIN PHASE 1"), &rows, mtg_engine::ids::PlayerId(0));
        let copies_line = with_copies.lines().last().expect("the copies row is last");
        assert!(copies_line.starts_with("5-7: "), "{with_copies}");
        assert!(
            GAME_RULES.contains(copies_line),
            "GAME_RULES must quote the shared row the harness sends. It sends:\n{copies_line}"
        );
    }

    /// #755: the rules for the pass-until row are sent to a seat that is
    /// offered it, and to no other — with the knob off they told the seat
    /// index 1 was the row when index 1 was an engine action. And they say
    /// what the menu then sends: the row at 1, the engine's rows from 2.
    #[test]
    fn the_pass_until_rules_go_only_to_a_seat_offered_the_row() {
        let registry = mtg_engine::cards::CardRegistry::default();
        for offered in [true, false] {
            let mut p = LlmPlayer::for_prompt_tests("Seat-755").with_pass_until(offered);
            p.init_conversation(&[], "", &registry, MatchFormat::SingleGame);
            let system = p.backend.system_prompt().to_string();
            assert_eq!(system.contains(PASS_UNTIL_ROW), offered,
                "pass-until offered = {offered}, and the system prompt says:\n{system}");
        }
        let labels: Vec<ActionRow> = ["Pass", PASS_UNTIL_ROW, "Tap Forest: Add {G}"].iter()
            .map(|s| ActionRow::One((*s).to_string())).collect();
        let menu = LlmPlayer::format_action_prompt(Some("MAIN PHASE 1"), &labels, mtg_engine::ids::PlayerId(0));
        assert!(menu.contains(&format!("\n1: {PASS_UNTIL_ROW}\n2: Tap Forest: Add {{G}}")), "{menu}");
        assert!(PASS_UNTIL_RULES.contains(&format!("`1: {PASS_UNTIL_ROW}`")) && PASS_UNTIL_RULES.contains("numbered from 2"));
    }

    /// #756: what the rules say ends a pass-until is what does. Every stop
    /// line `pass_until_stop` can write has its clause in the rules, and the
    /// stack clause is not "the opponent's" alone — the seat's own triggers
    /// stop it too.
    #[test]
    fn the_pass_until_rules_name_every_stop_that_is_applied() {
        for clause in [
            "something of the opponent's is put on the stack",
            "a blocker is declared against your attacker",
            "the opponent attacks you and blocks are in",
            "your next first main phase",
            "a later main phase of yours with a land drop or a sorcery-speed cast or activation on offer",
            "any prompt that is not a plain priority pass",
            "After you declare attackers you pass this way too",
            "You are never asked to respond to your own spell or ability while only yours are on the stack",
        ] {
            assert!(PASS_UNTIL_RULES.contains(clause), "the rules do not say {clause:?}");
        }
        // Stops that were dropped are not claimed.
        for gone in ["your turn begins", "attackers are declared against you"] {
            assert!(!PASS_UNTIL_RULES.contains(gone), "the rules still claim {gone:?}");
        }
        assert_eq!(PASS_UNTIL_ROW, "Pass until something happens",
            "the row names itself; the one list of stops is the system prompt's");
    }

    /// Any `p<N>` still in the text, which is the token this seat is never
    /// taught: `GAME_RULES` defines neither `p0` nor `p1`, and no prompt
    /// tells a seat which one it is (the CLI's header does, #115).
    fn raw_player_tokens(text: &str) -> Vec<String> {
        let b = text.as_bytes();
        (0..b.len())
            .filter(|&i| b[i] == b'p' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
            .filter(|&i| i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
            .map(|i| text[i..].chars().take(4).collect())
            .collect()
    }

    /// Every engine-produced `p<N>` reaching this seat is rewritten to the
    /// seat's own vocabulary — log entries, the resolution description, the
    /// context line (#465) — and the action rows were the one section no
    /// rewrite touched. The CR 616.1 prompt's own options said `instead p1
    /// mills 4 cards` under a context line saying the damage was "to you",
    /// so the one fact that inverts the decision was carried by a token
    /// nothing defines: a seat reading `p1` as its opponent picks the mill
    /// and empties its own library (#543).
    #[test]
    fn an_action_row_never_hands_the_seat_a_raw_player_token() {
        // Verbatim as `Undead Alchemist::replacement_offer` builds it
        // (`mtg-engine/src/cards/isd/undead_alchemist.rs`), and as a shared
        // copies row, which is the other shape a row can take.
        let rows = vec![
            ActionRow::One("Inquisitor's Flail (#11) on Undead Alchemist (#3): double it to 8".to_string()),
            ActionRow::One("Undead Alchemist (#3): instead p1 mills 4 cards".to_string()),
            ActionRow::Copies {
                label: "Undead Alchemist: instead p1 mills 4 cards".to_string(),
                ids: vec![ObjectId(3), ObjectId(4)],
            },
        ];
        for you in [0u8, 1] {
            let prompt = LlmPlayer::format_action_prompt(
                Some("Undead Alchemist (#3) would deal 4 combat damage to p1"),
                &rows,
                mtg_engine::ids::PlayerId(you),
            );
            assert!(raw_player_tokens(&prompt).is_empty(),
                "p{you} is offered undefined tokens {:?} in:\n{prompt}",
                raw_player_tokens(&prompt));
        }

        // And it is the right player, conjugated: the affected seat mills.
        let mine = LlmPlayer::format_action_prompt(
            None, &rows, mtg_engine::ids::PlayerId(1));
        assert!(mine.contains("instead You mill 4 cards"), "{mine}");
        let theirs = LlmPlayer::format_action_prompt(
            None, &rows, mtg_engine::ids::PlayerId(0));
        assert!(theirs.contains("instead Opp mills 4 cards"), "{theirs}");
    }

    /// #834: a trigger's cause is engine text, and the ordering rows passed
    /// it through raw — "triggered by: Rakish Heir (#39) dealt 2 combat
    /// damage to p1", to a seat that has never been told who `p1` is.
    #[test]
    fn an_ordering_row_never_hands_the_seat_a_raw_player_token() {
        let rows = vec![
            "Rakish Heir (#39) 2/2 — triggered ability: put a +1/+1 counter on that Vampire — triggered by: Rakish Heir (#39) dealt 2 combat damage to p1".to_string(),
            "Stromkirk Noble (#40) 1/1 — combat damage trigger: put a +1/+1 counter on Stromkirk Noble — triggered by: it dealt 1 combat damage to p1".to_string(),
        ];
        for you in [0u8, 1] {
            let text = LlmPlayer::ordering_text("p0: choose the order of 2 triggers", &rows, "rule",
                mtg_engine::ids::PlayerId(you));
            assert!(raw_player_tokens(&text).is_empty(),
                "p{you} is offered undefined tokens {:?} in:\n{text}", raw_player_tokens(&text));
        }
        let mine = LlmPlayer::ordering_text("", &rows, "rule", mtg_engine::ids::PlayerId(0));
        assert!(mine.contains("dealt 2 combat damage to Opp"), "{mine}");
    }

    /// #858: with a history window the seat resumes its last exchanges, and
    /// its system prompt still said no earlier prompt is kept. The default
    /// is the same bytes; a window says what is kept.
    #[test]
    fn the_rules_say_what_the_history_window_keeps() {
        assert!(GAME_RULES.contains(NO_HISTORY_KEPT), "the sentence the window replaces is in the rules");
        assert_eq!(game_rules(0), GAME_RULES);
        let kept = game_rules(2);
        assert!(!kept.contains("No earlier prompt is kept"), "{kept}");
        assert!(kept.contains("Up to 2 of your earlier decisions are also kept"), "{kept}");
    }

    /// The system prompt had been written around the leak rather than
    /// against it — `GAME_RULES` documented the expected option as `instead
    /// p1 mills 2 cards`, so the contract taught a token the same contract
    /// never explains. It has to quote what the harness now sends (#201's
    /// lesson, #543's line).
    #[test]
    fn game_rules_quotes_the_damage_option_as_the_harness_rewrites_it() {
        let sent = LlmPlayer::format_action_prompt(
            None,
            &[ActionRow::One("Undead Alchemist (#3): instead p1 mills 2 cards".to_string())],
            mtg_engine::ids::PlayerId(1),
        );
        let option = sent.lines()
            .find_map(|l| l.strip_prefix("0: "))
            .expect("the row is printed");
        let effect = option.split_once(": ").expect("`Name (#id): what it does`").1;
        assert_eq!(effect, "instead You mill 2 cards", "the option is rewritten for the seat");
        for text in [GAME_RULES, DAMAGE_EFFECT_RULE] {
            assert!(raw_player_tokens(text).is_empty(),
                "the rules must not teach a token they never define: {:?}", raw_player_tokens(text));
        }
    }

    /// The header comes first and "Recent events" follows it, which is the
    /// opposite of what the contract used to claim.
    #[test]
    fn a_prompt_leads_with_the_header_then_recent_events() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let mut player = LlmPlayer::for_prompt_tests("test");
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");

        let header = prompt.find("Turn ").expect("every prompt has a header line");
        if let Some(events) = prompt.find("Recent events:") {
            assert!(header < events, "header comes first:\n{prompt}");
        }
        let doc_header = GAME_RULES.find("`Turn N").expect("documented");
        let doc_events = GAME_RULES.find("`Recent events:`").expect("documented");
        assert!(
            doc_header < doc_events,
            "GAME_RULES lists the sections in the order they are sent"
        );
    }

    /// Boards, hands and stacks are headers with indented entries, not the
    /// single comma-separated lines the contract used to show.
    #[test]
    fn game_rules_shows_the_board_and_hand_shape_it_actually_sends() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let body = LlmPlayer::format_state_body(&view);

        assert!(body.contains("Hand:\n  "), "the hand is a header plus indented cards:\n{body}");
        assert!(
            !body.contains("Hand: Forest"),
            "the hand is not a single inline list:\n{body}"
        );
        for section in ["`Your board:`", "`Hand:`", "`Stack:`"] {
            assert!(
                GAME_RULES.contains(section),
                "GAME_RULES must name {section:?}, the heading the harness sends"
            );
        }
    }

    /// The CR 616.1 prompt is a flat numbered choice like any other, so the
    /// model needs to be told what it is choosing and what "first" means
    /// (issue #323).
    #[test]
    fn the_damage_effect_choice_is_documented() {
        for phrase in ["CR 616.1", "the affected player", "apply FIRST", "a prevention or a mill ends the damage",
                       "doubling leaves more"] {
            assert!(DAMAGE_EFFECT_RULE.contains(phrase), "the rule sent with the prompt explains {phrase:?}");
        }
    }

    /// Both mulligan prompts carry the context markers GAME_RULES documents.
    #[test]
    fn mulligan_prompts_carry_the_documented_context_markers() {
        let hand = "  0: Forest\n  1: Grizzly Bears {1}{G} 2/2\n";
        let keep = LlmPlayer::mulligan_prompt("You are on the draw", 0, 7, "", hand);
        assert!(
            keep.starts_with("[MULLIGAN DECISION]"),
            "the keep-or-mulligan prompt leads with its context marker:\n{keep}"
        );
        assert!(keep.contains(MULLIGAN_ADVICE), "the advice comes with the question:\n{keep}");

        let bottom = LlmPlayer::mulligan_bottom_prompt("You are on the draw", 2, "", hand);
        assert!(
            bottom.starts_with("[BOTTOM 2 CARD(S) AFTER MULLIGAN]"),
            "the bottoming prompt leads with its context marker:\n{bottom}"
        );
        let one = LlmPlayer::mulligan_bottom_prompt("You are on the play", 1, "", hand);
        assert!(
            one.starts_with("[BOTTOM 1 CARD AFTER MULLIGAN]"),
            "a single card drops the (S):\n{one}"
        );
    }

    // ── The concede confirmation (issue #209) ────────────────────────────

    /// A backend that answers from a script and remembers what it was asked.
    #[derive(Default)]
    struct ScriptedBackend {
        answers: Vec<serde_json::Value>,
        prompts: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
    }

    impl LlmBackend for ScriptedBackend {
        fn send(&mut self, message: &str) -> String {
            self.send_with_schema(message, &serde_json::Value::Null).to_string()
        }
        fn send_with_schema(&mut self, message: &str, _schema: &serde_json::Value) -> serde_json::Value {
            self.prompts.borrow_mut().push(message.to_string());
            if self.answers.is_empty() {
                return serde_json::Value::Null;
            }
            self.answers.remove(0)
        }
        fn init(&mut self, _deck_info: &str) {}
        fn conversation_len(&self) -> usize { self.prompts.borrow().len() }
        fn system_prompt(&self) -> &str { "" }
        fn model_name(&self) -> &str { "scripted" }
    }

    /// A priority offer whose display list collapses: two ways to cast one
    /// spell become one option, so Concede's display index (2) is not its
    /// index in `legal_actions` (3).
    fn collapsing_priority_offer() -> mtg_engine::engine::LegalActions {
        use mtg_engine::actions::{CastTargetSpec, CastableSpell};
        use mtg_engine::ids::ObjectId;

        let spell = ObjectId(41);
        let cast = |targets: Vec<mtg_engine::actions::Target>| Action::CastSpell {
            object_id: spell,
            targets,
            sacrifice: None,
            exile_count: None,
            exile_ids: Vec::new(),
            alternative_cost: None,
            tap_plan: Vec::new(),
        };
        mtg_engine::engine::LegalActions {
            actions: vec![
                Action::PassPriority,
                cast(vec![mtg_engine::actions::Target::Player(mtg_engine::ids::PlayerId(0))]),
                cast(vec![mtg_engine::actions::Target::Player(mtg_engine::ids::PlayerId(1))]),
                Action::Concede,
            ],
            combat_prompt: None,
            castable_spells: vec![CastableSpell {
                object_id: spell,
                name: "Geistflame".to_string(),
                is_flashback: false,
                from_graveyard: false,
                target_spec: CastTargetSpec::SingleTarget(vec![
                    mtg_engine::actions::Target::Player(mtg_engine::ids::PlayerId(0)),
                    mtg_engine::actions::Target::Player(mtg_engine::ids::PlayerId(1)),
                ]),
                tap_plan: Vec::new(),
                exile_x_from_gy_max: None,
                sacrifice_options: Vec::new(),
                additional_cost_label: None,
                alternative_cost: None,
            }],
            activatable_abilities: Vec::new(),
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        }
    }

    fn scripted_player(answers: Vec<serde_json::Value>) -> (LlmPlayer, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
        let prompts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let player = LlmPlayer {
            name: "t".to_string(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(ScriptedBackend { answers, prompts: std::rc::Rc::clone(&prompts) }),
            provider: Provider::Anthropic,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            // These tests read the engine's rows and their indices; the
            // seat's own pass-until row is tested on its own, below.
            offer_pass_until: false,
        };
        (player, prompts)
    }

    /// Issue #209: the guard tested the picked index against
    /// `legal_actions` while the index belonged to the collapsed display
    /// list, so the moment anything collapsed the confirmation was skipped
    /// and the game ended on an unconfirmed concede.
    #[test]
    fn a_concede_is_confirmed_even_when_the_action_list_collapsed() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let legal = collapsing_priority_offer();

        let (mut player, prompts) = scripted_player(vec![
            serde_json::json!({"action": 2}), // Concede, as displayed
            serde_json::json!({"confirm": false}), // ... then think better of it
        ]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        assert_eq!(
            asked.len(), 2,
            "the concede confirmation must be asked. Prompts: {asked:#?}"
        );
        assert!(
            asked[0].contains("0: Pass\n1: Cast Geistflame\n2: Concede"),
            "the display list really did collapse 4 actions to 3 options:\n{}", asked[0]
        );
        assert!(asked[1].contains("CONCEDE"), "the second prompt is the confirmation: {}", asked[1]);
        assert!(matches!(chosen, Action::PassPriority), "a cancelled concede passes instead, got {chosen:?}");
    }

    /// Issues #830 and #831: the mulligan, bottom-after-mulligan and
    /// concede-confirmation prompts were built outside `build_prompt`, so
    /// they carried no notes: a seat that said "keep and bottom Falkenrath
    /// Marauders" bottomed something else, and the concede was confirmed
    /// with no board and not the reasoning that chose it.
    #[test]
    fn the_mulligan_bottom_and_concede_prompts_carry_the_notes() {
        use mtg_engine::actions::{SetPrompt, SetPromptKind};
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let plan = "keep, and bottom Falkenrath Marauders";

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"mull": false})]);
        player.notes = Some(plan.to_string());
        player.choose_mulligan(&view, &[Action::MulliganKeep, Action::MulliganMull]);
        let keep = prompts.borrow()[0].clone();
        assert!(keep.starts_with("[MULLIGAN DECISION]\n"), "the marker still leads:\n{keep}");
        assert!(keep.contains(&format!("Your notes from your last decision:\n{plan}")), "{keep}");

        let hand: Vec<ObjectId> = view.your_hand.iter().map(|c| c.object_id).collect();
        assert!(!hand.is_empty(), "the contract view has a hand to bottom from");
        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"card_indices": [0]})]);
        player.notes = Some(plan.to_string());
        player.choose_card_set(&view, &SetPrompt {
            kind: SetPromptKind::BottomAfterMulligan, player: view.you,
            options: hand, min: 1, max: 1,
        });
        let bottom = prompts.borrow()[0].clone();
        assert!(bottom.starts_with("[BOTTOM 1 CARD AFTER MULLIGAN]\n"), "{bottom}");
        assert!(bottom.contains(&format!("Your notes from your last decision:\n{plan}")), "{bottom}");

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"confirm": false})]);
        player.notes = Some("Concede: no outs against lethal next turn.".to_string());
        assert!(!player.confirm_concede(&view));
        let confirm = prompts.borrow()[0].clone();
        assert!(confirm.contains("Your notes from your last decision:\nConcede: no outs"), "{confirm}");
        assert!(confirm.contains("You: "), "the board is in front of the question:\n{confirm}");
        assert!(confirm.contains("[CONFIRM CONCEDE]"), "{confirm}");

        // The next game of a match starts with no notes: the plan was for
        // the last game's board.
        player.init_conversation(&[], "", &registry, MatchFormat::SingleGame);
        assert!(player.notes_section().is_empty(), "a new game's first prompt has no notes");
    }

    /// Issue #637: the division prompt is asked as the amount, and the
    /// answer goes back as the prompt's index (`amount - min`). An answer
    /// outside the range is refused and lethal — the engine's own default —
    /// is assigned instead, with a REJECTED line.
    #[test]
    fn a_combat_damage_division_is_answered_as_the_amount() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let options: Vec<String> = (2..=5).map(|a| format!("{a} to Bear")).collect();
        let legal = mtg_engine::engine::LegalActions {
            actions: options.iter().enumerate().map(|(i, o)| Action::ResolveChoice {
                choice: mtg_engine::actions::ResolvedChoice::ChosenIndex(i, o.clone()) }).collect(),
            combat_prompt: None,
            castable_spells: Vec::new(),
            activatable_abilities: Vec::new(),
            context: Some("Combat damage from Boar".to_string()),
            resolution_prompt: Some(mtg_engine::state::ResolutionChoiceKind::AssignCombatDamage {
                description: "Combat damage from Boar (5 power, CR 510.1c): how much goes to Bear?".into(),
                attacker: ObjectId(1), blocker: ObjectId(2), min: 2, max: 5,
                options: options.clone(), first_strike_only: false,
            }),
            set_prompt: None,
        };

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"amount": 4})]);
        match player.choose_action(&view, &legal) {
            Action::ResolveChoice { choice: mtg_engine::actions::ResolvedChoice::ChosenIndex(i, label) } => {
                assert_eq!((i, label.as_str()), (2, "4 to Bear"), "amount 4 is index 2 of 2..=5");
            }
            other => panic!("expected the division's index, got {other:?}"),
        }
        let asked = prompts.borrow();
        assert_eq!(asked.len(), 1, "one question, not a menu walk");
        assert!(asked[0].contains("an integer from 2 to 5"), "{}", asked[0]);
        drop(asked);

        let (mut player, _) = scripted_player(vec![serde_json::json!({"amount": 9})]);
        assert!(matches!(player.choose_action(&view, &legal), Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenIndex(0, _) }),
            "an out-of-range amount assigns lethal");
    }

    /// And a confirmed concede still concedes.
    #[test]
    fn a_confirmed_concede_concedes() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let legal = collapsing_priority_offer();

        let (mut player, _prompts) = scripted_player(vec![
            serde_json::json!({"action": 2}),
            serde_json::json!({"confirm": true}),
        ]);
        assert!(matches!(player.choose_action(&view, &legal), Action::Concede));
    }

    /// A priority offer over three copies of one creature with one
    /// activated ability, another permanent with its own, Pass and Concede.
    /// The copies are not adjacent in the engine's order.
    fn copies_priority_offer() -> mtg_engine::engine::LegalActions {
        use mtg_engine::actions::{ActivatableAbility, ActivatableAbilityOption};
        let activate = |id: u64| Action::ActivateAbility {
            object_id: ObjectId(id),
            ability_index: 0,
            targets: Vec::new(),
            tap_plan: Vec::new(),
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        };
        let ability = |id: u64, name: &str, desc: &str| ActivatableAbility {
            object_id: ObjectId(id),
            ability_index: 0,
            source_card_id: None,
            name: format!("{name} (#{id})"),
            description: desc.to_string(),
            target_options: Vec::new(),
            tap_plan: Vec::new(),
            sorcery_speed: false,
            option_combos: vec![ActivatableAbilityOption { targets: Vec::new(), sacrifice: None }],
        };
        let hatch = "{1}{U}: Put a hatchling counter. At 5, transform.";
        mtg_engine::engine::LegalActions {
            actions: vec![
                Action::PassPriority,
                activate(43),
                activate(50),
                activate(45),
                activate(46),
                Action::Concede,
            ],
            combat_prompt: None,
            castable_spells: Vec::new(),
            activatable_abilities: vec![
                ability(43, "Ludevic's Test Subject", hatch),
                ability(45, "Ludevic's Test Subject", hatch),
                ability(46, "Ludevic's Test Subject", hatch),
                ability(50, "Civilized Scholar", "{T}: Draw a card, then discard a card."),
            ],
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        }
    }

    /// Issue #827: a cast or activation with one legal target is aimed
    /// without a follow-up prompt (CR 601.2c), so the row is the only place
    /// the seat can read where it goes. Brimstone Volley against Witchbane
    /// Orb read "Cast Brimstone Volley (tap 3x Mountain)" and hit its own
    /// caster seven times. A row with a choice left in it names nothing.
    #[test]
    fn a_row_with_one_legal_target_names_it() {
        use mtg_engine::actions::{ActivatableAbility, ActivatableAbilityOption,
            CastTargetSpec, CastableSpell, Target};
        let view = empty_view();
        let me = Target::Player(view.you);
        let opp = Target::Player(mtg_engine::ids::PlayerId(1 - view.you.0));
        let cast = |id: u64, t: Target| Action::CastSpell {
            object_id: ObjectId(id), targets: vec![t], sacrifice: None, exile_count: None,
            exile_ids: Vec::new(), alternative_cost: None, tap_plan: Vec::new(),
        };
        let spell = |id: u64, name: &str, options: Vec<Target>| CastableSpell {
            object_id: ObjectId(id), name: name.to_string(), is_flashback: false,
            from_graveyard: false, target_spec: CastTargetSpec::SingleTarget(options),
            tap_plan: Vec::new(), exile_x_from_gy_max: None, sacrifice_options: Vec::new(),
            additional_cost_label: None, alternative_cost: None,
        };
        let ability = |id: u64, targets: Vec<Vec<Target>>| ActivatableAbility {
            object_id: ObjectId(id), ability_index: 0, source_card_id: None,
            name: format!("Prodder (#{id})"), description: "{T}: 1 damage to any target.".into(),
            target_options: Vec::new(), tap_plan: Vec::new(), sorcery_speed: false,
            option_combos: targets.into_iter()
                .map(|t| ActivatableAbilityOption { targets: t, sacrifice: None }).collect(),
        };
        let activate = |id: u64, t: Target| Action::ActivateAbility {
            object_id: ObjectId(id), ability_index: 0, targets: vec![t], tap_plan: Vec::new(),
            sacrifice: None, x_value: None, source_card_id: None,
        };
        let legal = mtg_engine::engine::LegalActions {
            actions: vec![
                Action::PassPriority,
                cast(41, me.clone()),
                cast(42, me.clone()), cast(42, opp.clone()),
                activate(50, opp.clone()),
                activate(51, me.clone()), activate(51, opp.clone()),
                Action::Concede,
            ],
            combat_prompt: None,
            castable_spells: vec![
                spell(41, "Brimstone Volley", vec![me.clone()]),
                spell(42, "Geistflame", vec![me.clone(), opp.clone()]),
            ],
            activatable_abilities: vec![
                ability(50, vec![vec![opp.clone()]]),
                ability(51, vec![vec![me.clone()], vec![opp.clone()]]),
            ],
            context: Some("MAIN PHASE 2".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        };
        let (_, rows) = LlmPlayer::build_action_rows(&view, &legal);
        let text: Vec<String> = rows.iter().map(|r| match r {
            ActionRow::One(l) => l.clone(),
            ActionRow::Copies { label, .. } => label.clone(),
        }).collect();
        let row = |needle: &str| text.iter().find(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no row for {needle}: {text:#?}")).clone();
        assert!(row("Brimstone Volley").ends_with(" targeting you"),
            "the forced target is named: {text:#?}");
        assert!(!row("Geistflame").contains("targeting"),
            "a target still to choose is not: {text:#?}");
        assert!(row("(#50)").ends_with(" targeting opponent"),
            "nor is an ability's forced target left out: {text:#?}");
        assert!(!row("(#51)").contains("targeting"), "{text:#?}");
        assert!(GAME_RULES.contains("targeting you`"),
            "GAME_RULES says a row with one target names it");
    }

    /// Issue #461: N copies of one permanent with one activated ability
    /// were N rows differing only in their `(#id)`, comma-joined onto one
    /// unwrapped line with everything else. Copies share a row with an index
    /// per copy, every option is on its own line, and an index still names
    /// the copy it was shown as.
    #[test]
    fn copies_of_one_ability_share_a_row_and_each_index_names_its_copy() {
        let view = empty_view();
        let legal = copies_priority_offer();

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 2})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert_eq!(
            list,
            "Available actions:\n\
             0: Pass\n\
             1-3: Activate Ludevic's Test Subject ({1}{U}: Put a hatchling counter. At 5, transform.) — one per copy: 1=#43, 2=#45, 3=#46\n\
             4: Activate Civilized Scholar (#50) ({T}: Draw a card, then discard a card.)\n\
             5: Concede\n",
            "one row per line, copies grouped with the lone ability and Concede outside the group:\n{}", asked[0]
        );
        assert!(
            matches!(chosen, Action::ActivateAbility { object_id: ObjectId(45), .. }),
            "index 2 is the second copy, #45, not the engine's third action: {chosen:?}"
        );

        // The indices past the group still land on what they were shown as.
        let (mut player, _) = scripted_player(vec![serde_json::json!({"action": 4})]);
        let chosen = player.choose_action(&view, &legal);
        assert!(matches!(chosen, Action::ActivateAbility { object_id: ObjectId(50), .. }), "{chosen:?}");
        let (mut player, _) = scripted_player(vec![serde_json::json!({"action": 5}), serde_json::json!({"confirm": true})]);
        assert!(matches!(player.choose_action(&view, &legal), Action::Concede));
    }

    /// A priority offer over `copies` copies of one creature with a native
    /// index-0 ability, where the copies in `enchanted` also carry an
    /// Aura granting an index-0 ability described by `granted`.
    ///
    /// The engine collects native abilities before attached ones, which is
    /// why it is always the granted ability that a coarser key drops.
    pub(crate) fn granted_ability_offer(copies: &[u64], enchanted: &[u64], granted: &str)
        -> mtg_engine::engine::LegalActions
    {
        use mtg_engine::actions::{ActivatableAbility, ActivatableAbilityOption};
        const AURA: CardId = CardId(77);
        let native = "{G}: Regenerate";
        let activate = |id: u64, source: Option<CardId>| Action::ActivateAbility {
            object_id: ObjectId(id),
            ability_index: 0,
            targets: Vec::new(),
            tap_plan: Vec::new(),
            sacrifice: None,
            x_value: None,
            source_card_id: source,
        };
        let ability = |id: u64, source: Option<CardId>, desc: &str| ActivatableAbility {
            object_id: ObjectId(id),
            ability_index: 0,
            source_card_id: source,
            name: format!("Ulvenwald Mystics (#{id})"),
            description: desc.to_string(),
            target_options: Vec::new(),
            tap_plan: Vec::new(),
            sorcery_speed: false,
            option_combos: vec![ActivatableAbilityOption { targets: Vec::new(), sacrifice: None }],
        };
        let mut actions = vec![Action::PassPriority];
        let mut abilities = Vec::new();
        for &id in copies {
            actions.push(activate(id, None));
            abilities.push(ability(id, None, native));
        }
        for &id in enchanted {
            actions.push(activate(id, Some(AURA)));
            abilities.push(ability(id, Some(AURA), granted));
        }
        actions.push(Action::Concede);
        mtg_engine::engine::LegalActions {
            actions,
            combat_prompt: None,
            castable_spells: Vec::new(),
            activatable_abilities: abilities,
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        }
    }

    /// Issue #589: the seat collapsed `ActivateAbility` on
    /// `(object_id, ability_index)` while the engine keys the same offer on
    /// `(object_id, source_card_id, ability_index)` — the field whose whole
    /// job is to tell an Aura-granted ability from a native one. An Aura
    /// granting an ability at an index the host already uses therefore lost
    /// its row, always the granted one, because natives are collected first.
    ///
    /// Silently: `legal.actions` and `legal.activatable_abilities` agree, so
    /// the invariant checker passes; the CLI renders `legal.actions` and so
    /// cannot lose a row; the random seat picks from the flat list. Only the
    /// LLM seat, and only by never being offered the option. Measured at
    /// 718 of 718 menus in one game — and with Skeletal Grimace granting at
    /// index 0, that is where almost every creature's first ability lives.
    ///
    /// The two rows are not redundant: `{G}: Regenerate` and `{B}:
    /// Regenerate` are the same effect for different mana, and the seat can
    /// only ever pay one of them.
    #[test]
    fn an_aura_granted_ability_keeps_its_row_when_its_index_collides() {
        let view = empty_view();
        let legal = granted_ability_offer(&[2, 4, 6], &[2], "{B}: Regenerate");

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 4})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert_eq!(
            list,
            "Available actions:\n\
             0: Pass\n\
             1-3: Activate Ulvenwald Mystics ({G}: Regenerate) — one per copy: 1=#2, 2=#4, 3=#6\n\
             4: Activate Ulvenwald Mystics (#2) ({B}: Regenerate)\n\
             5: Concede\n",
            "the Aura-granted ability is a row of its own, on the one copy that has it:\n{}",
            asked[0]
        );

        // And the index resolves to the granted activation, not the native
        // one it collided with.
        assert!(
            matches!(chosen, Action::ActivateAbility {
                object_id: ObjectId(2), ability_index: 0, source_card_id: Some(CardId(77)), .. }),
            "index 4 is the granted ability on #2: {chosen:?}"
        );
    }

    /// Issue #612: a "one per copy" row is a claim about the board — n copies
    /// of one permanent, one index each, each told apart by its `#id`. Two
    /// activations of ONE permanent that render the same string are not
    /// copies: they are two ways of paying one ability's cost, and the row
    /// asserted a second permanent that is not there. Skirsdag High Priest's
    /// two-creature tap cost, with two Demon tokens on the board, produced
    /// `— one per copy: 4=#37, 5=#37` for the single Priest, giving the seat
    /// no way to tell index 4 from index 5.
    ///
    /// The engine-side fix stops the labels colliding; this is the surface
    /// refusing to state something false if one ever collides again.
    #[test]
    fn a_row_that_would_name_one_permanent_twice_is_not_a_copies_row() {
        use mtg_engine::actions::{ActivatableAbility, ActivatableAbilityOption};

        // One permanent, two activations, one description — the shape a card
        // that encodes its cost payment in `ability_index` produces when two
        // payments render alike.
        let same = "Morbid — {T}, Tap two creatures: Create a 5/5 Demon with flying (tap Demon & Demon)";
        let activate = |index: usize| Action::ActivateAbility {
            object_id: ObjectId(37),
            ability_index: index,
            targets: Vec::new(),
            tap_plan: Vec::new(),
            sacrifice: None,
            x_value: None,
            source_card_id: None,
        };
        let ability = |index: usize| ActivatableAbility {
            object_id: ObjectId(37),
            ability_index: index,
            source_card_id: None,
            name: "Skirsdag High Priest (#37)".to_string(),
            description: same.to_string(),
            target_options: Vec::new(),
            tap_plan: Vec::new(),
            sorcery_speed: false,
            option_combos: vec![ActivatableAbilityOption { targets: Vec::new(), sacrifice: None }],
        };
        let legal = mtg_engine::engine::LegalActions {
            actions: vec![Action::PassPriority, activate(0), activate(1), Action::Concede],
            combat_prompt: None,
            castable_spells: Vec::new(),
            activatable_abilities: vec![ability(0), ability(1)],
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        };

        let view = empty_view();
        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 2})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert!(!list.contains("one per copy"),
            "there is one #37 on the board, so no row may say the board holds \
             two of it:\n{}", asked[0]);
        assert_eq!(list.matches("Skirsdag High Priest").count(), 2,
            "both activations are still offered, one index each — a row that \
             cannot be told from its neighbour is still better than a row that \
             is gone:\n{}", asked[0]);
        assert!(matches!(chosen,
            Action::ActivateAbility { object_id: ObjectId(37), ability_index: 1, .. }),
            "and index 2 is the second activation, not the first again: {chosen:?}");
    }

    /// The other half of #589: restoring the row is not enough if the row
    /// then joins the wrong group. The copy grouping keys on the label with
    /// the `(#id)` stripped, so an Aura granting an ability that reads and
    /// costs exactly like the host's native one would collapse into the
    /// native group — listing one copy twice, and claiming the copies with
    /// no Aura offer it too.
    #[test]
    fn a_granted_ability_that_reads_like_the_native_one_is_its_own_group() {
        let view = empty_view();
        // #2 and #4 wear the Aura; #6 does not. The grant is word-for-word
        // the native ability.
        let legal = granted_ability_offer(&[2, 4, 6], &[2, 4], "{G}: Regenerate");

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 4})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert_eq!(
            list,
            "Available actions:\n\
             0: Pass\n\
             1-3: Activate Ulvenwald Mystics ({G}: Regenerate) — one per copy: 1=#2, 2=#4, 3=#6\n\
             4-5: Activate Ulvenwald Mystics ({G}: Regenerate) — one per copy: 4=#2, 5=#4\n\
             6: Concede\n",
            "the granted ability groups over the enchanted copies only, and \
             #6 — which has no Aura — is not in that group:\n{}",
            asked[0]
        );
        assert!(
            matches!(chosen, Action::ActivateAbility {
                object_id: ObjectId(2), source_card_id: Some(CardId(77)), .. }),
            "index 4 is the granted ability on #2: {chosen:?}"
        );
    }

    /// The cast arm of #589. The seat keyed a cast on
    /// `(object_id, alternative_cost.is_some())` while the engine keys it
    /// on `(object_id, alternative_cost)` — so two *different* alternative
    /// costs on one object collapsed into one row, and the seat could only
    /// ever cast it the first way the engine happened to list.
    ///
    /// No card in the current pool offers two, which is why nothing caught
    /// it and why this drives the collapse directly rather than a game.
    #[test]
    fn two_different_alternative_costs_on_one_spell_are_two_rows() {
        use mtg_engine::actions::{CastTargetSpec, CastableSpell};
        use mtg_engine::types::{Color, ManaCost, ManaSymbol};

        let cheap = ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]);
        let dear = ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Blue)]);
        let cast = |alt: &ManaCost| Action::CastSpell {
            object_id: ObjectId(8),
            targets: Vec::new(),
            tap_plan: Vec::new(),
            alternative_cost: Some(alt.clone()),
            exile_count: None,
            exile_ids: Vec::new(),
            sacrifice: None,
        };
        let castable = |alt: &ManaCost| CastableSpell {
            object_id: ObjectId(8),
            name: "Two-Headed Bargain".to_string(),
            is_flashback: false,
            target_spec: CastTargetSpec::NoTargets,
            tap_plan: Vec::new(),
            exile_x_from_gy_max: None,
            sacrifice_options: Vec::new(),
            additional_cost_label: None,
            alternative_cost: Some(alt.clone()),
            from_graveyard: false,
        };
        let legal = mtg_engine::engine::LegalActions {
            actions: vec![Action::PassPriority, cast(&cheap), cast(&dear), Action::Concede],
            combat_prompt: None,
            castable_spells: vec![castable(&cheap), castable(&dear)],
            activatable_abilities: Vec::new(),
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        };

        let view = empty_view();
        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 2})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert_eq!(
            list,
            "Available actions:\n\
             0: Pass\n\
             1: Cast Two-Headed Bargain (alternative cost {G})\n\
             2: Cast Two-Headed Bargain (alternative cost {4}{U})\n\
             3: Concede\n",
            "each alternative cost is its own way to cast the spell:\n{}", asked[0]
        );
        let Action::CastSpell { alternative_cost, .. } = chosen else {
            panic!("a cast row casts: {chosen:?}")
        };
        assert_eq!(alternative_cost, Some(dear), "index 2 is the second cost");
    }

    /// Issue #611: CR 702.33 lets one card in the graveyard carry several
    /// instances of flashback at once — Past in Flames grants one equal to the
    /// card's mana cost, alongside the printed one — and CR 601.2b makes which
    /// to pay the caster's choice. The row's cost note was gated on
    /// `!is_flashback`, so a flashback row never said what it charged and the
    /// only thing telling two of them apart was the tap plan. Once the mana is
    /// already in the pool there is no tap plan, so both rows rendered the
    /// byte-identical string `Flashback Geistflame` and `seen_cast_labels`
    /// dropped the second — a legal option gone, with `COLLAPSED` logging a
    /// count and not which row went.
    #[test]
    fn two_flashback_costs_on_one_card_stay_two_rows_with_the_mana_floating() {
        use mtg_engine::actions::{CastTargetSpec, CastableSpell};
        use mtg_engine::types::{Color, ManaCost, ManaSymbol};

        let granted = ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]);
        let printed = ManaCost::new(vec![
            ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)]);
        let cast = |alt: &ManaCost| Action::CastSpell {
            object_id: ObjectId(30),
            targets: Vec::new(),
            // The mana is already floating: nothing left to tap, which is
            // what used to make the two rows indistinguishable.
            tap_plan: Vec::new(),
            alternative_cost: Some(alt.clone()),
            exile_count: None,
            exile_ids: Vec::new(),
            sacrifice: None,
        };
        let castable = |alt: &ManaCost| CastableSpell {
            object_id: ObjectId(30),
            name: "Geistflame".to_string(),
            is_flashback: true,
            target_spec: CastTargetSpec::NoTargets,
            tap_plan: Vec::new(),
            exile_x_from_gy_max: None,
            sacrifice_options: Vec::new(),
            additional_cost_label: None,
            alternative_cost: Some(alt.clone()),
            from_graveyard: false,
        };
        let legal = mtg_engine::engine::LegalActions {
            actions: vec![Action::PassPriority, cast(&granted), cast(&printed), Action::Concede],
            combat_prompt: None,
            castable_spells: vec![castable(&granted), castable(&printed)],
            activatable_abilities: Vec::new(),
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: None,
            set_prompt: None,
        };

        let view = empty_view();
        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 2})]);
        let chosen = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        let list = &asked[0][asked[0].find("Available actions:\n").expect("the list")..];
        assert_eq!(
            list,
            "Available actions:\n\
             0: Pass\n\
             1: Flashback Geistflame (flashback cost {R})\n\
             2: Flashback Geistflame (flashback cost {3}{R})\n\
             3: Concede\n",
            "each flashback cost is its own way to cast, and each row says \
             which cost it charges:\n{}", asked[0]
        );
        let Action::CastSpell { alternative_cost, .. } = chosen else {
            panic!("a cast row casts: {chosen:?}")
        };
        assert_eq!(alternative_cost, Some(printed),
            "and the index the seat picked is the cost its row named");
    }

    /// A backend that answers `{}` — either because the model sent one, or
    /// because it gave up and never called at all.
    struct MuteBackend {
        model: &'static str,
        failure: Option<String>,
    }

    impl LlmBackend for MuteBackend {
        fn send(&mut self, message: &str) -> String {
            self.send_with_schema(message, &serde_json::Value::Null).to_string()
        }
        fn send_with_schema(&mut self, _m: &str, _s: &serde_json::Value) -> serde_json::Value {
            serde_json::json!({})
        }
        fn take_call_failure(&mut self) -> Option<String> { self.failure.clone() }
        fn init(&mut self, _deck_info: &str) {}
        fn system_prompt(&self) -> &str { "" }
        fn model_name(&self) -> &str { self.model }
    }

    fn mute_player(name: &str, model: &'static str, failure: Option<&str>) -> LlmPlayer {
        let mut player = LlmPlayer::for_prompt_tests(name).with_pass_until(false);
        player.backend = Box::new(MuteBackend {
            model,
            failure: failure.map(std::string::ToString::to_string),
        });
        player
    }

    /// Issue #587: when a seat's `claude -p` died, the backend returned
    /// nothing, the caller substituted the literal `{}`, and every surface
    /// downstream described the event as a model that had sent an empty
    /// object — `MALFORMED … ({}), defaulting to keep`, and `39 answers
    /// rejected` in the summary. Those are the words a seat gets for
    /// hallucinating an out-of-range index. The two call for opposite
    /// responses from the operator — fix the CLI, or fix the model — and in
    /// a mixed run they were added into one number.
    ///
    /// This is the half of #399 its close left behind: #489 made the
    /// substitution *counted*; nothing made it *attributable*.
    #[test]
    fn a_backend_that_never_answered_is_not_a_seat_that_answered_badly() {
        let view = empty_view();
        let legal = copies_priority_offer();

        // One seat whose subprocess died, one whose model really did send
        // `{}`. Distinct names and models, because the tallies are global.
        let mut dead = mute_player("Seat-587-dead", "model-587-dead",
            Some("claude -p exhausted all 3 attempts"));
        let mut bad = mute_player("Seat-587-bad", "model-587-bad", None);
        let _ = dead.choose_action(&view, &legal);
        let _ = bad.choose_action(&view, &legal);

        let unanswered = get_unanswered_by_seat();
        let rejected = get_rejected_by_seat();
        assert_eq!(unanswered.get("Seat-587-dead"), Some(&1),
            "a decision the backend never answered is counted as one: {unanswered:?}");
        assert_eq!(rejected.get("Seat-587-dead"), None,
            "…and not as an answer the seat gave: {rejected:?}");
        assert_eq!(rejected.get("Seat-587-bad"), Some(&1),
            "a model that really sent {{}} is still a rejected answer: {rejected:?}");
        assert_eq!(unanswered.get("Seat-587-bad"), None,
            "…and is not reported as a dead backend: {unanswered:?}");

        // The per-model counters split the same way, which is what both
        // runners' summaries read.
        let usage = get_llm_model_usage();
        assert_eq!(usage.get("model-587-dead").map(|u| u.unanswered), Some(1));
        assert_eq!(usage.get("model-587-dead").map(|u| u.rejected), Some(0));
        assert_eq!(usage.get("model-587-bad").map(|u| u.rejected), Some(1));
        assert_eq!(usage.get("model-587-bad").map(|u| u.unanswered), Some(0));

        // And the sentences the summaries print say which is which.
        assert_eq!(unanswered_note(0), "");
        assert_eq!(unanswered_note(1), ", 1 decision the backend never answered → fallback");
        assert_eq!(unanswered_note(39), ", 39 decisions the backend never answered → fallback");
    }

    /// The attack schema bounds every index by what the prompt offered, and
    /// an index outside it — which used to be schema-valid and silently
    /// became "did not attack" — is a rejected answer (issue #635).
    #[test]
    fn an_attack_index_the_prompt_did_not_offer_is_refused_not_dropped() {
        let schema = LlmPlayer::attackers_schema(2, 1);
        assert_eq!(schema["properties"]["attacker_indices"]["items"]["enum"], serde_json::json!([0, 1]));
        let pw = &schema["properties"]["planeswalker_attacks"]["items"]["properties"];
        assert_eq!(pw["attacker"]["enum"], serde_json::json!([0, 1]));
        assert_eq!(pw["planeswalker"]["enum"], serde_json::json!([0]));
        assert_eq!(LlmPlayer::attackers_schema(2, 0)["properties"]["planeswalker_attacks"]["maxItems"], 0);

        let view = empty_view();
        let prompt = CombatPrompt::ChooseAttackers {
            eligible: vec![ObjectId(22), ObjectId(23)],
            must_attack: vec![],
            defending_player: PlayerId(1),
            defending_planeswalkers: vec![ObjectId(40)],
        };
        // Off by one: index 2 of two attackers, and a planeswalker that is not there.
        let mut seat = fixed_player("Seat-635", "model-635", serde_json::json!({
            "thoughts": "t", "attacker_indices": [1, 2],
            "planeswalker_attacks": [{"attacker": 0, "planeswalker": 1}]}));
        let Action::DeclareAttackers { attackers, planeswalker_attacks } = seat.choose_combat(&view, &prompt)
            else { panic!("an attack declaration") };
        assert_eq!(attackers, vec![(ObjectId(23), PlayerId(1))], "the offered index still attacks");
        assert!(planeswalker_attacks.is_empty());
        assert_eq!(get_rejected_by_seat().get("Seat-635"), Some(&1),
            "the dropped indices are one rejected answer, not a silent no-attack");
    }

    /// An attacker named in both `attacker_indices` and `planeswalker_attacks`,
    /// or at two planeswalkers, is a contradictory answer: it is read one way
    /// (the planeswalker; the first one) and counted, not reinterpreted in
    /// silence (issue #663).
    #[test]
    fn a_contradictory_attack_answer_is_counted_not_silently_reread() {
        let view = empty_view();
        let prompt = CombatPrompt::ChooseAttackers {
            eligible: vec![ObjectId(22), ObjectId(23)],
            must_attack: vec![],
            defending_player: PlayerId(1),
            defending_planeswalkers: vec![ObjectId(40), ObjectId(41)],
        };
        let declare = |seat: &str, reply: serde_json::Value| {
            let mut p = fixed_player(seat, "model-663", reply);
            let Action::DeclareAttackers { attackers, planeswalker_attacks } = p.choose_combat(&view, &prompt)
                else { panic!("an attack declaration") };
            (attackers, planeswalker_attacks, get_rejected_by_seat().get(seat).copied())
        };

        // The control: a consistent split counts nothing.
        let (players, walkers, rejected) = declare("Seat-663-ok", serde_json::json!({
            "thoughts": "t", "attacker_indices": [1],
            "planeswalker_attacks": [{"attacker": 0, "planeswalker": 0}]}));
        assert_eq!(players, vec![(ObjectId(23), PlayerId(1))]);
        assert_eq!(walkers, vec![(ObjectId(22), ObjectId(40))]);
        assert_eq!(rejected, None);

        // The same attacker at the player and at a walker.
        let (players, walkers, rejected) = declare("Seat-663-both", serde_json::json!({
            "thoughts": "t", "attacker_indices": [0],
            "planeswalker_attacks": [{"attacker": 0, "planeswalker": 0}]}));
        assert!(players.is_empty(), "it attacks once");
        assert_eq!(walkers, vec![(ObjectId(22), ObjectId(40))]);
        assert_eq!(rejected, Some(1), "the contradiction is one rejected answer");

        // The same attacker at two walkers.
        let (players, walkers, rejected) = declare("Seat-663-twice", serde_json::json!({
            "thoughts": "t", "attacker_indices": [],
            "planeswalker_attacks": [{"attacker": 0, "planeswalker": 0}, {"attacker": 0, "planeswalker": 1}]}));
        assert!(players.is_empty());
        assert_eq!(walkers, vec![(ObjectId(22), ObjectId(40))], "it attacks the first walker only");
        assert_eq!(rejected, Some(1));
    }

    /// The pile-division schema requires a pile for every permanent, and
    /// an answer that names only some of them anyway is a rejected answer,
    /// not a silent "everything else in pile B" (issue #662).
    #[test]
    fn a_pile_division_that_skips_a_permanent_is_counted_not_filled_in() {
        use mtg_engine::actions::ResolvedChoice;
        let view = empty_view();
        let ids = [ObjectId(10), ObjectId(11), ObjectId(12)];
        let schema = LlmPlayer::pile_division_schema(ids.len());
        assert_eq!(schema["properties"]["piles"]["minItems"], 3, "every permanent must be named: {schema}");
        assert_eq!(schema["properties"]["piles"]["maxItems"], 3, "every permanent must be named: {schema}");

        // A complete answer is taken as given and counts nothing.
        let mut whole = fixed_player("Seat-662-whole", "model-662-whole", serde_json::json!({
            "thoughts": "t", "piles": ["A", "B", "A"]}));
        let Action::ResolveChoice { choice: ResolvedChoice::ChosenSubset(a) } =
            whole.choose_pile_division(&view, &ids, "divide", PlayerId(1))
            else { panic!("a pile division") };
        assert_eq!(a, vec![ObjectId(10), ObjectId(12)]);
        assert_eq!(get_rejected_by_seat().get("Seat-662-whole"), None);

        // No list, an empty list, and one that names a single permanent.
        for (seat, answer) in [
            ("Seat-662-none", serde_json::json!({"thoughts": "t"})),
            ("Seat-662-empty", serde_json::json!({"thoughts": "t", "piles": []})),
            ("Seat-662-half", serde_json::json!({"thoughts": "t", "piles": ["A"]})),
        ] {
            let mut p = fixed_player(seat, "model-662", answer);
            let _ = p.choose_pile_division(&view, &ids, "divide", PlayerId(1));
            assert_eq!(get_rejected_by_seat().get(seat), Some(&1),
                "{seat}: the unnamed permanents were put in pile B for the seat — one rejected answer");
        }
    }

    /// #857: what is being divided or funded changes the prompt, never the
    /// schema the request carries. The pile division was keyed by
    /// `Name (#id)` and X funding by the untapped groups' names, with their
    /// counts in the descriptions, so nearly every such call was a cold write
    /// of the cached prefix.
    #[test]
    fn pile_division_and_x_funding_send_one_schema_whatever_the_board() {
        use mtg_engine::funding::{FundingCategory, FundingGroup, FundingOptions};
        let sent = |s: &serde_json::Value| cache_stable_schema(s).to_string();
        assert_eq!(sent(&LlmPlayer::pile_division_schema(2)), sent(&LlmPlayer::pile_division_schema(7)));

        let group = |name: &str, n: u32, per: u32, category| FundingGroup {
            name: name.to_string(), category, mana_per_tap: per,
            source_ids: (0..n).map(|i| ObjectId(300 + u64::from(i))).collect(), colors_produced: vec![],
        };
        let offers = [
            (std::collections::BTreeMap::new(), vec![group("Sol Ring", 2, 2, FundingCategory::Rocks)]),
            (std::collections::BTreeMap::from([(mtg_engine::types::ManaType::Black, 1)]),
             vec![group("Forest", 3, 1, FundingCategory::Lands), group("Mountain", 1, 1, FundingCategory::Lands),
                  group("Avacyn's Pilgrim", 1, 1, FundingCategory::Dorks)]),
        ];
        let mut schemas = Vec::new();
        for (pool, groups) in offers {
            let mut legal = x_funding_offer();
            if let Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding { options, .. }) =
                legal.resolution_prompt.as_mut() {
                *options = FundingOptions { pool, groups, max_x: 6, x_discount: 0, pool_order: vec![] };
            }
            let recorded = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut p = LlmPlayer::for_prompt_tests("Seat-857").with_pass_until(false);
            p.backend = Box::new(SchemaRecorder { reply: serde_json::json!({}), schemas: recorded.clone() });
            let _ = p.choose_action(&empty_view(), &legal);
            schemas.push(sent(&recorded.borrow()[0]));
        }
        assert_eq!(schemas[0], schemas[1], "X funding sent a schema per board");
    }

    /// The X-funding answer is one amount per listed source, held to that
    /// source's legal amounts: a Sol Ring makes 0, 2 or 4, never 3, and an
    /// answer that says 3 is asked again with the reason.
    #[test]
    fn an_x_funding_source_is_held_to_its_own_amounts() {
        let schema = {
            let recorded = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut p = LlmPlayer::for_prompt_tests("Seat-857-amounts").with_pass_until(false);
            p.backend = Box::new(SchemaRecorder { reply: serde_json::json!({}), schemas: recorded.clone() });
            let _ = p.choose_action(&empty_view(), &x_funding_offer());
            let s = recorded.borrow()[0].clone();
            s
        };
        let floating = serde_json::json!({"white": "0", "blue": "0", "black": "0", "red": "0", "green": "0", "colorless": "0"});
        let answer = |amount: &str| serde_json::json!({"thoughts": "t", "cancel": false,
            "floating": floating, "sources": [amount]});
        assert_eq!(schema_violation(&schema, &answer("4")), None);
        assert!(schema_violation(&schema, &answer("3")).is_some_and(|w| w.contains("sources[0]")),
            "{:?}", schema_violation(&schema, &answer("3")));
        // A slot's own values repair a copied row, as an `items` enum does.
        assert_eq!(repair_enum_strings(&schema, &answer("4 mana")), answer("4"));
        assert!(schema_violation(&schema, &serde_json::json!({"thoughts": "t", "cancel": false,
            "floating": floating, "sources": []})).is_some(), "every listed source is answered");
    }

    /// #930: the amounts are string enums on the wire, and the prompt shows
    /// them as numbers, which is how the reader takes them too (#596). The
    /// check took only the strings, so an X funding answered in numbers
    /// was asked again every time; and it required all six pool colours,
    /// five of which an empty pool can only fill with 0.
    #[test]
    fn an_x_funding_is_answered_in_numbers_and_with_only_the_colours_that_float() {
        use mtg_engine::types::ManaType;
        let schema_for = |seat: &str, legal: &mtg_engine::engine::LegalActions| {
            let recorded = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut p = LlmPlayer::for_prompt_tests(seat).with_pass_until(false);
            p.backend = Box::new(SchemaRecorder { reply: serde_json::json!({}), schemas: recorded.clone() });
            let _ = p.choose_action(&empty_view(), legal);
            let s = recorded.borrow()[0].clone();
            s
        };
        let empty = schema_for("Seat-930-empty", &x_funding_offer());
        let six = serde_json::json!({"white": 0, "blue": 0, "black": 0, "red": 0, "green": 0, "colorless": 0});
        let answer = |floating: serde_json::Value, sources: serde_json::Value| serde_json::json!(
            {"thoughts": "t", "cancel": false, "floating": floating, "sources": sources});
        assert_eq!(schema_violation(&empty, &answer(six.clone(), serde_json::json!([4]))), None,
            "numbers are the amounts the prompt shows");
        assert_eq!(schema_violation(&empty, &answer(serde_json::json!({}), serde_json::json!(["2"]))), None,
            "nothing floats, so no colour is required");
        assert_eq!(schema_violation(&empty, &serde_json::json!({"thoughts": "t", "cancel": false, "sources": [0]})), None);
        // A number is still held to the amounts, and to what floats.
        assert_eq!(schema_violation(&empty, &answer(six.clone(), serde_json::json!([3]))).as_deref(),
            Some("`sources[0]` is 3; it must be one of 0, 2, 4"));
        assert_eq!(schema_violation(&empty, &answer(serde_json::json!({"red": 1}), serde_json::json!([0]))).as_deref(),
            Some("`floating.red` is 1; it must be 0"));
        assert!(schema_violation(&empty, &answer(six, serde_json::json!([2.0]))).is_some(), "2.0 is not an amount");
        let line = answer_text::answer_format(&empty);
        assert!(line.contains(r#""sources": [integer]"#) && !line.contains("floating"), "{line}");

        // Two red floating: red is asked, as a number, and required; the
        // other five are not.
        let red = schema_for("Seat-930-red",
            &x_funding_offer_with_pool(std::collections::BTreeMap::from([(ManaType::Red, 2)])));
        let line = answer_text::answer_format(&red);
        assert!(line.contains(r#""floating": {"red": integer 0 to 2}"#), "{line}");
        assert_eq!(schema_violation(&red, &answer(serde_json::json!({"red": 2}), serde_json::json!([0]))), None);
        assert_eq!(schema_violation(&red, &answer(serde_json::json!({}), serde_json::json!([0]))).as_deref(),
            Some("`floating.red` is missing"));
        // The request is the same whatever floats: the marker is not sent.
        assert_eq!(cache_stable_schema(&empty), cache_stable_schema(&red));
        assert!(!cache_stable_schema(&empty).to_string().contains(ONLY_VALUE));
        assert!(!sanitize_schema_for_gemini(&empty).to_string().contains(ONLY_VALUE));
    }

    /// A backend that answers one fixed object, whatever it is asked.
    struct FixedBackend {
        model: &'static str,
        reply: serde_json::Value,
    }

    impl LlmBackend for FixedBackend {
        fn send(&mut self, message: &str) -> String {
            self.send_with_schema(message, &serde_json::Value::Null).to_string()
        }
        fn send_with_schema(&mut self, _m: &str, _s: &serde_json::Value) -> serde_json::Value {
            self.reply.clone()
        }
        fn init(&mut self, _deck_info: &str) {}
        fn system_prompt(&self) -> &str { "" }
        fn model_name(&self) -> &str { self.model }
    }

    fn fixed_player(name: &str, model: &'static str, reply: serde_json::Value) -> LlmPlayer {
        let mut player = LlmPlayer::for_prompt_tests(name).with_pass_until(false);
        player.backend = Box::new(FixedBackend { model, reply });
        player
    }

    /// Two untapped Sol Rings and nothing else: the only allocations are
    /// 0, 2 and 4, and the schema offers them as the strings "0", "2", "4".
    fn x_funding_offer() -> mtg_engine::engine::LegalActions {
        x_funding_offer_with_pool(std::collections::BTreeMap::new())
    }

    /// [`x_funding_offer`] with `pool` floating.
    fn x_funding_offer_with_pool(
        pool: std::collections::BTreeMap<mtg_engine::types::ManaType, u32>,
    ) -> mtg_engine::engine::LegalActions {
        use mtg_engine::funding::{FundingCategory, FundingGroup, FundingOptions};
        let options = FundingOptions {
            pool,
            groups: vec![FundingGroup {
                name: "Sol Ring".to_string(),
                category: FundingCategory::Rocks,
                mana_per_tap: 2,
                source_ids: vec![ObjectId(301), ObjectId(302)],
                colors_produced: vec![],
            }],
            max_x: 4,
            x_discount: 0,
            pool_order: vec![],
        };
        mtg_engine::engine::LegalActions {
            actions: Vec::new(),
            combat_prompt: None,
            castable_spells: Vec::new(),
            activatable_abilities: Vec::new(),
            context: Some("MAIN PHASE 1".to_string()),
            resolution_prompt: Some(mtg_engine::state::ResolutionChoiceKind::ChooseXFunding {
                description: "Choose X for Devil's Play".to_string(),
                options,
                source_id: ObjectId(400),
                is_ability: false,
            }),
            set_prompt: None,
        }
    }

    fn funded_x(player: &mut LlmPlayer, legal: &mtg_engine::engine::LegalActions) -> u32 {
        use mtg_engine::actions::ResolvedChoice;
        let view = empty_view();
        match player.choose_action(&view, legal) {
            Action::ResolveChoice { choice: ResolvedChoice::XFunding(f) } => f.x_value(),
            other => panic!("an X funding prompt is answered with a funding response: {other:?}"),
        }
    }

    /// A backend that answers a fixed object and keeps every schema it was sent.
    struct SchemaRecorder {
        reply: serde_json::Value,
        schemas: std::rc::Rc<std::cell::RefCell<Vec<serde_json::Value>>>,
    }

    impl LlmBackend for SchemaRecorder {
        fn send(&mut self, message: &str) -> String {
            self.send_with_schema(message, &serde_json::Value::Null).to_string()
        }
        fn send_with_schema(&mut self, _m: &str, s: &serde_json::Value) -> serde_json::Value {
            self.schemas.borrow_mut().push(s.clone());
            self.reply.clone()
        }
        fn init(&mut self, _deck_info: &str) {}
        fn system_prompt(&self) -> &str { "" }
        fn model_name(&self) -> &str { "model-749" }
    }

    /// #749: the engine takes `CancelCast` at X funding, an "up to N" target
    /// set, an exile-from-graveyard cost and the taps an activation's cost
    /// asks for, and the CLI, the page and the random seat all offer it.
    /// This seat had no way to say it: an X spell it could no longer afford
    /// was cast for X = 0. Each of the four now carries a `cancel` field
    /// that answers `CancelCast`; an effect resolving, with no cast behind
    /// it, does not.
    #[test]
    fn the_four_cancellable_prompts_offer_cancel_and_take_it() {
        use mtg_engine::actions::{ResolvedChoice, Target};
        use mtg_engine::state::{PendingEffect, ResolutionChoiceKind};
        let with = |kind: ResolutionChoiceKind| mtg_engine::engine::LegalActions {
            resolution_prompt: Some(kind), ..x_funding_offer()
        };
        let ids = vec![ObjectId(501), ObjectId(502)];
        let prompts = [
            ("x-funding", x_funding_offer(), true),
            ("target-set", with(ResolutionChoiceKind::ChooseTargetSet {
                description: "Memory's Journey: choose up to 3 targets".to_string(),
                options: ids.iter().map(|&o| Target::Object(o)).collect(), min: 0, max: 2,
                source_id: ObjectId(400), fixed: vec![], by_count: vec![],
            }), true),
            ("exile-cost", with(ResolutionChoiceKind::ChooseExileFromGraveyard {
                description: "Stitched Drake: exile a creature card".to_string(),
                options: ids.clone(), min: 1, max: 1, source_id: ObjectId(400),
            }), true),
            ("activation-taps", with(ResolutionChoiceKind::ChooseObjectSet {
                description: "tap two untapped creatures".to_string(),
                options: ids.clone(), min: 2, max: 2,
                effect: PendingEffect::PayActivationTaps { source_id: ObjectId(400) },
            }), true),
            ("effect", with(ResolutionChoiceKind::ChooseObjectSet {
                description: "choose two cards".to_string(),
                options: ids.clone(), min: 2, max: 2,
                effect: PendingEffect::Destroy { source_name: "Curse".to_string() },
            }), false),
        ];
        for (name, legal, cancellable) in prompts {
            for cancel in [true, false] {
                let schemas = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
                let mut p = LlmPlayer::for_prompt_tests(&format!("Seat-749-{name}")).with_pass_until(false);
                p.backend = Box::new(SchemaRecorder {
                    reply: serde_json::json!({"thoughts": "t", "cancel": cancel, "indices": [0, 1],
                        "floating": {}, "sources": ["2"]}),
                    schemas: schemas.clone(),
                });
                let chosen = p.choose_action(&empty_view(), &legal);
                let schema = schemas.borrow().last().cloned().expect("one call");
                let offered = schema["properties"].get("cancel").is_some();
                assert_eq!(offered, cancellable, "{name}: cancel offered: {schema}");
                if cancellable {
                    assert!(schema["required"].as_array().unwrap().contains(&serde_json::json!("cancel")),
                        "{name}: cancel is required, as every other field is");
                }
                let cancelled = matches!(chosen, Action::ResolveChoice { choice: ResolvedChoice::CancelCast });
                assert_eq!(cancelled, cancel && cancellable, "{name}, cancel = {cancel}: {chosen:?}");
            }
        }
    }

    /// Issue #596: the X-funding reader was `as_str().and_then(parse).
    /// unwrap_or(0)`, so an answer whose values were JSON numbers instead of
    /// the schema's strings read as 0 for every group. The response became
    /// `FundingResponse::default()`, which `funding::validate` accepts
    /// because an empty response is a legal X = 0 — so `log_rejected` was
    /// never called, the `answers rejected → fallback` counter stayed at
    /// zero, and the seat logged `X funding sum = 0`, byte-identical to a
    /// model that deliberately announced zero. A seat cast every X spell for
    /// nothing and the operator-facing record said nothing had gone wrong.
    ///
    /// Two halves: the same number in the other JSON type is the same
    /// allocation (the eleven other readers in this file take `as_u64()`),
    /// and a value that really cannot be read is still substituted but is
    /// **counted**.
    #[test]
    fn an_x_funding_answer_is_read_as_a_number_or_counted_as_unreadable() {
        let legal = x_funding_offer();

        // The control: the schema's own shape.
        let mut strings = fixed_player("Seat-596-strings", "model-596-strings",
            serde_json::json!({"sources": ["4"]}));
        assert_eq!(funded_x(&mut strings, &legal), 4);

        // The defect: the same allocation as a JSON number.
        let mut numbers = fixed_player("Seat-596-numbers", "model-596-numbers",
            serde_json::json!({"sources": [4]}));
        assert_eq!(funded_x(&mut numbers, &legal), 4,
            "a number is the same allocation as the string of that number");

        // Neither is a rejection: both said 4 and both got 4.
        let rejected = get_rejected_by_seat();
        assert_eq!(rejected.get("Seat-596-strings"), None, "{rejected:?}");
        assert_eq!(rejected.get("Seat-596-numbers"), None, "{rejected:?}");

        // And what genuinely cannot be read is substituted AND counted, so
        // the one counter built to surface this no longer reads clean.
        for (seat, model, value) in [
            ("Seat-596-word", "model-596-word", serde_json::json!("two")),
            ("Seat-596-empty", "model-596-empty", serde_json::json!("")),
            ("Seat-596-hex", "model-596-hex", serde_json::json!("0x2")),
            ("Seat-596-float", "model-596-float", serde_json::json!(2.5)),
            ("Seat-596-negative", "model-596-negative", serde_json::json!(-2)),
            ("Seat-596-object", "model-596-object", serde_json::json!({"taps": 2})),
        ] {
            let mut player = fixed_player(seat, model,
                serde_json::json!({"sources": [value]}));
            assert_eq!(funded_x(&mut player, &legal), 0,
                "{seat}: an unreadable allocation is substituted with nothing");
            let rejected = get_rejected_by_seat();
            assert_eq!(rejected.get(seat), Some(&1),
                "{seat}: the substitution is counted as an answer the harness could not use: \
                 {rejected:?}");
            assert_eq!(get_llm_model_usage().get(model).map(|u| u.rejected), Some(1),
                "{seat}: and in the per-model tally the summaries read");
        }
    }

    /// Issue #466: the system prompt's card reference was the union of both
    /// decklists, so a seat knew on turn 1 every card its opponent's deck
    /// held — and, the list being exhaustive, every card it did not. The
    /// seat is told about a card when it comes into view, in the decision
    /// prompt, and about nothing before.
    #[test]
    fn a_card_in_view_from_outside_your_deck_gets_its_text_in_the_prompt() {
        let registry = CardRegistry::with_all_cards();
        let mut player = LlmPlayer::for_prompt_tests("t");
        let own = vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)];
        player.init_conversation(&own, "", &registry, MatchFormat::SingleGame);

        let system = player.system_prompt_for_test();
        assert!(!system.contains("## Card reference"),
            "with no public reference, the system prompt has no reference section:\n{system}");
        assert!(!system.contains("Dissipate") && !system.contains("Grimgrin"),
            "nothing about a card the seat has not seen");

        let (you, opp) = (PlayerId(0), PlayerId(1));
        let mut view = empty_view();
        view.battlefield.push(perm(7, "Delver of Secrets", 1, 1, opp));
        view.battlefield.push(perm(8, "Grizzly Bears", 2, 2, opp));
        view.battlefield.push(perm(9, "Grizzly Bears", 2, 2, you));
        view.revealed_names.insert(ObjectId(30), "Dissipate".to_string());
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");

        let section = prompt.find("Opp's cards in view:\n").expect("the section is there");
        let body = &prompt[section..prompt.find("[MAIN PHASE 1]").expect("then the question")];
        assert!(body.contains("Delver of Secrets {U} | Creature — Human Wizard 1/1\n  At the beginning of your upkeep"),
            "the opponent's creature is described:\n{body}");
        assert!(body.contains("Dissipate {1}{U}{U} | Instant\n  Counter target spell."),
            "a revealed card is described:\n{body}");
        assert!(!body.contains("Grizzly Bears {1}{G}"),
            "a card from your own deck is not repeated, whoever controls it:\n{body}");
        assert!(prompt.find("Opp board:").unwrap() < section && section < prompt.find("[MAIN PHASE 1]").unwrap(),
            "the section sits between the board and the question:\n{prompt}");

        // GAME_RULES names the section by the heading the harness sends.
        assert!(GAME_RULES.contains("`Opp's cards in view:`"), "the section is documented");

        // Nothing in view from outside the deck: no section at all.
        let mut view = empty_view();
        view.battlefield.push(perm(9, "Grizzly Bears", 2, 2, you));
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        assert!(!prompt.contains("Opp's cards in view"), "{prompt}");
    }

    /// Issue #465: the context line named the opponent by the engine's seat
    /// label — `[RESPOND TO p1's Lightning Bolt]` — while every other line
    /// of the same prompt, and GAME_RULES' own example, says `opp's`. The
    /// seat is never told which seat number it is.
    #[test]
    fn the_context_line_names_the_opponent_as_opp() {
        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let mut legal = collapsing_priority_offer();
        legal.context = Some("RESPOND TO p1's Lightning Bolt".to_string());

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 0})]);
        let _ = player.choose_action(&view, &legal);

        let asked = prompts.borrow();
        assert!(
            asked[0].contains("[RESPOND TO opp's Lightning Bolt]"),
            "the context line reads like the rest of the prompt:\n{}", asked[0]
        );
        assert!(
            !asked[0].split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == "p1"),
            "no raw seat label reaches the prompt:\n{}", asked[0]
        );

        // And your own spell is still yours.
        legal.context = Some("RESPOND TO your Lightning Bolt".to_string());
        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"action": 0})]);
        let _ = player.choose_action(&view, &legal);
        assert!(prompts.borrow()[0].contains("[RESPOND TO your Lightning Bolt]"));
    }

    /// Issue #463: the cleanup discard was the only in-game decision built
    /// without `build_prompt` — 345 characters of card names, with no turn,
    /// life totals, board or graveyard — and, because it bypassed the
    /// builder, it never consumed the log either.
    #[test]
    fn the_cleanup_discard_prompt_carries_the_game_state() {
        let (state, registry) = view_for_contract_test();
        let mut view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        view.step = Step::Cleanup;
        view.turn_number = 18;
        view.display_log.push("p1 cast Ghoulraiser (#35)".to_string());
        let hand: Vec<ObjectId> = view.your_hand.iter().map(|c| c.object_id).collect();
        assert!(!hand.is_empty(), "the opening hand was drawn");
        let prompt = mtg_engine::actions::SetPrompt {
            kind: mtg_engine::actions::SetPromptKind::DiscardToHandSize,
            player: view.you,
            options: hand.clone(),
            min: 1,
            max: 1,
        };

        let (mut player, prompts) = scripted_player(vec![serde_json::json!({"card_indices": [0]})]);
        let chosen = player.choose_card_set(&view, &prompt);
        assert!(matches!(chosen, Action::DiscardCards { ref cards } if cards == &hand[..1]));

        let asked = prompts.borrow();
        let sent = &asked[0];
        assert!(sent.starts_with("Turn 18 - Cleanup (your turn)\n"),
            "the discard leads with the turn header like every other decision:\n{sent}");
        for section in ["Recent events:\n", "Opp cast Ghoulraiser (#35)\n", "You: 20hp", "Opp: 20hp", "Hand:\n  "] {
            assert!(sent.contains(section), "the discard prompt carries {section:?}:\n{sent}");
        }
        let context = sent.find("[DISCARD 1 CARD]").expect("the context line is there");
        assert!(sent[context..].contains("Your hand:\n  0: "),
            "the numbered hand the answer indexes follows the context line:\n{sent}");
        assert!(sent.find("You: 20hp").unwrap() < context, "state first, then the question");
        assert_eq!(player.last_log_index, view.display_log.len(),
            "answering the discard consumes the log, so the next recap is a delta");
    }

    /// Issue #464: nothing capped the recap, so a quiet stretch made it the
    /// largest thing in the prompt — 307 entries, 74% of one prompt — with
    /// nothing saying what it covered. The newest entries are kept and the
    /// block says how many older ones it dropped and through which turn.
    #[test]
    fn a_long_recap_keeps_the_newest_entries_and_says_what_it_dropped() {
        let (state, registry) = view_for_contract_test();
        let mut view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        view.display_log.clear();
        for turn in 1..=100u32 {
            let who = if turn % 2 == 1 { "p1" } else { "p0" };
            view.display_log.push(format!("── Turn {turn} ({who}) ──"));
            view.display_log.push(format!("{who} drew a card"));
            view.display_log.push(format!("{who} played Forest (#{turn})"));
        }
        let total = view.display_log.len();
        let cap = LlmPlayer::MAX_RECENT_EVENTS;
        assert!(total > cap);

        let mut player = LlmPlayer::for_prompt_tests("t");
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        let start = prompt.find("Recent events:\n").expect("a recap") + "Recent events:\n".len();
        let block = &prompt[start..prompt[start..].find("\n\n").expect("the block ends") + start];
        let lines: Vec<&str> = block.lines().collect();

        // The kept entries are the tail of the log: the newest `cap`. The
        // omitted stretch is 300 - 80 = 220 entries, three per turn, so it
        // ends just after turn 74's banner and the marker says so.
        let omitted = total - cap;
        assert_eq!(lines[0], format!("… {omitted} earlier entries omitted, through turn 74 …"),
            "the marker says how much was dropped and how far it reached:\n{block}");
        assert_eq!(lines.len(), cap + 1, "the cap plus the marker:\n{block}");
        assert_eq!(lines[lines.len() - 1], "You played Forest (#100)", "the newest entry is last");
        assert_eq!(lines[1], "You drew a card", "and the kept stretch starts where the dropped one ended");
        assert_eq!(player.last_log_index, total, "the whole log counts as shown, dropped entries included");

        // GAME_RULES documents the cap and the marker it produces.
        assert!(GAME_RULES.contains(&format!("most recent {cap} entries")),
            "GAME_RULES states the cap the harness applies");
        assert!(GAME_RULES.contains(&LlmPlayer::omitted_events_marker(227, Some("94"))),
            "GAME_RULES quotes the marker shape the harness sends");
    }

    /// A land tapped for mana is not sent as an event — the cast it paid
    /// for is, and the board shows what is tapped (reports/llm-cost.md,
    /// phase 3).
    #[test]
    fn a_mana_tap_is_not_an_event_but_the_cast_it_paid_for_is() {
        let (state, registry) = view_for_contract_test();
        let mut view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        view.display_log = vec![
            "p1 tapped Plains (#45) for mana (pool: White:1)".to_string(),
            "p1 cast Swords to Plowshares (#72) targeting Savannah Lions (#63)".to_string(),
        ];
        let mut player = LlmPlayer::for_prompt_tests("t");
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        assert!(!prompt.contains("for mana"), "{prompt}");
        assert!(prompt.contains("Recent events:\nOpp cast Swords to Plowshares (#72)"), "{prompt}");
        assert_eq!(player.last_log_index, 2, "both count as shown");
    }

    /// A recap that fits carries no marker and every entry.
    #[test]
    fn a_short_recap_is_whole_and_unmarked() {
        let (state, registry) = view_for_contract_test();
        let mut view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        view.display_log = vec!["── Turn 3 (p0) ──".to_string(), "p0 drew a card".to_string()];
        let mut player = LlmPlayer::for_prompt_tests("t");
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        assert!(prompt.contains("Recent events:\n── Turn 3 (your turn) ──\nYou drew a card\n\n"),
            "{prompt}");
        assert!(!prompt.contains("omitted"), "{prompt}");
    }

    /// The recap example in GAME_RULES uses the vocabulary the recap uses.
    #[test]
    fn game_rules_recap_example_is_rewritten_like_the_recap() {
        let entry = LlmPlayer::rewrite_log_entry("p0 drew a card", mtg_engine::ids::PlayerId(0));
        assert_eq!(entry, "You drew a card");
        assert!(raw_player_tokens(GAME_RULES).is_empty(), "no stale seat label in the rules");
    }

    /// A cast option with several legal targets names none — they come from
    /// a follow-up prompt, which is what the contract says (a forced target
    /// is named on the row instead, #827).
    #[test]
    fn game_rules_does_not_promise_inline_targets() {
        assert!(
            !GAME_RULES.contains("Cast Lightning Bolt → Goblin Piker"),
            "a cast row never offers a choice of targets inline; the follow-up prompt asks"
        );
        assert!(
            GAME_RULES.contains("select a target:"),
            "GAME_RULES names the follow-up target prompt the harness sends"
        );
    }

    #[test]
    fn format_counters_none_returns_none() {
        let counters: HashMap<CounterType, u32> = HashMap::new();
        assert_eq!(LlmPlayer::format_counters(&counters), None);
    }

    #[test]
    fn format_counters_zero_count_returns_none() {
        let mut counters = HashMap::new();
        counters.insert(CounterType::PlusOnePlusOne, 0);
        assert_eq!(LlmPlayer::format_counters(&counters), None);
    }

    #[test]
    fn format_counters_plus_one_plus_one() {
        let mut counters = HashMap::new();
        counters.insert(CounterType::PlusOnePlusOne, 2);
        assert_eq!(
            LlmPlayer::format_counters(&counters).as_deref(),
            Some("+1+1x2"),
        );
    }

    #[test]
    fn format_counters_minus_one_minus_one() {
        let mut counters = HashMap::new();
        counters.insert(CounterType::MinusOneMinusOne, 1);
        assert_eq!(
            LlmPlayer::format_counters(&counters).as_deref(),
            Some("-1-1x1"),
        );
    }

    #[test]
    fn format_counters_loyalty() {
        let mut counters = HashMap::new();
        counters.insert(CounterType::Loyalty, 4);
        assert_eq!(
            LlmPlayer::format_counters(&counters).as_deref(),
            Some("LOYx4"),
        );
    }

    #[test]
    fn format_counters_mixed_stable_order() {
        // +1/+1 and -1/-1 together (weird, but a valid transient state
        // before SBAs annihilate). Confirms ordering is plus-then-minus.
        let mut counters = HashMap::new();
        counters.insert(CounterType::PlusOnePlusOne, 3);
        counters.insert(CounterType::MinusOneMinusOne, 1);
        assert_eq!(
            LlmPlayer::format_counters(&counters).as_deref(),
            Some("+1+1x3,-1-1x1"),
        );
    }

    /// Bug 37-001 (`audits/AUDIT_BUGS.md)`: `format_counters` only
    /// surfaces +1/+1, -1/-1, and Loyalty counters. Slime counters
    /// (Gutter Grime's stockpile) and Study counters (Grimoire of the
    /// Dead's progress) are stripped from the display, so the LLM has
    /// no way to see how many slime counters Gutter Grime has or how
    /// close Grimoire of the Dead is to its 3-counter activation.
    ///
    /// This test asserts the EXPECTED CORRECT behavior, so it
    /// currently fails. It will start passing as soon as Bug 37-001
    /// is fixed.
    #[test]
    fn bug_37_001_format_counters_includes_slime_and_study() {
        let mut counters = HashMap::new();
        counters.insert(CounterType::Slime, 5);
        counters.insert(CounterType::Study, 2);
        let formatted = LlmPlayer::format_counters(&counters);
        let formatted_str = formatted.as_deref().unwrap_or("");
        assert!(
            formatted_str.contains("Slime") || formatted_str.contains("SLIME"),
            "format_counters should surface Slime counters so the LLM can \
             see Gutter Grime's stockpile. Bug 37-001: the helper drops \
             every counter type other than +1/+1, -1/-1, and Loyalty. \
             Got: {formatted:?}",
        );
        assert!(
            formatted_str.contains("Study") || formatted_str.contains("STUDY"),
            "format_counters should surface Study counters so the LLM can \
             see Grimoire of the Dead's progress. Bug 37-001: dropped. \
             Got: {formatted:?}",
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // format_combat_creature_list — disambiguation regression tests
    // ─────────────────────────────────────────────────────────────────

    use mtg_engine::ids::{CardId, ObjectId, PlayerId};
    use mtg_engine::types::{CardType, Step, ManaPool};
    use mtg_engine::view::{GameView, PermanentView};

    /// #865: the opponent's floating mana was said only by the "tapped …
    /// for mana" event, which a prompt no longer carries.
    #[test]
    fn the_opponents_floating_mana_is_in_the_prompt() {
        let mut view = empty_view();
        view.opponents = vec![mtg_engine::view::OpponentView {
            id: PlayerId(1), life: 20, hand_size: 7, library_size: 53,
            mana_pool: ManaPool::default(), mulligan_count: 0,
        }];
        assert!(!LlmPlayer::format_state_body(&view).contains("Opp mana pool"));
        view.opponents[0].mana_pool.add(mtg_engine::types::ManaType::Black, 1);
        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("Opp mana pool: Black:1\n"), "{body}");
    }

    pub(crate) fn empty_view() -> GameView {
        GameView {
            stack_puts: 0,
            opponent_stack_puts: 0,
            you: PlayerId(0),
            your_hand: vec![],
            your_life: 20,
            your_mana_pool: ManaPool::default(),
            your_library_size: 30,
            your_library_cards: vec![],
            your_mulligan_count: 0,
            opponents: vec![],
            battlefield: vec![],
            graveyards: vec![],
            stack: vec![],
            first_strike_damage_step: false,
            creature_died_this_turn: false,
            spells_cast_this_turn: vec![],
            double_faces: Default::default(),
            nameable_cards: Default::default(),
            exile: vec![],
            step: Step::PrecombatMain,
            active_player: PlayerId(0),
            priority_player: Some(PlayerId(0)),
            turn_number: 1,
            display_log: vec![],
            full_log: vec![],
            revealed_names: HashMap::new(),
        }
    }

    /// Issue #611: `Flashback available:` is the only place in the prompt that
    /// names a flashback cost at all, and it printed the card's printed cost
    /// and nothing else. With Past in Flames' granted cost also on the card
    /// (CR 702.33 allows several instances at once) the section stated the
    /// cost of a row the seat was NOT offered — picking the row it was
    /// offered spent one mana against the `{3}{R}` the prompt had named.
    #[test]
    fn the_flashback_section_names_every_cost_the_card_carries() {
        use mtg_engine::types::{Color, ManaCost, ManaSymbol};

        let granted = ManaCost::new(vec![ManaSymbol::Colored(Color::Red)]);
        let printed = ManaCost::new(vec![
            ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Red)]);
        let mut view = empty_view();
        view.graveyards = vec![(PlayerId(0), vec![mtg_engine::view::CardView {
            object_id: ObjectId(30),
            card_id: mtg_engine::ids::CardId(1),
            name: "Geistflame".to_string(),
            cost: Some(granted.clone()),
            supertypes: vec![],
            card_types: vec![mtg_engine::types::CardType::Instant],
            power: None,
            toughness: None,
            oracle_text: String::new(),
            owner: PlayerId(0),
            flashback_costs: vec![granted, printed],
        }])];

        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("Geistflame (flashback {R} or {3}{R})"),
            "both instances are payable and the seat is offered both, so the \
             section that names the cost names both:\n{body}");
    }

    /// Late in a game the prompt was mostly repetition: every dead
    /// creature's and spent spell's rules text under "Opp's cards in view"
    /// (a third of a prompt's bytes), a graveyard one card a line, and one
    /// death's eight triggers eight times in the events and eight on the
    /// stack (reports/llm-cost.md, phase 3). A graveyard card is described
    /// where its text works from there; identical neighbours are one line
    /// with a count; a zone's cards are grouped by name.
    #[test]
    fn a_late_game_prompt_says_each_thing_once() {
        let registry = CardRegistry::with_all_cards();
        let mut player = LlmPlayer::for_prompt_tests("t");
        let own = vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)];
        player.init_conversation(&own, "", &registry, MatchFormat::SingleGame);
        let (you, opp) = (PlayerId(0), PlayerId(1));
        let yard = |id: u64, name: &str, owner: PlayerId| mtg_engine::view::CardView {
            object_id: ObjectId(id), card_id: mtg_engine::ids::CardId(0), name: name.to_string(),
            cost: None, supertypes: vec![], card_types: vec![], power: None, toughness: None,
            oracle_text: String::new(), owner, flashback_costs: vec![],
        };
        let mut view = empty_view();
        view.battlefield.push(perm(7, "Selhoff Occultist", 2, 3, opp));
        view.graveyards = vec![
            (you, vec![yard(1, "Forest", you), yard(2, "Grizzly Bears", you), yard(3, "Forest", you)]),
            (opp, vec![yard(4, "Mulch", opp), yard(5, "Gnaw to the Bone", opp), yard(6, "Mulch", opp)]),
        ];
        for _ in 0..3 {
            view.stack.push(mtg_engine::view::StackItemView {
                object_id: ObjectId(40), card_id: mtg_engine::ids::CardId(0),
                name: "Selhoff Occultist's triggered ability".to_string(), source_id: Some(ObjectId(7)),
                controller: opp, targets: vec![mtg_engine::actions::Target::Player(you)],
                x_value: None, cost: None, supertypes: vec![], card_types: vec![], power: None,
                toughness: None, oracle_text: String::new(), resolving: false,
            });
        }
        let prompt = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        let section = &prompt[prompt.find("Opp's cards in view:\n").expect("the section")..];
        assert!(section.contains("Selhoff Occultist {2}{U}"), "on the battlefield: described\n{section}");
        assert!(section.contains("Gnaw to the Bone {2}{G}"), "flashback works from the graveyard\n{section}");
        assert!(!section.contains("Mulch {1}{G}"), "a spent sorcery is its name in the list\n{section}");
        assert!(prompt.contains("Your graveyard:\n  2x Forest\n  Grizzly Bears\n"), "{prompt}");
        assert!(prompt.contains("Opp graveyard:\n  2x Mulch\n  Gnaw to the Bone\n"), "{prompt}");
        assert!(prompt.contains("Stack:\n  Selhoff Occultist's triggered ability (#7) (opponent's) targeting you (×3)\n"),
            "{prompt}");

        assert_eq!(LlmPlayer::collapse_repeats(["a", "b", "b", "b", "a"].map(String::from)),
            vec!["a".to_string(), "b (×3)".to_string(), "a".to_string()]);
        assert!(LlmPlayer::is_mana_tap_entry("Opp's mana pool empties (Black:1)"), "not news");
    }

    /// A new game of a match starts with no attack planned: game 2 repeats
    /// game 1's turn numbers and object ids, so a leftover plan could be
    /// declared on the new board unasked (#862 review).
    #[test]
    fn a_new_game_forgets_the_last_games_planned_attack() {
        let registry = CardRegistry::with_all_cards();
        let mut player = LlmPlayer::for_prompt_tests("t");
        let own = vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)];
        player.attack_plan = Some(AttackPlan { turn: 3, attackers: vec![ObjectId(30)], opp_puts: 0 });
        player.plan_done_pending = true;
        player.init_conversation(&own, "", &registry, MatchFormat::SingleGame);
        assert!(player.attack_plan.is_none() && !player.plan_done_pending);
    }

    /// #864: a graveyard card that does nothing from there is its name in
    /// the list — unless the question or something on the stack points at
    /// it, when its text is what the answer turns on: Memory's Journey's
    /// targets, a name to choose, the opponent's reanimation spell.
    #[test]
    fn a_graveyard_card_the_question_points_at_is_described() {
        let registry = CardRegistry::with_all_cards();
        let mut player = LlmPlayer::for_prompt_tests("t");
        let own = vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)];
        player.init_conversation(&own, "", &registry, MatchFormat::SingleGame);
        let opp = PlayerId(1);
        let yard = |id: u64, name: &str| mtg_engine::view::CardView {
            object_id: ObjectId(id), card_id: mtg_engine::ids::CardId(0), name: name.to_string(),
            cost: None, supertypes: vec![], card_types: vec![], power: None, toughness: None,
            oracle_text: String::new(), owner: opp, flashback_costs: vec![],
        };
        let mut view = empty_view();
        view.graveyards = vec![(opp, vec![yard(114, "Unbreathing Horde"), yard(116, "Falkenrath Noble"), yard(5, "Mulch")])];
        let cards = |player: &mut LlmPlayer, view: &GameView, ask: &str| {
            let prompt = player.build_prompt(view, ask);
            prompt.find("Opp's cards in view:\n").map_or_else(String::new, |i| prompt[i..].to_string())
        };
        // A plain menu: names only.
        let plain = cards(&mut player, &view, "Available actions:\n0: Pass\n");
        assert!(!plain.contains("Unbreathing Horde {2}{B}") && !plain.contains("Falkenrath Noble {3}{B}"), "{plain}");

        // A target set naming them by number.
        let journey = cards(&mut player, &view,
            "Memory's Journey: choose up to 3 targets\n0: Unbreathing Horde (#114) (in opponent's graveyard)\n");
        assert!(journey.contains("Unbreathing Horde {2}{B}"), "{journey}");
        assert!(!journey.contains("Falkenrath Noble {3}{B}"), "only what is asked about:\n{journey}");

        // A list of names to choose from.
        let name = cards(&mut player, &view, "Choose a card name:\n0: Falkenrath Noble\n1: Griselbrand\n");
        assert!(name.contains("Falkenrath Noble {3}{B}"), "{name}");

        // The opponent's spell on the stack targeting one.
        view.stack.push(mtg_engine::view::StackItemView {
            object_id: ObjectId(94), card_id: mtg_engine::ids::CardId(0),
            name: "Ghoulcaller's Chant".to_string(), source_id: Some(ObjectId(94)),
            controller: opp, targets: vec![mtg_engine::actions::Target::Object(ObjectId(116))],
            x_value: None, cost: None, supertypes: vec![], card_types: vec![], power: None,
            toughness: None, oracle_text: String::new(), resolving: false,
        });
        let chant = cards(&mut player, &view, "Available actions:\n0: Pass\n");
        assert!(chant.contains("Falkenrath Noble {3}{B}"), "{chant}");
        assert!(!chant.contains("Mulch {1}{G}"), "{chant}");
    }

    /// #785's review: a transformed permanent is named by its back face, and
    /// that name found no card, so an opponent's Insectile Aberration was an
    /// empty entry under "Opp's cards in view". It names the same card.
    #[test]
    fn a_back_face_name_is_described_with_both_faces() {
        let registry = mtg_engine::cards::CardRegistry::with_all_cards();
        let entry = card_reference_entry("Insectile Aberration", &registry);
        assert!(entry.contains("Delver of Secrets") && entry.contains("Insectile Aberration"), "{entry}");
        assert!(entry.to_lowercase().contains("flying"), "the back's text: {entry}");
    }

    /// Issue #832: whether a creature died this turn (morbid) and how many
    /// spells each side cast (werewolves) were in the prompt only as the
    /// one "Recent events" line after the event. The view carries them and
    /// every prompt states them.
    #[test]
    fn every_prompt_says_what_this_turn_has_done() {
        let (mut state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, PlayerId(0), &registry);
        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("This turn: spells cast — you 0, opp 0\n"), "{body}");

        state.creature_died_this_turn = true;
        state.num_spells_cast_this_turn.insert(PlayerId(1), 2);
        let view = GameView::for_player(&state, PlayerId(0), &registry);
        assert!(view.creature_died_this_turn);
        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("This turn: spells cast — you 0, opp 2; a creature died\n"), "{body}");
    }

    /// Issue #835: two of Rakish Heir's triggers, one for the Heir and one
    /// for a Stromkirk Noble, were one Stack line — "put a +1/+1 counter on
    /// that Vampire (#39) (your) (×2)" beside the Heir's id — and the seat
    /// read it as "Rakish Heir gets two counters". Each line names its
    /// Vampire.
    #[test]
    fn a_watchers_triggers_on_the_stack_say_which_creature_each_is_for() {
        use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
        let (mut state, registry) = view_for_contract_test();
        let you = PlayerId(0);
        let mut put = |name: &str| {
            let card_id = registry.get_id_by_name(name).unwrap();
            let data = registry.card_data(card_id).unwrap();
            let id = state.create_object(card_id, you, mtg_engine::types::Zone::Battlefield, data.power, data.toughness);
            state.get_object_mut(id).unwrap().name = name.into();
            (id, card_id)
        };
        let (heir, heir_card) = put("Rakish Heir");
        let (noble, _) = put("Stromkirk Noble");
        let text = registry.card_data(heir_card).unwrap().triggered_abilities[0].description.clone();
        for dealer in [heir, noble] {
            state.stack.push(mtg_engine::state::StackEntry::Trigger(PendingTrigger::new(
                TriggerSource::new(heir, heir_card, you, text.clone()),
                TriggerEvent::AnyCombatDamageToPlayer { dealer, damaged_player: PlayerId(1), amount: 2 })));
        }
        let view = GameView::for_player(&state, you, &registry);
        let body = LlmPlayer::format_state_body(&view);
        let stack = &body[body.find("Stack:\n").expect("a stack section")..];
        assert!(stack.contains(&format!("put a +1/+1 counter on Rakish Heir (#{})", heir.0)), "{stack}");
        assert!(stack.contains(&format!("put a +1/+1 counter on Stromkirk Noble (#{})", noble.0)), "{stack}");
        assert!(!stack.contains("that Vampire") && !stack.contains("(×2)"), "{stack}");
        // The Noble's trigger went on last, so it is on top: the section
        // lists it first, which is what the rules say of it. They said
        // "top last" (#870's review), so a seat in a response war read the
        // bottom entry as the next to resolve.
        let noble_line = stack.find(&format!("counter on Stromkirk Noble (#{})", noble.0)).unwrap();
        let heir_line = stack.find(&format!("counter on Rakish Heir (#{})", heir.0)).unwrap();
        assert!(noble_line < heir_line, "{stack}");
        assert!(GAME_RULES.contains("`Stack:` (top first"), "the rules say the order the section prints");
    }

    /// Issue #828: exile was the one public zone the prompt only counted
    /// ("3exile"). A seat holding Runic Repetition chose whether to cast it
    /// without seeing which of its cards it could return, and a target
    /// prompt naming an exiled card did not say whose it was.
    #[test]
    fn the_state_body_lists_both_players_exile() {
        let card = |id: u64, name: &str, owner: u8| mtg_engine::view::CardView {
            object_id: ObjectId(id),
            card_id: mtg_engine::ids::CardId(u32::try_from(id).unwrap()),
            name: name.to_string(),
            cost: None,
            supertypes: vec![],
            card_types: vec![mtg_engine::types::CardType::Instant],
            power: None,
            toughness: None,
            oracle_text: String::new(),
            owner: PlayerId(owner),
            flashback_costs: vec![],
        };
        let mut view = empty_view();
        view.exile = vec![card(23, "Think Twice", 0), card(40, "Dead Weight", 1)];
        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("Your exile:\n  Think Twice\n"), "{body}");
        assert!(body.contains("Opp exile:\n  Dead Weight\n"), "{body}");
        assert_eq!(LlmPlayer::obj_name(&view, ObjectId(23)), "Think Twice (#23) (in your exile)");
        assert_eq!(LlmPlayer::obj_name(&view, ObjectId(40)), "Dead Weight (#40) (in opponent's exile)");

        view.exile.clear();
        let body = LlmPlayer::format_state_body(&view);
        assert!(!body.contains("exile:"), "an empty exile is not listed: {body}");
    }

    /// Issue #460: the engine offers one action per (object, ability_index),
    /// so a permanent with more than one mana ability produced rows that
    /// were byte-identical and were not the same action — a dual land as
    /// two, a filter land as six. 10 of the 19 actions in one measured
    /// prompt were five identical `Tap <dual>` pairs, and the board line
    /// gave no help either. This is #118 on the prompt, which is a
    /// different surface from the screen: nothing in a CLI game can show
    /// it.
    #[test]
    fn two_mana_abilities_on_one_land_are_two_different_rows() {
        let you = PlayerId(0);
        let mut land = perm(9, "Clifftop Retreat", 0, 0, you);
        land.card_types = vec![CardType::Land];
        land.power = None;
        land.toughness = None;
        land.effective_power = None;
        land.effective_toughness = None;
        land.mana_abilities = vec![(0, "Add {R}".into()), (1, "Add {W}".into())];
        let mut view = empty_view();
        view.battlefield.push(land);

        let label = |i: usize| LlmPlayer::format_single_action(&view, &Action::ActivateManaAbility {
            object_id: ObjectId(9),
            ability_index: i,
        });
        assert_eq!(label(0), "Tap Clifftop Retreat: Add {R}");
        assert_eq!(label(1), "Tap Clifftop Retreat: Add {W}");
        assert_ne!(label(0), label(1),
            "two actions that make different mana are two different rows");

        // A permanent whose abilities the view did not describe still reads
        // as a tap for mana rather than as a bare name.
        let mut plain = perm(10, "Island", 0, 0, you);
        plain.card_types = vec![CardType::Land];
        plain.mana_abilities = vec![];
        view.battlefield.push(plain);
        assert_eq!(
            LlmPlayer::format_single_action(&view, &Action::ActivateManaAbility {
                object_id: ObjectId(10),
                ability_index: 0,
            }),
            "Tap Island for mana");
    }

    /// Issue #496: one declare-blockers decision cost up to 20 model calls
    /// and then threw the whole answer away. The schema cannot express the
    /// constraint -- "two or more on this attacker, or none" is joint over
    /// several blockers, and each blocker's `enum` is built independently --
    /// so attempt 20 was byte-identical to attempt 1 and a deterministic
    /// seat repeated itself by construction: 80 of one game's 98 calls, and
    /// 85% of its prompt bytes, were retries of four decisions, while both
    /// of the program's bounds count decisions and saw one. Then the
    /// fallback declared NO blocks, discarding the legal blocks in the same
    /// answer -- which the engine, handed the identical declaration, keeps.
    #[test]
    fn an_unusable_blocker_answer_is_repaired_once_not_re_asked_twenty_times() {
        let you = PlayerId(0);
        let menace = perm(20, "Terror of Kruin Pass", 3, 3, PlayerId(1));
        let plain = perm(21, "Goblin Piker", 2, 1, PlayerId(1));
        let mut view = empty_view();
        view.battlefield.push(menace);
        view.battlefield.push(plain);
        for id in [30u64, 31, 32] {
            view.battlefield.push(perm(id, "Darkthicket Wolf", 2, 2, you));
        }

        let attackers = [ObjectId(20), ObjectId(21)];
        let blockers = [ObjectId(30), ObjectId(31), ObjectId(32)];
        let legal_blocks: HashMap<ObjectId, Vec<ObjectId>> = blockers.iter()
            .map(|&b| (b, attackers.to_vec()))
            .collect();
        // Only the menace attacker has a minimum, and it is 2.
        let min_blockers: HashMap<ObjectId, u32> = HashMap::from([(ObjectId(20), 2)]);

        // One blocker on the menace attacker (illegal on its own), one good
        // block on the other attacker, one declining.
        let (mut player, prompts) = recording_player_answering(serde_json::json!({
            "thoughts": "t", "blocks": [{"blocker": 0, "attacker": 0}, {"blocker": 1, "attacker": 1}],
        }));
        let action = player.choose_blockers_structured(
            &view, &blockers, &attackers, &legal_blocks, &min_blockers);

        // Asked once, corrected once, and then no more: the schema is
        // identical every time, so further re-asks buy nothing.
        assert_eq!(prompts.borrow().len(), 2,
            "one corrective re-ask, not twenty: {} calls", prompts.borrow().len());

        // And the legal block survives. Declaring nothing at all left the
        // seat worse off for having answered than for saying nothing.
        let Action::DeclareBlockers { assignments } = action else {
            panic!("a blocker decision declares blockers");
        };
        assert_eq!(assignments, vec![(ObjectId(31), ObjectId(21))],
            "the under-minimum pair is dropped and the legal block kept, \
             which is what the engine does with the same answer (#72)");
    }

    /// The blockers schema grows with the two lists, not their product: it
    /// was one property per blocker, each an enum of the attackers it could
    /// block — 95,000 values at 1,000 permanents (#642). And a pair the
    /// board does not allow, which the schema can no longer rule out, is
    /// refused rather than declared.
    #[test]
    fn the_blockers_schema_grows_with_the_creatures_not_their_pairs() {
        let size = |b: usize, a: usize| LlmPlayer::blockers_schema(b, a).to_string().len();
        let small = size(10, 10);
        let large = size(100, 100);
        assert!(large < small * 20,
            "10x the creatures must not be 100x the schema: {small} -> {large} bytes");

        let attackers = [ObjectId(20), ObjectId(21)];
        let blockers = [ObjectId(30), ObjectId(31)];
        // Blocker 30 cannot block the flier 21.
        let legal: HashMap<ObjectId, Vec<ObjectId>> = HashMap::from([
            (ObjectId(30), vec![ObjectId(20)]),
            (ObjectId(31), vec![ObjectId(20), ObjectId(21)]),
        ]);
        let text = LlmPlayer::block_reach_text(&blockers, &attackers, &legal);
        assert!(text.contains("blockers 0: attackers 0") && text.contains("blockers 1: any attacker"), "{text}");
        let (kept, errors) = LlmPlayer::parse_blocks(&serde_json::json!({"blocks": [
            {"blocker": 0, "attacker": 1}, {"blocker": 1, "attacker": 1}, {"blocker": 1, "attacker": 0},
            {"blocker": 5, "attacker": 0}]}), &blockers, &attackers, &legal);
        assert_eq!(kept, vec![(ObjectId(31), ObjectId(21))]);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    /// The repair is the engine's rule, not a second copy of it: two
    /// blockers on a menace attacker is a legal declaration and must
    /// survive untouched.
    #[test]
    fn a_gang_block_that_meets_the_minimum_is_not_repaired_away() {
        let kept = mtg_engine::combat::partition_under_minimum_blocks(
            &[(ObjectId(30), ObjectId(20)), (ObjectId(31), ObjectId(20)),
              (ObjectId(32), ObjectId(21))],
            |attacker| if attacker == ObjectId(20) { 2 } else { 1 },
        );
        assert_eq!(kept.0.len(), 3, "every pair stands: {kept:?}");
        assert!(kept.1.is_empty(), "nothing dropped: {kept:?}");
    }

    /// Issue #495: the pile-division prompt dropped the two facts that
    /// decide the answer. It passed `legal.context` — "<source>: divide
    /// into piles" — instead of the engine's `description`, which names
    /// whose board is being divided, and it never said that the target
    /// player then sacrifices a pile. Dividing the opponent's board you
    /// want both piles equally painful; dividing your own you want one
    /// pile empty and sacrifice that one. A seat handed the old prompt
    /// had to supply Liliana's rules text from memory and split 12/12.
    ///
    /// Same twenty lines: every non-creature read `Name (#id) 0/0`,
    /// because the rows came from the combat formatter. Liliana's −6
    /// divides ALL permanents.
    #[test]
    fn the_pile_division_prompt_says_whose_board_and_who_sacrifices() {
        let you = PlayerId(0);
        let opponent = PlayerId(1);
        let mut swamp = perm(9, "Swamp", 0, 0, you);
        swamp.card_types = vec![CardType::Land];
        swamp.power = None;
        swamp.toughness = None;
        swamp.effective_power = None;
        swamp.effective_toughness = None;
        let bear = perm(10, "Grizzly Bears", 2, 2, you);
        let view = {
            let mut v = empty_view();
            v.battlefield.push(swamp);
            v.battlefield.push(bear);
            v
        };
        let ids = [ObjectId(9), ObjectId(10)];
        let description = "Liliana of the Veil -6: divide p1's permanents into two piles";

        let ask = |target: PlayerId| {
            let (mut player, prompts) = recording_player();
            player.choose_pile_division(&view, &ids, description, target);
            let recorded = prompts.borrow()[0].clone();
            recorded
        };

        // Whose permanents these are, in the seat's own vocabulary.
        let dividing_theirs = ask(opponent);
        assert!(dividing_theirs.contains("divide opp's permanents"),
            "the engine's description reaches the seat, p-rewritten (#465):\n{dividing_theirs}");
        assert!(!dividing_theirs.contains("p1"),
            "no engine-global player labels:\n{dividing_theirs}");

        // And that a pile is sacrificed, by whom.
        let dividing_mine = ask(you);
        for prompt in [&dividing_theirs, &dividing_mine] {
            assert!(prompt.contains("sacrifice"),
                "the prompt says a pile is sacrificed:\n{prompt}");
        }
        assert!(dividing_mine.contains("You then choose"),
            "dividing your own board, you pick the pile that dies:\n{dividing_mine}");
        assert!(dividing_theirs.contains("Your opponent then chooses"),
            "dividing theirs, they pick:\n{dividing_theirs}");
        assert_ne!(dividing_mine, dividing_theirs,
            "the two cases ask for opposite answers, so they cannot read alike");

        // The land is not a 0/0; the creature still has its P/T.
        let rows: Vec<&str> = dividing_mine.lines()
            .filter(|l| l.starts_with("  0: ") || l.starts_with("  1: ")).collect();
        assert_eq!(rows, vec!["  0: Swamp (#9)", "  1: Grizzly Bears (#10) 2/2"],
            "a permanent with no P/T does not print one:\n{dividing_mine}");
    }

    /// Issue #494: `format_single_action` had no arm for
    /// `ActivateLoyaltyAbility`, so the row fell through to the engine's
    /// `Display` — `Activate loyalty ability 2 on obj#1` — the only label
    /// shape in a whole night's harvest that did. It named neither the
    /// planeswalker, nor what the ability costs or does, nor its target;
    /// and since the engine enumerates one action per target, Liliana's −6
    /// aimed at the seat and the same −6 aimed at its opponent were two
    /// byte-identical rows. The seat took the wrong one and sacrificed its
    /// own board. This is #61 on the prompt, which the CLI fixed and the
    /// LLM table never heard about.
    #[test]
    fn a_loyalty_ability_row_names_the_ability_and_its_target() {
        let you = PlayerId(0);
        let opponent = PlayerId(1);
        let mut lili = perm(1, "Liliana of the Veil", 0, 0, you);
        lili.card_types = vec![CardType::Planeswalker];
        lili.power = None;
        lili.toughness = None;
        lili.effective_power = None;
        lili.effective_toughness = None;
        lili.loyalty_abilities = vec![
            (0, "+1: Each player discards a card.".into()),
            (1, "−2: Target player sacrifices a creature.".into()),
            (2, "−6: Separate all permanents target player controls into two piles.".into()),
        ];
        let mut view = empty_view();
        view.battlefield.push(lili);

        let label = |index: usize, targets: Vec<mtg_engine::actions::Target>| {
            LlmPlayer::format_single_action(&view, &Action::ActivateLoyaltyAbility {
                object_id: ObjectId(1),
                ability_index: index,
                targets,
            })
        };

        // The ability is named, not numbered.
        let plus_one = label(0, vec![]);
        assert!(plus_one.contains("Liliana of the Veil") && plus_one.contains("+1: Each player discards"),
            "the row names the permanent and the ability: {plus_one}");
        assert!(!plus_one.contains("loyalty ability 0"),
            "the index is not what the row says: {plus_one}");

        // The two −6s differ, because the target is the whole decision.
        let at_you = label(2, vec![mtg_engine::actions::Target::Player(you)]);
        let at_opp = label(2, vec![mtg_engine::actions::Target::Player(opponent)]);
        assert_ne!(at_you, at_opp,
            "one action per target: two targets are two rows, not one row twice");
        assert!(at_you.contains("You") && at_opp.contains("Opponent"),
            "each row says who it is aimed at: {at_you:?} / {at_opp:?}");

        // A permanent the view did not describe still reads as an action.
        let mut plain = perm(2, "Garruk Relentless", 0, 0, you);
        plain.card_types = vec![CardType::Planeswalker];
        plain.loyalty_abilities = vec![];
        view.battlefield.push(plain);
        assert!(
            LlmPlayer::format_single_action(&view, &Action::ActivateLoyaltyAbility {
                object_id: ObjectId(2),
                ability_index: 1,
                targets: vec![],
            }).contains("Garruk Relentless"),
            "the fallback still names the permanent");
    }

    /// #746: a tap plan groups like sources again, by the bare name the
    /// `Tap <land>` rows use, and GAME_RULES shows what is sent.
    #[test]
    fn a_tap_plan_groups_like_sources_by_name() {
        let mut view = empty_view();
        for id in [1, 2] {
            view.battlefield.push(perm(id, "Island", 0, 0, PlayerId(0)));
        }
        view.battlefield.push(perm(3, "Hinterland Harbor", 0, 0, PlayerId(0)));
        let plan = [(ObjectId(1), 0), (ObjectId(2), 0), (ObjectId(3), 0)];
        assert_eq!(LlmPlayer::format_tap_plan(&view, &plan), "2x Island, Hinterland Harbor");
        let two = LlmPlayer::format_tap_plan(&view, &plan[..2]);
        assert!(GAME_RULES.contains(&format!("(tap {two})")), "GAME_RULES shows a grouped plan: (tap {two})");
    }

    fn perm(id: u64, name: &str, power: i32, toughness: i32, controller: PlayerId) -> PermanentView {
        PermanentView {
            object_id: ObjectId(id),
            card_id: CardId(0),
            name: name.into(),
            supertypes: vec![],
            card_types: vec![CardType::Creature],
            controller,
            owner: controller,
            tapped: false,
            power: Some(power),
            toughness: Some(toughness),
            effective_power: Some(power),
            effective_toughness: Some(toughness),
            damage_marked: 0,
            damaged_by: Vec::new(),
            regeneration_shields: 0,
            affected_by_summoning_sickness: false,
            attached_to: None,
            attached_to_player: None,
            keywords: vec![],
            colors: vec![],
            subtypes: vec![],
            printed_power: None,
            printed_toughness: None,
            star_pt: false,
            is_token: false,
            is_copy: false,
            protections: vec![],
            restrictions: vec![],
            granted_abilities: vec![],
            attacking: None,
            blocking: vec![],
            blocked_by: vec![],
            blocked: false,
            oracle_text: String::new(),
            counters: HashMap::new(),
            loyalty_abilities: vec![],
            mana_abilities: vec![],
            named_card: None,
        }
    }

    /// A Curse: an Aura attached to a *player* rather than an object.
    fn curse(id: u64, name: &str, enchanted: PlayerId, controller: PlayerId, oracle: &str) -> PermanentView {
        let mut p = perm(id, name, 0, 0, controller);
        p.card_types = vec![CardType::Enchantment];
        p.power = None;
        p.toughness = None;
        p.effective_power = None;
        p.effective_toughness = None;
        p.attached_to_player = Some(enchanted);
        p.oracle_text = oracle.into();
        p
    }

    /// Issue #468: the seat had no way at all to learn a regeneration shield
    /// was up. Not from the board text, which carried every other flag, and
    /// not from the log, which mentions a shield only when it is spent — so
    /// a seat reading "creature, 3/3" was pricing removal against a creature
    /// that survives it.
    #[test]
    fn a_live_regeneration_shield_is_in_the_board_text() {
        let you = PlayerId(0);
        let mut corpse = perm(58, "Walking Corpse", 3, 3, you);

        let perms = vec![&corpse];
        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(!output.contains("regen"), "no shield, nothing said: {output}");

        corpse.regeneration_shields = 1;
        let perms = vec![&corpse];
        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(output.contains("regen shield"), "got {output}");

        corpse.regeneration_shields = 2;
        let perms = vec![&corpse];
        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(output.contains("2 regen shields"), "a second shield stacks: {output}");
    }

    /// Issue #506: `PermanentView::protections` exists precisely because
    /// protection is not a `Keyword` in this engine and cannot ride in
    /// `keywords` — and both LLM board renderers called `format_keywords`,
    /// so the field #243/#297 added for exactly this reached the CLI and no
    /// model seat. Elite Inquisitor's printed protections never appeared on
    /// the row, and Spare from Evil's timed one is representable nowhere
    /// else at all, so it was invisible on both boards. A seat picking
    /// blocks and targets was doing it from a bare name, P/T and keyword
    /// list.
    ///
    /// Issue #504's restrictions are the same gap on the sibling field, and
    /// both renderers are checked here because reaching one of the two is
    /// how this happened in the first place.
    #[test]
    fn protections_and_restrictions_reach_both_llm_board_renderers() {
        let you = PlayerId(0);
        let mut inq = perm(90, "Elite Inquisitor", 2, 2, you);
        inq.keywords = vec![mtg_engine::types::Keyword::FirstStrike];
        inq.protections = vec!["protection from Vampires".into(),
                               "protection from Werewolves".into()];
        inq.restrictions = vec!["can't block".into()];

        let perms = vec![&inq];
        let board = LlmPlayer::format_perms_compact(&perms, &perms, you);
        for said in ["first strike", "protection from Vampires",
                     "protection from Werewolves", "can't block"] {
            assert!(board.contains(said), "the board omits {said:?}: {board}");
        }

        let mut view = empty_view();
        view.battlefield = vec![inq.clone()];
        let row = LlmPlayer::format_combat_creature(&view, ObjectId(90));
        for said in ["first strike", "protection from Vampires", "can't block"] {
            assert!(row.contains(said), "the combat row omits {said:?}: {row}");
        }

        // A creature with neither says neither, on both.
        let plain = perm(91, "Walking Corpse", 2, 2, you);
        let perms = vec![&plain];
        let board = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(!board.contains("protection") && !board.contains("can't"), "got {board}");
    }

    /// `[S]` is the engine's CR 302.6 answer, and both board renderers say
    /// it only when the engine does.
    ///
    /// The flag used to be the raw "entered this turn" view field, so a
    /// haste creature was handed to the seat as `Manor Skeleton (#3) 1/1
    /// black, haste [S]` while the same request's legend defined `S` as
    /// "summoning sick (entered this turn, can't attack)" and the next
    /// section offered it as a legal attacker. One row saying both things
    /// about one creature (#605, the harness's half of #139). The seat
    /// talked itself past it, which is why nobody noticed and not a reason
    /// it was harmless.
    #[test]
    fn the_sickness_flag_says_what_the_engine_says_on_both_board_renderers() {
        let you = PlayerId(0);
        for restricted in [false, true] {
            let mut c = perm(3, "Manor Skeleton", 1, 1, you);
            c.keywords = vec![mtg_engine::types::Keyword::Haste];
            c.affected_by_summoning_sickness = restricted;

            let perms = vec![&c];
            let board = LlmPlayer::format_perms_compact(&perms, &perms, you);
            assert!(board.contains("haste"), "the row still says haste: {board}");
            assert_eq!(board.contains("[S]"), restricted,
                "restricted={restricted} but the board says {board}");

            let mut view = empty_view();
            view.battlefield = vec![c.clone()];
            let row = LlmPlayer::format_combat_creature(&view, ObjectId(3));
            assert!(!row.contains("[S]") || restricted,
                "restricted={restricted} but the combat row says {row}");
        }
    }

    /// CR 706.2: an ability a copy effect added is on neither surface the
    /// seat can read. The row is built from the COPIED card's name and
    /// characteristics, and the seat looks ability text up in its own
    /// decklist — which holds the copied card's text, not the clone's. So
    /// an Evil Twin's destroy ability was offered in the action list and
    /// described nowhere (the harness half of issue #501).
    #[test]
    fn an_ability_a_copy_effect_granted_is_in_the_board_text() {
        let you = PlayerId(0);
        let mut clone = perm(33, "Merciless Predator", 3, 2, you);

        let perms = vec![&clone];
        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(!output.contains("also has"), "nothing granted, nothing said: {output}");

        clone.granted_abilities = vec![
            "{U}{B}, {T}: Destroy target creature with the same name".into()];
        let perms = vec![&clone];
        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        assert!(output.contains("also has: {U}{B}, {T}: Destroy target creature with the same name"),
            "got {output}");
    }

    /// A Curse's whole identity is whom it enchants (CR 702.5c), and the
    /// prompt never said. It attaches to a player, so it is not in the aura
    /// map (keyed on objects) and fell through to the plain "other
    /// permanents" line; `short_effect_summary` drops the "Enchant player"
    /// line, so the text that survived said "enchanted player" with no
    /// antecedent. Two same-named Curses on opposite players read
    /// identically, and the controller is no proxy — a seat can curse
    /// itself. The CLI has said this since #81; the prompt now does too.
    #[test]
    fn a_curse_says_which_player_it_enchants() {
        let you = PlayerId(0);
        let opp = PlayerId(1);
        let oracle = "Enchant player\nAt the beginning of enchanted player's upkeep, \
this Aura deals 1 damage to that player.";

        let on_you = curse(27, "Curse of the Pierced Heart", you, you, oracle);
        let on_opp = curse(28, "Curse of the Pierced Heart", opp, you, oracle);
        let perms = vec![&on_you, &on_opp];

        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);

        assert!(output.contains("(#27)") && output.contains("(#28)"),
            "both curses are listed: {output}");
        let line_27 = output.lines().find(|l| l.contains("(#27)")).expect("curse 27 on a line");
        let line_28 = output.lines().find(|l| l.contains("(#28)")).expect("curse 28 on a line");
        assert!(line_27.contains("[enchanting you]"),
            "a curse on the viewing seat says so: {line_27}");
        assert!(line_28.contains("[enchanting opponent]"),
            "a curse on the other seat says so: {line_28}");
        assert_ne!(line_27, line_28,
            "two same-named curses on opposite players must not render identically");

        // And as a target, where the label used to name the controller only
        // (#736's propagation review).
        let mut view = empty_view();
        view.battlefield.push(on_you.clone());
        view.battlefield.push(on_opp.clone());
        assert_eq!(LlmPlayer::obj_name(&view, ObjectId(27)),
            "Curse of the Pierced Heart (#27) (your, enchanting you)");
        assert_eq!(LlmPlayer::obj_name(&view, ObjectId(28)),
            "Curse of the Pierced Heart (#28) (your, enchanting opponent)");
    }

    /// Issue #325: an ordering response is a permutation of the offered
    /// indices or it is nothing — a duplicate, a gap or an index out of
    /// range falls back to the listed order rather than a partial one.
    #[test]
    fn an_order_response_is_a_permutation_or_nothing() {
        let ok = serde_json::json!([2, 0, 1]);
        assert_eq!(LlmPlayer::parse_order_response(&ok, 3), Some(vec![2, 0, 1]));
        for bad in [serde_json::json!([0, 1]), serde_json::json!([0, 1, 1]), serde_json::json!([0, 1, 3]),
                    serde_json::json!([0, 1, 2, 0]), serde_json::json!("2 0 1"), serde_json::json!(null),
                    serde_json::json!([0, -1, 2])] {
            assert_eq!(LlmPlayer::parse_order_response(&bad, 3), None, "{bad}");
        }
        assert_eq!(LlmPlayer::parse_order_response(&serde_json::json!([]), 0), Some(vec![]));
    }

    /// Issue #737: a line whose subject becomes "You" conjugates whatever
    /// its verb is, not one of a listed six; a token after a preposition is
    /// not a subject; and a line opening with a possessive is capitalised.
    #[test]
    fn any_verb_after_you_is_conjugated() {
        let you = PlayerId(0);
        let w = |l: &str| LlmPlayer::generic_player_rewrite(l, you);
        assert_eq!(w("Night Terrors: p0 reveals hand: Plains"), "Night Terrors: You reveal hand: Plains");
        assert_eq!(w("Heretic's Punishment: p0 has an empty library"), "Heretic's Punishment: You have an empty library");
        assert_eq!(w("Forbidden Alchemy: p0 looks at the top 4 cards"), "Forbidden Alchemy: You look at the top 4 cards");
        assert_eq!(w("a counter on each creature p0 controls"), "a counter on each creature You control");
        assert_eq!(w("p0 gains control of Olivia Voldaren"), "You gain control of Olivia Voldaren");
        assert_eq!(w("p0 searches, p0 copies it, p0 passes"), "You search, You copy it, You pass");
        assert_eq!(w("p0 keeps (0 mulligans)"), "You keep (0 mulligans)");
        // The opponent's verb, a past tense, and an object are left alone.
        assert_eq!(w("p1 reveals hand"), "Opp reveals hand");
        assert_eq!(w("p0 cast Doom Blade"), "You cast Doom Blade");
        assert_eq!(w("deals 3 damage to p0 ones"), "deals 3 damage to You ones");
        // A possessive opening the line is capitalised.
        assert_eq!(w("p0's Ashmouth Hound (#38)'s block trigger"), "Your Ashmouth Hound (#38)'s block trigger");
        assert_eq!(super::base_form("dies").as_deref(), Some("die"));
        assert_eq!(super::base_form("its"), None);
        assert_eq!(super::base_form("less"), None);
    }

    /// Issue #713: the pool-emptying line (#708) read "your unspent mana
    /// empties from their pool" once the seat's token was rewritten. The
    /// engine now names the pool once, as a possessive every surface turns
    /// into a sentence.
    #[test]
    fn the_pool_emptying_line_reads_as_one_owner() {
        let you = PlayerId(0);
        assert_eq!(LlmPlayer::generic_player_rewrite("p0's mana pool empties (Green:1)", you),
            "Your mana pool empties (Green:1)");
        assert_eq!(LlmPlayer::generic_player_rewrite("p1's mana pool empties (Black:2)", you),
            "Opp's mana pool empties (Black:2)");
    }

    /// Issue #724: the board says who is attacking and who is blocking whom.
    /// "Recent events" is a delta since the last prompt, so from the second
    /// prompt of a combat step the board was the only place left to say it,
    /// and it said only `[T]`.
    #[test]
    fn the_board_marks_attackers_and_blockers() {
        use mtg_engine::view::AttackTarget;
        let you = PlayerId(0);
        let opp = PlayerId(1);
        let mut tusker = perm(30, "Kalonian Tusker", 3, 3, you);
        tusker.tapped = true;
        tusker.attacking = Some(AttackTarget::Player(opp));
        tusker.blocked_by = vec![ObjectId(45)];
        let mut lions = perm(45, "Savannah Lions", 2, 1, opp);
        lions.blocking = vec![ObjectId(30)];
        let mut vigilant = perm(31, "Abbey Griffin", 2, 2, you);
        vigilant.attacking = Some(AttackTarget::Player(opp));
        let idle = perm(32, "Grizzly Bears", 2, 2, you);
        let all = vec![&tusker, &lions, &vigilant, &idle];

        let mine = LlmPlayer::format_perms_compact(&[&tusker, &vigilant, &idle], &all, you);
        let theirs = LlmPlayer::format_perms_compact(&[&lions], &all, you);
        let line = |out: &str, id: u64| out.lines().find(|l| l.contains(&format!("(#{id}) ")))
            .unwrap_or_else(|| panic!("#{id} on a line: {out}")).to_string();
        assert!(line(&mine, 30).contains("attacking Opp"), "{}", line(&mine, 30));
        assert!(line(&mine, 30).contains("blocked by Savannah Lions (#45)"), "{}", line(&mine, 30));
        assert!(line(&mine, 31).contains("attacking Opp"), "an untapped attacker is not idle: {}", line(&mine, 31));
        assert!(!line(&mine, 32).contains("attacking"), "{}", line(&mine, 32));
        assert!(line(&theirs, 45).contains("blocking Kalonian Tusker (#30)"), "{}", line(&theirs, 45));
        // Seen from the other seat, the same attacker is attacking "you".
        let other = LlmPlayer::format_perms_compact(&[&tusker], &all, opp);
        assert!(other.contains("attacking you"), "{other}");
    }

    /// Issue #740: a stolen creature read like its controller's own copies —
    /// `Vampire Interloper (#22) 2/1 black, flying, can't block` beside four
    /// more — and a stolen land was counted among the controller's lands by
    /// name. Which one goes back when the control effect ends (CR 611.2b),
    /// and to whose graveyard (CR 404.1), could not be read. The board says
    /// whose each is when it is not its controller's.
    #[test]
    fn the_board_says_whose_a_stolen_permanent_is() {
        let you = PlayerId(0);
        let opp = PlayerId(1);
        let own = perm(71, "Vampire Interloper", 2, 1, you);
        let mut stolen = perm(22, "Vampire Interloper", 2, 1, you);
        stolen.owner = opp;
        let land = |id: u64, owner: PlayerId, controller: PlayerId| {
            let mut l = perm(id, "Forest", 0, 0, controller);
            l.card_types = vec![CardType::Land];
            l.owner = owner;
            l
        };
        let (f1, f2, f3) = (land(1, you, you), land(2, you, you), land(3, opp, you));
        let mut torch = perm(40, "Blazing Torch", 0, 0, opp);
        torch.card_types = vec![CardType::Artifact];
        torch.owner = you;
        let mine = vec![&own, &stolen, &f1, &f2, &f3];
        let all = vec![&own, &stolen, &f1, &f2, &f3, &torch];
        let board = LlmPlayer::format_perms_compact(&mine, &all, you);
        let line = |id: u64| board.lines().find(|l| l.contains(&format!("(#{id}) ")))
            .unwrap_or_else(|| panic!("#{id} on a line: {board}")).to_string();
        assert!(line(22).contains("owned by opponent"), "{board}");
        assert!(!line(71).contains("owned by"), "{board}");
        assert!(board.contains("2x Forest\n") && board.contains("1x Forest [owned by opponent]"),
            "the stolen Forest is not one of the controller's: {board}");
        // From the owner's seat, the same permanents are its own.
        let theirs = LlmPlayer::format_perms_compact(&[&torch], &all, opp);
        assert!(theirs.contains("[owned by opponent]"), "{theirs}");
        let seen_by_owner = LlmPlayer::format_perms_compact(&[&torch], &all, you);
        assert!(seen_by_owner.contains("[owned by you]"), "{seen_by_owner}");
    }

    /// Issue #932: after combat the board said `[3dmg]` and not that an
    /// Abattoir Ghoul still on the battlefield dealt it, which decides
    /// whether killing the creature gains life; that lived only in the
    /// one prompt whose recent events held the damage line.
    #[test]
    fn the_board_names_the_permanents_that_dealt_a_creature_its_damage() {
        let (you, opp) = (PlayerId(0), PlayerId(1));
        let ghoul = perm(6, "Abattoir Ghoul", 3, 2, you);
        let mut crab = perm(44, "Fortress Crab", 1, 6, opp);
        crab.damage_marked = 3;
        // #90 was a spell, gone from the battlefield: not named.
        crab.damaged_by = vec![ObjectId(6), ObjectId(90)];
        let mut burnt = perm(45, "Fortress Crab", 1, 6, opp);
        burnt.damage_marked = 2;
        burnt.damaged_by = vec![ObjectId(90)];
        let owned = vec![ghoul, crab, burnt];
        let all: Vec<&PermanentView> = owned.iter().collect();
        let board = LlmPlayer::format_perms_compact(&all, &all, you);
        assert!(board.contains("Fortress Crab (#44) 1/6 colorless [3dmg,damaged this turn by Abattoir Ghoul (#6)]"), "{board}");
        assert!(board.contains("Fortress Crab (#45) 1/6 colorless [2dmg]"), "{board}");
        // Regenerated: the damage is gone, and the Ghoul still triggers
        // when it dies this turn (destruction.rs `regenerate`).
        let mut regenerated = owned[1].clone();
        regenerated.damage_marked = 0;
        let owned = vec![owned[0].clone(), regenerated];
        let all: Vec<&PermanentView> = owned.iter().collect();
        let board = LlmPlayer::format_perms_compact(&all, &all, you);
        assert!(board.contains("Fortress Crab (#44) 1/6 colorless [damaged this turn by Abattoir Ghoul (#6)]"), "{board}");
    }

    /// Issue #726: every flag the board prints is one the legend defines,
    /// in the spelling it prints — the legend documented counters as
    /// `+1/+1:N` while the board printed `+1+1xN`, and never mentioned
    /// `token`, `copy`, `regen shield` or `names:`.
    #[test]
    fn the_board_legend_defines_every_flag_the_board_prints() {
        use mtg_engine::view::AttackTarget;
        let you = PlayerId(0);
        let opp = PlayerId(1);
        let mut a = perm(30, "Kalonian Tusker", 3, 3, you);
        a.tapped = true;
        a.attacking = Some(AttackTarget::Player(opp));
        a.blocked_by = vec![ObjectId(45)];
        a.damage_marked = 2;
        a.damaged_by = vec![ObjectId(45)];
        a.regeneration_shields = 1;
        a.counters.insert(CounterType::PlusOnePlusOne, 1);
        a.counters.insert(CounterType::MinusOneMinusOne, 1);
        a.is_token = true;
        let mut alone = perm(31, "Walking Corpse", 2, 2, you);
        alone.attacking = Some(AttackTarget::Player(opp));
        alone.blocked = true;
        let mut b = perm(45, "Savannah Lions", 2, 1, opp);
        b.blocking = vec![ObjectId(30)];
        b.affected_by_summoning_sickness = true;
        b.is_copy = true;
        b.regeneration_shields = 2;
        let mut c = perm(50, "Nevermore", 0, 0, you);
        c.card_types = vec![CardType::Enchantment];
        c.power = None;
        c.toughness = None;
        c.effective_power = None;
        c.effective_toughness = None;
        c.named_card = Some("Geistflame".into());
        // A Curse on each player, one of them with a flag of its own (#736).
        let oracle = "Enchant player\nAt the beginning of enchanted player's upkeep, \
this Aura deals 1 damage to that player.";
        let mut d = curse(51, "Curse of the Pierced Heart", you, opp, oracle);
        d.tapped = true;
        let e = curse(52, "Curse of the Pierced Heart", opp, you, oracle);
        // A stolen creature and a stolen land (#740).
        let mut f = perm(53, "Vampire Interloper", 2, 1, you);
        f.owner = opp;
        let mut g = perm(54, "Forest", 0, 0, you);
        g.card_types = vec![CardType::Land];
        g.owner = opp;
        let all = vec![&a, &alone, &b, &c, &d, &e, &f, &g];
        let board = LlmPlayer::format_perms_compact(&all, &all, you);
        assert!(board.contains("blocked (no blockers left)"), "{board}");
        assert!(board.contains("enchanting you") && board.contains("enchanting opponent"), "{board}");
        assert!(board.contains("1x Forest [owned by opponent]"), "{board}");
        let legend_start = GAME_RULES.find("Status flags after a permanent").expect("the legend");
        let legend = &GAME_RULES[legend_start..(legend_start + 2500).min(GAME_RULES.len())];
        for line in board.lines() {
            // One bracket per line: a second one is a flag outside the
            // bracket the legend describes (#736).
            assert!(line.matches(" [").count() <= 1, "more than one bracket: {line:?}");
            let Some(open) = line.rfind(" [") else { continue };
            let close = line[open..].find(']').map_or(line.len(), |i| open + i);
            for flag in line[open + 2..close].split(',') {
                // The defining part of a flag: its words before any count or name.
                let key = match flag {
                    f if f.starts_with("attacking") => "`attacking",
                    f if f.starts_with("blocking") => "`blocking",
                    f if f.starts_with("blocked by") => "`blocked by",
                    f if f.starts_with("names:") => "`names:",
                    f if f.ends_with("dmg") => "`Ndmg`",
                    f if f.starts_with("damaged this turn by") => "`damaged this turn by",
                    f if f.ends_with("regen shields") => "`N regen shields`",
                    f if f.starts_with("+1+1x") => "`+1+1xN`",
                    f if f.starts_with("-1-1x") => "`-1-1xN`",
                    f => &format!("`{f}`"),
                };
                assert!(legend.contains(key), "board flag {flag:?} (from {line:?}) is not in the legend");
            }
        }
        assert!(!legend.contains("entered this turn"),
            "`S` lasts until the controller's next turn, not the turn it entered");
    }

    /// Issue #333: the board text never said a permanent was legendary, so a

    /// seat could not see the legend rule (CR 704.5j) coming. A legendary
    /// creature carries "legendary" with its keywords; any other legendary
    /// permanent carries it in its flags.
    #[test]
    fn a_legend_is_marked_on_the_board() {
        let you = PlayerId(0);
        let mut mikaeus = perm(24, "Mikaeus, the Lunarch", 1, 1, you);
        mikaeus.supertypes = vec![mtg_engine::types::Supertype::Legendary];
        mikaeus.keywords = vec![mtg_engine::types::Keyword::Flying];
        mikaeus.colors = vec![mtg_engine::types::Color::White];
        let mut bears = perm(25, "Grizzly Bears", 2, 2, you);
        bears.colors = vec![mtg_engine::types::Color::Green];
        let mut grimoire = perm(26, "Grimoire of the Dead", 0, 0, you);
        grimoire.card_types = vec![CardType::Artifact];
        grimoire.power = None;
        grimoire.toughness = None;
        grimoire.effective_power = None;
        grimoire.effective_toughness = None;
        grimoire.supertypes = vec![mtg_engine::types::Supertype::Legendary];
        let perms = vec![&mikaeus, &bears, &grimoire];

        let output = LlmPlayer::format_perms_compact(&perms, &perms, you);
        let line = |id: u64| output.lines().find(|l| l.contains(&format!("(#{id})")))
            .unwrap_or_else(|| panic!("#{id} on a line: {output}")).to_string();
        assert!(line(24).contains("1/1 legendary, white, flying"), "{}", line(24));
        assert!(!line(25).contains("legendary"), "{}", line(25));
        assert!(line(26).contains("[legendary]"), "{}", line(26));
    }

    fn aura(id: u64, name: &str, attached_to: u64, controller: PlayerId) -> PermanentView {

        PermanentView {
            object_id: ObjectId(id),
            card_id: CardId(0),
            name: name.into(),
            supertypes: vec![],
            card_types: vec![CardType::Enchantment],
            controller,
            owner: controller,
            tapped: false,
            power: None,
            toughness: None,
            effective_power: None,
            effective_toughness: None,
            damage_marked: 0,
            damaged_by: Vec::new(),
            regeneration_shields: 0,
            affected_by_summoning_sickness: false,
            attached_to: Some(ObjectId(attached_to)),
            attached_to_player: None,
            keywords: vec![],
            colors: vec![],
            subtypes: vec![],
            printed_power: None,
            printed_toughness: None,
            star_pt: false,
            is_token: false,
            is_copy: false,
            protections: vec![],
            restrictions: vec![],
            granted_abilities: vec![],
            attacking: None,
            blocking: vec![],
            blocked_by: vec![],
            blocked: false,
            oracle_text: String::new(),
            counters: HashMap::new(),
            loyalty_abilities: vec![],
            mana_abilities: vec![],
            named_card: None,
        }
    }

    #[test]
    fn disambiguate_unique_names_unchanged() {
        let mut view = empty_view();
        view.battlefield.push(perm(1, "Grizzly Bears", 2, 2, PlayerId(0)));
        view.battlefield.push(perm(2, "Llanowar Elves", 1, 1, PlayerId(0)));
        let labels = LlmPlayer::format_combat_creature_list(&view, &[ObjectId(1), ObjectId(2)]);
        assert_eq!(labels[0], "Grizzly Bears (#1) 2/2");
        assert_eq!(labels[1], "Llanowar Elves (#2) 1/1");
    }

    #[test]
    fn disambiguate_identical_names_get_ids() {
        let mut view = empty_view();
        view.battlefield.push(perm(10, "Rakish Heir", 4, 2, PlayerId(0)));
        view.battlefield.push(perm(11, "Rakish Heir", 4, 2, PlayerId(0)));
        view.battlefield.push(perm(12, "Rakish Heir", 4, 2, PlayerId(0)));
        let labels = LlmPlayer::format_combat_creature_list(
            &view,
            &[ObjectId(10), ObjectId(11), ObjectId(12)],
        );
        // Each gets a unique object ID — no extra disambiguation needed
        assert_eq!(labels[0], "Rakish Heir (#10) 4/2");
        assert_eq!(labels[1], "Rakish Heir (#11) 4/2");
        assert_eq!(labels[2], "Rakish Heir (#12) 4/2");
    }

    #[test]
    fn disambiguate_attached_aura_shown_inline() {
        let mut view = empty_view();
        view.battlefield.push(perm(20, "Rakish Heir", 4, 2, PlayerId(0)));
        view.battlefield.push(perm(21, "Rakish Heir", 4, 2, PlayerId(0)));
        view.battlefield.push(aura(22, "Bonds of Faith", 21, PlayerId(0)));

        let labels = LlmPlayer::format_combat_creature_list(
            &view,
            &[ObjectId(20), ObjectId(21)],
        );
        assert_eq!(labels[0], "Rakish Heir (#20) 4/2");
        assert_eq!(labels[1], "Rakish Heir (#21) 4/2 (attached: Bonds of Faith)");
    }

    #[test]
    fn disambiguate_partial_collision_all_get_ids() {
        let mut view = empty_view();
        view.battlefield.push(perm(30, "Grizzly Bears", 2, 2, PlayerId(0)));
        view.battlefield.push(perm(31, "Grizzly Bears", 2, 2, PlayerId(0)));
        view.battlefield.push(perm(32, "Llanowar Elves", 1, 1, PlayerId(0)));
        let labels = LlmPlayer::format_combat_creature_list(
            &view,
            &[ObjectId(30), ObjectId(31), ObjectId(32)],
        );
        assert_eq!(labels[0], "Grizzly Bears (#30) 2/2");
        assert_eq!(labels[1], "Grizzly Bears (#31) 2/2");
        assert_eq!(labels[2], "Llanowar Elves (#32) 1/1");
    }

    /// GAME_RULES describes the sacrifice-cost flow the seat really gets: one
    /// row, then a sacrifice prompt (#471). It went on saying "listed once
    /// per creature you could sacrifice" long after that stopped being true
    /// (issue #672).
    #[test]
    fn game_rules_describes_the_sacrifice_prompt_the_seat_is_sent() {
        assert!(!GAME_RULES.contains("listed once per creature you could sacrifice"),
            "the one-row-per-sacrifice listing is gone");
        // The follow-up prompt says what it asks; the system prompt promises
        // that of every prompt but the action list and combat.
        assert!(GAME_RULES.contains("Every other prompt — a target, a cost, X, an order, damage, a mulligan — says what it asks."));
    }

    /// Two objects that share a name in a public zone are never the same
    /// row: a land on the battlefield, a stack object and a graveyard card
    /// each carry their id and whose they are (issue #668), and a stack
    /// line puts the entry's controller by the entry rather than after its
    /// target's own tag (issue #671).
    #[test]
    fn same_named_objects_in_public_zones_are_told_apart_by_id_and_owner() {
        let (state, registry) = view_for_contract_test();
        let mut view = GameView::for_player(&state, PlayerId(0), &registry);
        let land = |id: u64, controller| {
            let mut p = perm(id, "Mountain", 0, 0, controller);
            p.card_types = vec![CardType::Land];
            p.power = None;
            p.toughness = None;
            p
        };
        view.battlefield = vec![land(12, PlayerId(0)), land(64, PlayerId(1))];
        let card = view.your_hand.first().cloned().expect("the opening hand was drawn");
        let in_yard = |id: u64| {
            let mut c = card.clone();
            c.object_id = ObjectId(id);
            c.name = "Dissipate".into();
            c
        };
        view.graveyards = vec![(PlayerId(0), vec![in_yard(28)]), (PlayerId(1), vec![in_yard(82)])];
        let spell = |id: u64, controller, targets| mtg_engine::view::StackItemView {
            object_id: ObjectId(id),
            card_id: CardId(0),
            name: "Geistflame".to_string(),
            source_id: Some(ObjectId(id)),
            controller,
            targets,
            x_value: None,
            cost: None, supertypes: vec![], card_types: vec![],
            power: None, toughness: None, oracle_text: String::new(), resolving: false,
        };
        view.stack = vec![
            spell(100, PlayerId(1), vec![mtg_engine::actions::Target::Object(ObjectId(12))]),
            spell(101, PlayerId(0), vec![]),
        ];

        let name = |id| LlmPlayer::obj_name(&view, ObjectId(id));
        assert_eq!(name(12), "Mountain (#12) (your)");
        assert_eq!(name(64), "Mountain (#64) (opponent's)");
        assert_eq!(name(100), "Geistflame (#100) (opponent's)");
        assert_eq!(name(101), "Geistflame (#101) (your)");
        assert_eq!(name(28), "Dissipate (#28) (in your graveyard)");
        assert_eq!(name(82), "Dissipate (#82) (in opponent's graveyard)");

        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("  Geistflame (#100) (opponent's) targeting Mountain (#12) (your)\n"),
            "the entry's controller sits by the entry, the target's by the target: {body}");

        // The spell resolving now is on the stack, and says so (#766).
        view.stack[1].resolving = true;
        let body = LlmPlayer::format_state_body(&view);
        assert!(body.contains("  Geistflame (#101) (your) — resolving\n"), "{body}");
    }

    #[test]
    fn disambiguate_different_pt_both_get_ids() {
        let mut view = empty_view();
        view.battlefield.push(perm(40, "Howlpack of Estwald", 4, 6, PlayerId(0)));
        view.battlefield.push(perm(41, "Howlpack of Estwald", 5, 7, PlayerId(0)));
        let labels = LlmPlayer::format_combat_creature_list(
            &view,
            &[ObjectId(40), ObjectId(41)],
        );
        assert_eq!(labels[0], "Howlpack of Estwald (#40) 4/6");
        assert_eq!(labels[1], "Howlpack of Estwald (#41) 5/7");
    }

    // ─────────────────────────────────────────────────────────────────
    // Audit failing tests — harness prompts
    // ─────────────────────────────────────────────────────────────────

    /// Bug 37-002 (`audits/AUDIT_BUGS.md)`: target-selection prompts use
    /// `obj_name`, which returns the raw object name with a
    /// controller suffix but no per-collision disambiguator. Two
    /// same-named creatures under the same controller collapse to
    /// identical strings — the LLM can't tell them apart.
    ///
    /// The fix is to create a `format_object_labels` helper modeled
    /// on `format_combat_creature_list` and route `prompt_target_selection`
    /// through it. For now, we assert the symptom: `obj_name` returns
    /// the same string for two distinct same-named creatures.
    ///
    /// This test asserts the EXPECTED CORRECT behavior, so it currently
    /// fails. It will start passing as soon as Bug 37-002 is fixed
    /// (either by `obj_name` gaining collision awareness, or by
    /// `prompt_target_selection` routing through a new disambiguator).
    #[test]
    fn bug_37_002_target_selection_disambiguates_identical_creatures() {
        let mut view = empty_view();
        view.battlefield.push(perm(50, "Champion of the Parish", 1, 1, PlayerId(0)));
        view.battlefield.push(perm(51, "Champion of the Parish", 1, 1, PlayerId(0)));

        let label_a = LlmPlayer::obj_name(&view, ObjectId(50));
        let label_b = LlmPlayer::obj_name(&view, ObjectId(51));

        assert_ne!(
            label_a, label_b,
            "obj_name (used by prompt_target_selection to render \
             target-choice labels) should produce distinct strings for \
             two same-named creatures under the same controller. Bug \
             37-002: both collapse to 'Champion of the Parish (your)', \
             so the LLM can't deliberately pick between index 0 and \
             index 1."
        );
    }

    /// Bug H10 (`audits/AUDIT_BUGS.md)`: The board-state display uses
    /// comma as both the keyword separator within a creature and the
    /// creature separator in a list, so a creature with multiple
    /// keywords runs into the next creature's name. Example:
    /// `Creature A, flying, Creature B` parses ambiguously.
    ///
    /// `format_perms_compact` generates this display. We check that
    /// when two creatures are present and the first one has a
    /// keyword, the output contains a clear separator that's
    /// distinguishable from the keyword list.
    #[test]
    fn bug_h10_board_display_distinguishes_keyword_and_creature_separators() {
        let mut view = empty_view();
        let mut p0 = perm(60, "Angel", 4, 4, PlayerId(0));
        p0.keywords = vec![mtg_engine::types::Keyword::Flying];
        view.battlefield.push(p0);
        view.battlefield.push(perm(61, "Grizzly Bears", 2, 2, PlayerId(0)));

        let perms: Vec<_> = view.battlefield.iter().collect();
        let output = LlmPlayer::format_perms_compact(&perms, &perms, PlayerId(0));

        let suspicious = output.contains("Flying, Grizzly Bears")
            || output.contains("flying, Grizzly Bears");
        assert!(
            !suspicious,
            "Board-state display uses comma as both the keyword \
             separator within a creature and the creature separator \
             between entries — 'Flying, Grizzly Bears' is ambiguous. \
             Bug H10. Got: {output:?}",
        );
    }

    /// Cast labels for spells with additional costs should surface
    /// the cost in the label (e.g. "exile a creature from GY").
    #[test]
    fn cast_label_includes_additional_cost() {
        use mtg_engine::actions::{CastTargetSpec, CastableSpell};

        let cs = CastableSpell {
            object_id: ObjectId(200),
            name: "Stitched Drake".into(),
            is_flashback: false,
            from_graveyard: false,
            target_spec: CastTargetSpec::NoTargets,
            tap_plan: vec![],
            exile_x_from_gy_max: None,
            sacrifice_options: vec![],
            additional_cost_label: Some("exile 1 creature from GY".into()),
            alternative_cost: None,
        };

        let cost_note = cs.additional_cost_label.as_deref().unwrap_or("");
        let label = format!("Cast {} ({})", cs.name, cost_note);

        assert!(
            label.to_lowercase().contains("exile"),
            "Cast label should mention the additional cost. label = {label:?}",
        );
    }

    // Bug H8 (X-cost spell labels don't show X) was removed as part of the
    // X-cost funding refactor. Under the new flow, the LLM picks "Cast
    // Devil's Play" without any preset X; X is chosen afterward via a
    // `ChooseXFunding` structured prompt. The cast-label is intentionally
    // X-free — there's no value to display at cast-selection time.

    /// `ChosenIndex` labels are always provided (not optional) so the
    /// LLM always sees a descriptive label for indexed choices.
    #[test]
    fn chosen_index_label_is_required() {
        use mtg_engine::actions::ResolvedChoice;

        let view = empty_view();
        let label = LlmPlayer::format_single_action(
            &view,
            &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "Creature".into()) },
        );
        assert_eq!(label, "Creature");
    }

    /// Bug J (`audits/AUDIT_BUGS.md)`: Harvest Pyre's X-cost cast
    /// options used to collapse to a single max-X entry in the LLM
    /// player's display. The engine emitted one `CastSpell` per
    /// (X, subset) combination, but `seen_spell_objects` deduped by
    /// `object_id` so only the first (max X) entry was shown. A
    /// graveyard-care deck could never cast Harvest Pyre with X<max.
    ///
    /// Fix: the engine now emits ONE `CastSpell` per target (no
    /// subset enumeration — see `audit_bugs.rs::bug_harvest_pyre_auto_selects_exile`).
    /// The display label exposes a *range* (`X=0..N (0..N damage)`)
    /// so the LLM knows it can pick any X up to N. After picking the
    /// cast, the engine sets up a `ChooseExileFromGraveyard` resolution
    /// prompt and the `choose_exile_from_graveyard` handler returns a
    /// `ChosenExileSet`. This test pins both halves of the contract:
    /// the label format and the dispatch path.
    #[test]
    fn bug_j_harvest_pyre_exposes_x_range_not_just_max() {
        use mtg_engine::actions::{CastTargetSpec, CastableSpell};

        let cs = CastableSpell {
            object_id: ObjectId(202),
            name: "Harvest Pyre".into(),
            is_flashback: false,
            from_graveyard: false,
            target_spec: CastTargetSpec::SingleTarget(vec![]),
            tap_plan: vec![],
            exile_x_from_gy_max: Some(3),
            sacrifice_options: vec![],
            additional_cost_label: Some("exile cards from GY".into()),
            alternative_cost: None,
        };

        // Mirror the LLM display logic in mtg-player/src/llm.rs around
        // the `x_suffix` computation: the label must show a range so
        // the LLM knows X<max is reachable.
        let x_suffix = cs.exile_x_from_gy_max
            .map(|n| format!(" X=0..{n} (0..{n} damage)"))
            .unwrap_or_default();
        let label = format!("Cast {}{}", cs.name, x_suffix);

        assert!(
            label.contains("X=0..3") || label.contains("0..=") || label.contains("any X"),
            "Harvest Pyre's cast label should expose the FULL range of \
             X choices (not just X=max) so a graveyard-care deck can \
             preserve creatures by picking X<max. label = {label:?}",
        );
    }

    // ── Every index prompt carries the board (issue #491) ────────────────

    /// A backend that keeps every prompt it is handed, so a test can read
    /// what the seat would really have been asked.
    struct RecordingBackend {
        prompts: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
        reply: serde_json::Value,
        system_prompt: String,
    }

    impl LlmBackend for RecordingBackend {
        fn send(&mut self, message: &str) -> String {
            self.prompts.borrow_mut().push(message.to_string());
            String::new()
        }
        fn send_with_schema(&mut self, message: &str, _schema: &serde_json::Value) -> serde_json::Value {
            self.prompts.borrow_mut().push(message.to_string());
            self.reply.clone()
        }
        fn init(&mut self, deck_info: &str) {
            self.system_prompt = deck_info.to_string();
        }
        fn system_prompt(&self) -> &str { &self.system_prompt }
        fn model_name(&self) -> &str { "recording" }
    }

    /// A seat that answers every index prompt with 0, and keeps the prompts.
    fn recording_player() -> (LlmPlayer, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
        recording_player_answering(serde_json::json!({"thoughts": "t", "action": 0}))
    }

    /// A seat that answers every prompt with `reply`, whatever it is asked.
    fn recording_player_answering(
        reply: serde_json::Value,
    ) -> (LlmPlayer, std::rc::Rc<std::cell::RefCell<Vec<String>>>) {
        let prompts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut player = LlmPlayer::for_prompt_tests("test");
        player.backend = Box::new(RecordingBackend {
            prompts: std::rc::Rc::clone(&prompts),
            reply,
            system_prompt: String::new(),
        });
        (player, prompts)
    }

    /// What every decision is entitled to: the turn, the step, and both
    /// life totals. `build_prompt` is the only thing that supplies them.
    fn assert_carries_the_board(prompt: &str, what: &str) {
        assert!(prompt.starts_with("Turn "),
            "{what} must open with the turn/step header, got:\n{prompt}");
        assert!(prompt.contains("hp,") && prompt.contains("Opp: "),
            "{what} must carry both life totals, got:\n{prompt}");
    }

    /// #491: four prompt kinds built their message with a bare `format!`
    /// and handed it straight to `pick_action_index`, so the seat chose a
    /// target from as little as 48 characters — no turn, no life totals,
    /// no boards, no stack — while the CLI renders the whole board screen
    /// at the identical question (#122). The state body is built inside
    /// `pick_action_index` now, so a caller cannot ask for an index
    /// without it; these drive all four of the prompts that used to skip
    /// it and read what the backend was really sent.
    #[test]
    fn every_index_prompt_carries_the_board() {
        use mtg_engine::actions::{
            ActivatableAbility, ActivatableAbilityOption, CastTargetSpec, CastableSpell, Target,
        };

        let (state, registry) = view_for_contract_test();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        let opponent = mtg_engine::ids::PlayerId(1);
        let mine: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.owner == view.you)
            .map(|o| o.id)
            .take(2)
            .collect();
        assert_eq!(mine.len(), 2, "the fixture has objects to sacrifice");

        // "<card>: select a target" — the 48-character prompt.
        let (mut player, prompts) = recording_player();
        player.prompt_target_selection(
            &view,
            "Geistflame: select a target",
            &[Target::Player(view.you), Target::Player(opponent)],
        );
        assert_carries_the_board(&prompts.borrow()[0], "the target-selection prompt");
        // And its options are one per line, not one comma-joined row.
        assert!(prompts.borrow()[0].contains("0: You\n1: Opponent"),
            "one target per line:\n{}", prompts.borrow()[0]);

        // "<card>: choose a creature to sacrifice as additional cost".
        let (mut player, prompts) = recording_player();
        player.choose_cast_targets(&view, &CastableSpell {
            object_id: mine[0],
            name: "Altar's Reap".into(),
            is_flashback: false,
            from_graveyard: false,
            target_spec: CastTargetSpec::NoTargets,
            tap_plan: vec![],
            exile_x_from_gy_max: None,
            sacrifice_options: mine.clone(),
            additional_cost_label: Some("sacrifice a creature".into()),
            alternative_cost: None,
        }, &[]);
        assert_carries_the_board(&prompts.borrow()[0], "the cast-sacrifice prompt");

        // "<card>: choose a target for <ability>", then
        // "<card>: choose a creature to sacrifice" — both from one call.
        let (mut player, prompts) = recording_player();
        player.choose_ability_targets(&view, &ActivatableAbility {
            object_id: mine[0],
            ability_index: 0,
            source_card_id: None,
            name: "Demonmail Hauberk (#42)".into(),
            description: "Equip—Sacrifice a creature".into(),
            target_options: vec![],
            tap_plan: vec![],
            sorcery_speed: false,
            option_combos: vec![
                ActivatableAbilityOption { targets: vec![Target::Player(view.you)], sacrifice: Some(mine[0]) },
                ActivatableAbilityOption { targets: vec![Target::Player(view.you)], sacrifice: Some(mine[1]) },
                ActivatableAbilityOption { targets: vec![Target::Player(opponent)], sacrifice: Some(mine[0]) },
            ],
        }, &[]);
        let recorded = prompts.borrow();
        assert_eq!(recorded.len(), 2, "one prompt per dimension:\n{recorded:#?}");
        assert_carries_the_board(&recorded[0], "the ability-target prompt");
        assert_carries_the_board(&recorded[1], "the ability-sacrifice prompt");
    }
    // ── GAME_RULES against the formatters it documents (#492, #493) ──────

    /// #493: the spec section was updated to the one-row-per-line action
    /// list and the five worked examples below it were not, so the one
    /// section a seat parses on every decision was demonstrated five times
    /// in a shape the program had stopped emitting. Checked as a property
    /// of the const rather than five string comparisons, so a sixth
    /// example cannot be added in the old shape either.
    #[test]
    fn game_rules_shows_every_action_list_one_row_per_line() {
        let mut blocks = 0;
        let mut lines = GAME_RULES.lines().peekable();
        while let Some(line) = lines.next() {
            if line.trim_end() != "Available actions:" {
                continue;
            }
            blocks += 1;
            // Rows run until the fence or a blank line ends the block.
            while let Some(row) = lines.peek() {
                if row.trim().is_empty() || row.starts_with("```") {
                    break;
                }
                let row = lines.next().expect("peeked");
                let (index, rest) = row.split_once(": ").unwrap_or_else(||
                    panic!("every quoted action row is `N: label`, got {row:?}"));
                assert!(
                    index.split('-').all(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
                    "an action row is numbered `N:` or `N-M:`, got {row:?}"
                );
                // A second `N: ` on the same row is the comma-joined list
                // `format_action_prompt` has not emitted since #201.
                for (i, _) in rest.match_indices(": ") {
                    let before = rest[..i].rsplit(' ').next().unwrap_or("");
                    assert!(
                        before.is_empty() || !before.chars().all(|c| c.is_ascii_digit()),
                        "GAME_RULES quotes a comma-joined action list; the harness \
                         sends one row per line: {row:?}"
                    );
                }
            }
        }
        // The worked examples went when the system prompt was cut to what
        // decides a game (it is re-read, cached, on every decision); the
        // spec block stays, and any example added back is checked too.
        // The rules no longer quote a whole list (they are re-read, cached,
        // on every decision); any list added back is checked above.
        let _ = blocks;
    }

    /// #492: GAME_RULES told every seat, on every call, that the
    /// exile-from-graveyard prompt answers "with a boolean per card". It
    /// answers with an index array, and has since a card name stopped being
    /// a legal top-level schema key (#398) — so the standing instruction
    /// described a response the API would refuse, for the prompt whose
    /// unusable answers cancel a cast. The documented example is built
    /// through the formatter that sends it.
    #[test]
    fn game_rules_shows_the_marked_subset_prompt_it_actually_sends() {
        let labels = vec!["Reckless Waif (#44)".to_string()];
        let body = LlmPlayer::marked_list_body(
            &labels,
            "Harvest Pyre: choose 0-1 cards to exile from your graveyard \
             (each exiled card adds to the spell's X)",
            &LlmPlayer::marked_count_note(0, 1, "card"),
            "Name the cards to exile.",
        );
        // The prompt carries its own instructions; the rules no longer quote
        // it, and must not quote an older shape of it either.
        assert!(body.contains("Name the cards to exile."), "{body}");
        assert!(!GAME_RULES.contains("Harvest Pyre: choose"), "no stale example");

        // And it must not promise the shape that prompt stopped using. The
        // pile division is the one place a boolean per permanent is still
        // what the schema asks for.
        for (n, line) in GAME_RULES.lines().enumerate() {
            if line.contains("boolean") {
                assert!(
                    line.contains("pile"),
                    "GAME_RULES line {n} promises booleans for a prompt that \
                     answers with indices: {line:?}"
                );
            }
        }
    }
}

/// Issue #719: the metered seats retry within the game seat's budget, say
/// when a call produced no answer, and give up — so their runner forfeits
/// them — when no answer is coming.
#[cfg(test)]
mod metered_seat_failures {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    /// A server on a loopback port answering every request with `status`.
    fn stub_server(status: u16) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 { break; }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" { break; }
                }
                let mut body = vec![0; len];
                let _ = reader.read_exact(&mut body);
                let reply = "{\"error\":\"stub\"}";
                let _ = write!(stream,
                    "HTTP/1.1 {status} Stub\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len());
            }
        });
        format!("http://{addr}")
    }

    fn anthropic(base_url: String) -> AnthropicBackend {
        AnthropicBackend {
            client: Client::new(),
            api_key: "dummy".into(),
            model: "stub".into(),
            system_prompt: String::new(),
            conversation: Vec::new(),
            last_thinking: None,
            last_call_failure: None,
            gave_up: None,
            base_url,
            seat: String::new(),
            thinking: ThinkingLevel::Low,
            window: 0,
        }
    }

    /// A Messages API on a loopback port that keeps the last request body
    /// and answers every request with `answer` as the model's text.
    fn answering_server(answer: &'static str) -> (String, std::sync::Arc<std::sync::Mutex<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let kept = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 { break; }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" { break; }
                }
                let mut body = vec![0; len];
                let _ = reader.read_exact(&mut body);
                *kept.lock().unwrap() = String::from_utf8_lossy(&body).into_owned();
                let reply = serde_json::json!({
                    "content": [{"type": "text", "text": answer}],
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                }).to_string();
                let _ = write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len());
            }
        });
        (format!("http://{addr}"), seen)
    }

    /// The Messages API seat asks for `thoughts` in the JSON, as every
    /// other backend does, and the thoughts it is sent back become its
    /// notes and leave the answer. It used to strip the field and take its
    /// notes from the thinking channel, which at `off` does not exist and
    /// on the newest models comes back empty — a seat with no notes at all
    /// (reports/llm-cost.md, phase 3).
    #[test]
    fn the_api_seat_reasons_in_the_json_and_keeps_its_thoughts_as_notes() {
        let (url, seen) = answering_server(r#"{"thoughts":"Hold the Bolt for the flier.","action":2}"#);
        let mut backend = anthropic(url);
        let schema = LlmPlayer::enum_action_schema(3, "action", "Index of the chosen action");
        let got = backend.send_with_schema("prompt", &schema);
        assert_eq!(got["action"], 2, "{got}");
        assert!(got.get("thoughts").is_none(), "the thoughts are taken out of the answer: {got}");
        assert_eq!(backend.take_thinking().as_deref(), Some("Hold the Bolt for the flier."));
        let request: serde_json::Value = serde_json::from_str(&seen.lock().unwrap()).unwrap();
        let sent = &request["output_config"]["format"]["schema"];
        assert!(sent["properties"].get("thoughts").is_some(), "the schema asks for thoughts: {sent}");
        assert!(sent["required"].as_array().unwrap().iter().any(|v| v == "thoughts"), "{sent}");
    }

    /// A changed `output_config.format` invalidates the prompt cache, so a
    /// menu of three and a menu of seven send the same format: the indices
    /// they allow are checked on the answer, not sent.
    #[test]
    fn the_api_seat_sends_one_format_for_menus_of_any_length() {
        let format_for = |n: usize| {
            let (url, seen) = answering_server(r#"{"thoughts":"t","action":0}"#);
            let mut backend = anthropic(url);
            backend.send_with_schema("prompt", &LlmPlayer::enum_action_schema(n, "action", "Index of the chosen action"));
            let request: serde_json::Value = serde_json::from_str(&seen.lock().unwrap()).unwrap();
            request["output_config"]["format"].to_string()
        };
        let three = format_for(3);
        assert_eq!(three, format_for(7));
        assert!(!three.contains("enum"), "{three}");
    }

    /// The other side of sending no enums: an index the menu does not have
    /// is asked once more, with the reason, rather than read as the
    /// fallback's pass.
    #[test]
    fn an_answer_the_schema_no_longer_prevents_is_asked_again_with_the_reason() {
        struct Replies {
            replies: std::collections::VecDeque<serde_json::Value>,
            prompts: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
        }
        impl LlmBackend for Replies {
            fn send(&mut self, message: &str) -> String {
                self.send_with_schema(message, &serde_json::Value::Null).to_string()
            }
            fn send_with_schema(&mut self, m: &str, _s: &serde_json::Value) -> serde_json::Value {
                self.prompts.borrow_mut().push(m.to_string());
                self.replies.pop_front().unwrap_or_else(|| serde_json::json!({}))
            }
            fn init(&mut self, _deck_info: &str) {}
            fn system_prompt(&self) -> &str { "" }
            fn model_name(&self) -> &str { "model-reask" }
        }
        let menu = "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n1: Play Mountain\n2: Concede\n";
        for (first, expected_calls, chosen) in [(7, 2, 1), (1, 1, 1)] {
            let prompts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
            let mut p = LlmPlayer::for_prompt_tests("Seat-reask");
            p.backend = Box::new(Replies {
                replies: [serde_json::json!({"action": first}), serde_json::json!({"action": 1})].into(),
                prompts: prompts.clone(),
            });
            assert_eq!(p.pick_action_index(&tests::empty_view(), menu, 3), chosen);
            let prompts = prompts.borrow();
            assert_eq!(prompts.len(), expected_calls, "first answer {first}");
            if expected_calls == 2 {
                assert!(prompts[1].starts_with(&prompts[0]), "the same question, asked again");
                assert!(prompts[1].contains("is not one this question allows: `action` is 7; it must be 0 to 2."),
                    "the reason is given: {}", prompts[1]);
            }
        }
    }

    fn gemini(base_url: String) -> GeminiBackend {
        GeminiBackend {
            client: Client::new(),
            api_key: "dummy".into(),
            model: "stub".into(),
            thinking_level: None,
            system_prompt: String::new(),
            interaction_id: None,
            last_thinking: None,
            last_call_failure: None,
            gave_up: None,
            base_url,
            seat: String::new(),
            window: 0,
            chain_len: 0,
        }
    }

    #[test]
    fn a_refused_key_gives_the_seat_up() {
        let url = stub_server(401);
        let mut a = anthropic(url.clone());
        a.send("hello");
        assert!(a.take_call_failure().is_some(), "a 401 is no answer, not the model's 0");
        assert!(a.gave_up().is_some(), "a dead key is a seat that has stopped answering");

        let mut g = gemini(url);
        g.call_interactions_structured("hello", &serde_json::json!({"type": "object"}));
        assert!(g.take_call_failure().is_some());
        assert!(g.gave_up().is_some());
    }

    #[test]
    fn a_refused_request_is_no_answer_but_not_the_end() {
        let url = stub_server(400);
        let mut a = anthropic(url.clone());
        a.send("hello");
        let why = a.take_call_failure().expect("a 400 is no answer");
        assert!(why.contains("400"), "{why}");
        assert!(a.gave_up().is_none(), "the next request may be answered");

        let mut g = gemini(url);
        g.call_interactions_structured("hello", &serde_json::json!({"type": "object"}));
        assert!(g.take_call_failure().is_some());
        assert!(g.gave_up().is_none());
    }

    #[test]
    fn the_budget_decides_how_long_transient_failures_are_retried() {
        let mut tries = 0;
        let out: CallOutcome<()> = call_within_budget(std::time::Duration::ZERO,
            |_| { tries += 1; CallAttempt::Transient("503".into()) }, |_, _, _| {});
        assert!(matches!(out, CallOutcome::GaveUp(_)), "{out:?}");
        assert_eq!(tries, 1, "a spent budget is not slept past");

        let mut tries = 0;
        let out = call_within_budget(std::time::Duration::from_secs(60), |_| {
            tries += 1;
            if tries < 2 { CallAttempt::Transient("529".into()) } else { CallAttempt::Answer(7) }
        }, |_, _, _| {});
        assert!(matches!(out, CallOutcome::Answer(7)), "a transient failure is retried: {out:?}");

        assert!(matches!(classify_http_status::<()>(500, String::new()), CallAttempt::Transient(_)));
        assert!(matches!(classify_http_status::<()>(429, String::new()), CallAttempt::Transient(_)));
        assert!(matches!(classify_http_status::<()>(400, String::new()), CallAttempt::Refused(_)));
        assert!(matches!(classify_http_status::<()>(403, String::new()), CallAttempt::Dead(_)));
    }
}

/// A decision is sent on its own: the system prompt, at most a bounded
/// window of earlier exchanges, and the prompt — which restates the whole
/// position anyway. The seat's plan and a resume's recap travel inside the
/// prompt instead of the history.
#[cfg(test)]
mod stateless_decisions {
    use super::*;
    use mtg_engine::cards::CardRegistry;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::{Arc, Mutex};

    /// A Messages API on a loopback port that answers every request with a
    /// thinking block and `{"action":0}`, and keeps every request body.
    fn recording_api() -> (String, Arc<Mutex<Vec<serde_json::Value>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&bodies);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut len = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 { break; }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if line == "\r\n" { break; }
                }
                let mut body = vec![0; len];
                let _ = reader.read_exact(&mut body);
                let n = {
                    let mut seen = seen.lock().unwrap();
                    seen.push(serde_json::from_slice(&body).unwrap_or_default());
                    seen.len()
                };
                let reply = serde_json::json!({
                    "content": [
                        {"type": "thinking", "thinking": format!("Decision {n}: the board is stable, so I pass and keep Bolt for the flier.")},
                        {"type": "text", "text": "{\"action\":0}"}
                    ],
                    "usage": {"input_tokens": 1, "output_tokens": 1}
                }).to_string();
                let _ = write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len());
            }
        });
        (format!("http://{addr}"), bodies)
    }

    fn metered_player(url: &str, window: usize) -> LlmPlayer {
        let mut backend = AnthropicBackend {
            client: Client::new(),
            api_key: "dummy".into(),
            model: "stub".into(),
            system_prompt: "rules".into(),
            conversation: Vec::new(),
            last_thinking: None,
            last_call_failure: None,
            gave_up: None,
            base_url: url.to_string(),
            seat: String::new(),
            thinking: ThinkingLevel::Low,
            window: 0,
        };
        backend.set_history_window(window);
        LlmPlayer {
            name: "t".into(),
            last_log_index: 0,
            own_card_names: std::collections::HashSet::new(),
            card_texts: HashMap::new(),
            backend: Box::new(backend),
            provider: Provider::Anthropic,
            guide: None,
            session_logged: None,
            last_call_failure: None,
            pending_recap: None,
            notes: None,
            memory_mirror: None,
            pass_until: None,
            pass_until_stopped: None,
            plan: None,
            plan_stopped: None,
            attack_plan: None,
            plan_done_pending: false,
            offer_pass_until: pass_until_enabled(),
        }
    }

    fn a_view() -> (mtg_engine::state::GameState, CardRegistry) {
        use mtg_engine::engine::{setup_game, Decklist, GameConfig};
        let registry = CardRegistry::with_all_cards();
        let deck = Decklist {
            entries: vec![("Forest".to_string(), 20), ("Grizzly Bears".to_string(), 20)],
        };
        let config = GameConfig {
            player_names: vec!["you".into(), "opp".into()],
            decklists: vec![deck.clone(), deck],
            starting_life: 20,
            starting_player: Some(mtg_engine::ids::PlayerId(0)),
            rng_seed: Some(7),
        };
        let state = setup_game(&config, &registry);
        (state, registry)
    }

    /// The request for decision N carries the system prompt, the window
    /// and the prompt — not decisions 1..N-1. It used to carry all of them,
    /// so the input tokens of a game grew with the square of its decisions.
    #[test]
    fn a_request_carries_at_most_the_window_of_earlier_exchanges() {
        for (window, most_messages) in [(0usize, 1usize), (2, 5)] {
            let (url, bodies) = recording_api();
            let mut player = metered_player(&url, window);
            let (state, registry) = a_view();
            let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
            for _ in 0..6 {
                player.pick_action_index(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n", 1);
            }
            let bodies = bodies.lock().unwrap();
            assert_eq!(bodies.len(), 6);
            for (i, body) in bodies.iter().enumerate() {
                let messages = body["messages"].as_array().expect("a messages array");
                assert!(messages.len() <= most_messages,
                    "window {window}: request {} carries {} messages, more than the window allows",
                    i + 1, messages.len());
                assert_eq!(messages.last().unwrap()["role"], "user", "the prompt is last");
                assert_eq!(messages.len().min(2 * i + 1), messages.len(),
                    "window {window}: request {} cannot carry exchanges that never happened", i + 1);
            }
            // And the last request is no bigger than the first by more than
            // a window of exchanges: the history stopped growing.
            let size = |b: &serde_json::Value| b.to_string().len();
            let first = size(&bodies[0]);
            let last = size(&bodies[5]);
            let one_exchange = size(&bodies[1]) - first;
            assert!(last <= first + window * one_exchange + 64,
                "window {window}: request 6 is {last} bytes against {first} for request 1");
            assert!(player.conversation_len_for_test() <= 2 * window);
        }
    }

    /// The seat's reasoning from one decision comes back to it as notes at
    /// the top of the next prompt — the one thing the bounded conversation
    /// would otherwise lose — and is logged as THOUGHT as before.
    #[test]
    fn the_last_reasoning_comes_back_as_notes_in_the_next_prompt() {
        let (url, bodies) = recording_api();
        let mut player = metered_player(&url, 0);
        let (state, registry) = a_view();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);

        player.pick_action_index(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n", 1);
        assert_eq!(player.notes_for_test(),
            Some("Decision 1: the board is stable, so I pass and keep Bolt for the flier."));
        player.pick_action_index(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n", 1);

        let bodies = bodies.lock().unwrap();
        let prompt = |i: usize| bodies[i]["messages"][0]["content"].as_str().unwrap().to_string();
        assert!(!prompt(0).contains("Your notes from your last decision:"),
            "the first decision has no notes yet:\n{}", prompt(0));
        let second = prompt(1);
        let header = second.find("Turn ").expect("the header line");
        let notes = second.find("Your notes from your last decision:\nDecision 1: the board is stable")
            .expect("the second decision carries the first's reasoning");
        assert!(header < notes, "after the header:\n{second}");
        let board = second.find("You: ").expect("player status");
        assert!(notes < board, "before the board:\n{second}");
        // Documented where the seat reads the prompt format.
        let doc_header = GAME_RULES.find("`Turn N").unwrap();
        let doc_notes = GAME_RULES.find("`Your notes from your last decision:`").expect("documented");
        let doc_events = GAME_RULES.find("`Recent events:`").unwrap();
        assert!(doc_header < doc_notes && doc_notes < doc_events, "in the order they are sent");
    }

    /// The notes are the tail of the reasoning, bounded.
    #[test]
    fn notes_are_the_bounded_tail_of_the_reasoning() {
        assert_eq!(LlmPlayer::notes_from("  "), None, "an empty thinking block is not a note");
        assert_eq!(LlmPlayer::notes_from("short plan").as_deref(), Some("short plan"));
        let long = format!("{}so I attack with everything.", "reading the board, ".repeat(100));
        let notes = LlmPlayer::notes_from(&long).unwrap();
        assert!(notes.starts_with('…'), "marked as cut: {notes}");
        assert!(notes.ends_with("so I attack with everything."), "the end is kept: {notes}");
        assert!(notes.chars().count() <= LlmPlayer::NOTES_MAX_CHARS + 1);
        assert!(!notes.starts_with("…oard"), "cut at a word boundary: {notes}");
    }

    /// A resumed seat reads the recap in its first prompt after the
    /// resume, once, where it used to be a permanent conversation entry
    /// re-sent with every decision of the rest of the game.
    #[test]
    fn a_resumed_seat_reads_the_recap_once_in_its_next_prompt() {
        let mut player = LlmPlayer::for_prompt_tests("t");
        let (state, registry) = a_view();
        let view = GameView::for_player(&state, mtg_engine::ids::PlayerId(0), &registry);
        player.resume_from_log(
            &["── Turn 1 (p0) ──".to_string(), "p0 played Forest".to_string()],
            mtg_engine::ids::PlayerId(0),
        );
        let first = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        let header = first.find("Turn ").expect("header");
        let recap = first.find("Game resumed. Here is the complete game log so far:")
            .expect("the first prompt after a resume carries the recap");
        assert!(header < recap, "{first}");
        assert!(first.contains("You played Forest"), "in the seat's vocabulary:\n{first}");
        let second = player.build_prompt(&view, "[MAIN PHASE 1]\nAvailable actions:\n0: Pass\n");
        assert!(!second.contains("Game resumed"), "and the second does not:\n{second}");
        assert_eq!(player.conversation_len_for_test(), 0);
    }
}

#[cfg(test)]
mod env_seconds_cap {
    use super::*;

    /// A seconds override from the environment is capped at a day, so the
    /// deadline it makes cannot overflow an `Instant` (#812).
    #[test]
    fn an_override_too_long_for_the_clock_is_a_day() {
        // A name no other test or run sets, so setting it races nothing.
        const ENV: &str = "MTG_TEST_ENV_SECONDS_CAP_812";
        std::env::set_var(ENV, "18446744073709551615");
        let d = env_seconds(ENV, std::time::Duration::from_secs(5));
        assert_eq!(d, std::time::Duration::from_secs(MAX_ENV_SECONDS));
        assert!(std::time::Instant::now().checked_add(d).is_some());
        std::env::set_var(ENV, "7");
        assert_eq!(env_seconds(ENV, std::time::Duration::from_secs(5)), std::time::Duration::from_secs(7));
        std::env::remove_var(ENV);
        assert_eq!(env_seconds(ENV, std::time::Duration::from_secs(5)), std::time::Duration::from_secs(5));
    }
}
