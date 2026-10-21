//! Answers in text: a `claude -p` seat asked for its JSON in the prompt
//! rather than through `--json-schema`.
//!
//! The CLI's `--json-schema` is a tool the model calls and a second request
//! after it: every decision paid ~575 tokens of cache write and ~100 of
//! output for the round trip whatever was asked, and each kind of question
//! was a new tool ahead of the system prompt, so a write of the whole
//! cached prefix the first time a game asked it (`reports/llm-cost.md`,
//! phase 5). Asked in the prompt, the answer is one request: the fields
//! are said at the end of the decision's own prompt, which is never cached
//! for a second call anyway, and the long descriptions every game repeats
//! are said once, in the system prompt ([`answer_fields_section`]).
//!
//! The answer is held to the question's full schema afterwards
//! (`schema_violation`), as a `--json-schema` answer is, and an answer that
//! is not a JSON object is asked again like any other that is not one the
//! question allows.

use serde_json::Value;

use super::{ASK_AGAIN_DESCRIPTION, ATTACK_DESCRIPTION, PLAN_DESCRIPTION, THOUGHTS_DESCRIPTION, UNREAD};

/// `MTG_CLAUDE_CODE_ANSWERS`: `text` (the default) asks a `claude -p` seat
/// for its JSON in the prompt; `schema` sends `--json-schema` as before.
pub const ANSWERS_ENV: &str = "MTG_CLAUDE_CODE_ANSWERS";

/// Whether a `claude -p` seat is asked for its answer in the prompt.
#[must_use]
pub fn text_answers() -> bool {
    static WARNED: std::sync::Once = std::sync::Once::new();
    match std::env::var(ANSWERS_ENV).as_deref() {
        Err(_) | Ok("" | "text") => true,
        Ok("schema" | "tool") => false,
        Ok(other) => {
            WARNED.call_once(|| crate::stderr_line!(
                "Warning: {ANSWERS_ENV}={other:?} is not text or schema; answers stay in text."));
            true
        }
    }
}

/// The fields whose meaning is the same in every question that asks them,
/// said once in the system prompt instead of in every prompt.
pub const SHARED_FIELDS: [(&str, &str); 4] = [
    ("thoughts", THOUGHTS_DESCRIPTION),
    ("then", PLAN_DESCRIPTION),
    ("ask_again", ASK_AGAIN_DESCRIPTION),
    ("attack", ATTACK_DESCRIPTION),
];

/// The system prompt's "Answer format" section for a seat that answers in
/// text: how to answer, and what the shared fields mean.
#[must_use]
pub fn answer_fields_section() -> String {
    let mut s = String::from(
        "\n\n## Answer format\n\nAnswer every question with only a JSON object: no words before or after it and \
no code fence. Each question ends with the fields its object has. These mean the same wherever \
a question asks for them:\n");
    for (name, description) in SHARED_FIELDS {
        s.push_str(&format!("- `{name}`: {description}\n"));
    }
    s
}

/// The end of a prompt for a seat that answers in text: the object's
/// fields, in the order to write them — `thoughts` first, then the rest as
/// the schema lists them — with each one's type and, for a field not in
/// [`SHARED_FIELDS`], its description. A field this question does not read
/// ([`UNREAD`]), or a list nothing may go in, is left out: the answer is not
/// asked to fill it.
#[must_use]
pub fn answer_format(schema: &Value) -> String {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return "Answer with only a JSON object.".to_string();
    };
    let required: Vec<&str> = schema.get("required").and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    // A field this question does not read, or one nothing may go in (no
    // planeswalker to attack: `maxItems` 0), is not offered: offered, it
    // was filled, and the answer asked again (`planeswalker_attacks` has 1
    // entries; it takes at most 0).
    let mut names: Vec<&String> = properties.iter()
        .filter(|(_, spec)| spec.get(UNREAD).is_none() && spec.get("maxItems").and_then(Value::as_u64) != Some(0))
        .map(|(name, _)| name).collect();
    // `thoughts` first, as the reason before the answer; the rest in the
    // order the question requires them, then any it does not.
    names.sort_by_key(|n| (n.as_str() != "thoughts", required.iter().position(|r| r == n).unwrap_or(usize::MAX)));
    let fields: Vec<String> = names.iter().map(|name| {
        let optional = if required.contains(&name.as_str()) { "" } else { " (optional)" };
        format!("\"{name}\": {}{optional}", type_of(&properties[name.as_str()]))
    }).collect();
    let mut s = format!("Answer with only this JSON object: {{{}}}", fields.join(", "));
    for name in names {
        let shared = SHARED_FIELDS.iter().any(|(n, d)| n == name
            && properties[name.as_str()].get("description").and_then(Value::as_str) == Some(*d));
        if shared {
            continue;
        }
        if let Some(d) = properties[name.as_str()].get("description").and_then(Value::as_str) {
            s.push_str(&format!("\n`{name}`: {d}"));
        }
    }
    s
}

/// A field's type as the format line says it: `integer`, `0 to 7`,
/// `[integer]`, `{"blocker": integer, "attacker": integer}`.
fn type_of(spec: &Value) -> String {
    if let Some(values) = spec.get("enum").and_then(Value::as_array) {
        let ints: Option<Vec<i64>> = values.iter().map(Value::as_i64).collect();
        if let Some(ints) = ints.filter(|v| !v.is_empty()) {
            let (lo, hi) = (ints.iter().min().copied().unwrap_or(0), ints.iter().max().copied().unwrap_or(0));
            if usize::try_from(hi - lo + 1).ok() == Some(ints.len()) {
                return if lo == hi { lo.to_string() } else { format!("integer {lo} to {hi}") };
            }
            if ints.len() <= 12 {
                return format!("one of {}", ints.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "));
            }
        }
    }
    match spec.get("type").and_then(Value::as_str) {
        Some("array") => {
            let item = spec.get("items").map_or_else(|| "value".to_string(), type_of);
            format!("[{item}]")
        }
        Some("object") => match spec.get("properties").and_then(Value::as_object) {
            Some(p) if !p.is_empty() => format!("{{{}}}", p.iter()
                .map(|(k, v)| format!("\"{k}\": {}", type_of(v))).collect::<Vec<_>>().join(", ")),
            _ => "{}".to_string(),
        },
        Some("boolean") => "true or false".to_string(),
        Some(other) => other.to_string(),
        None => "value".to_string(),
    }
}

/// The JSON object in a text answer: the whole text, or the first `{` to
/// the last `}` when the model wrapped it in a code fence or a sentence.
#[must_use]
pub fn json_in_text(text: &str) -> Option<Value> {
    let text = text.trim();
    if let Ok(v @ Value::Object(_)) = serde_json::from_str(text) {
        return Some(v);
    }
    let (start, end) = (text.find('{')?, text.rfind('}')?);
    match serde_json::from_str(text.get(start..=end)?) {
        Ok(v @ Value::Object(_)) => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_line_names_the_fields_in_order_and_leaves_out_what_is_not_read() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "action": {"type": "integer", "enum": [0, 1, 2, 3], "description": "Index of the chosen action"},
                "ask_again": {"type": "boolean", "description": ASK_AGAIN_DESCRIPTION, UNREAD: true},
                "attack": {"type": "array", "items": {"type": "integer"}, "description": ATTACK_DESCRIPTION},
                "thoughts": {"type": "string", "description": THOUGHTS_DESCRIPTION},
            },
            "required": ["thoughts", "action", "ask_again", "attack"],
        });
        let f = answer_format(&schema);
        assert!(f.starts_with(r#"Answer with only this JSON object: {"thoughts": string, "action": integer 0 to 3, "attack": [integer]}"#), "{f}");
        assert!(f.contains("\n`action`: Index of the chosen action"), "{f}");
        assert!(!f.contains("ask_again") && !f.contains(ATTACK_DESCRIPTION), "shared and unread fields are not spelled out: {f}");
        let section = answer_fields_section();
        assert!(section.contains(ATTACK_DESCRIPTION) && section.contains("`thoughts`"), "{section}");
    }

    #[test]
    fn a_list_nothing_may_go_in_is_not_offered() {
        let schema = serde_json::json!({"type": "object", "properties": {
            "attacker_indices": {"type": "array", "items": {"type": "integer"}},
            "planeswalker_attacks": {"type": "array", "items": {"type": "object"}, "maxItems": 0,
                                     "description": "No planeswalker can be attacked: leave empty"}},
            "required": ["attacker_indices"]});
        let f = answer_format(&schema);
        assert!(!f.contains("planeswalker"), "{f}");
    }

    #[test]
    fn nested_shapes_are_said_as_json() {
        let schema = serde_json::json!({"type": "object", "properties": {
            "blocks": {"type": "array", "items": {"type": "object", "properties": {
                "blocker": {"type": "integer"}, "attacker": {"type": "integer"}}}},
            "mull": {"type": "boolean"}}, "required": ["blocks"]});
        let f = answer_format(&schema);
        assert!(f.contains(r#""blocks": [{"attacker": integer, "blocker": integer}]"#), "{f}");
        assert!(f.contains(r#""mull": true or false (optional)"#), "{f}");
    }

    #[test]
    fn the_object_is_found_in_a_fence_or_a_sentence() {
        assert_eq!(json_in_text(r#" {"action": 1} "#), Some(serde_json::json!({"action": 1})));
        assert_eq!(json_in_text("```json\n{\"action\": 2}\n```"), Some(serde_json::json!({"action": 2})));
        assert_eq!(json_in_text("I pass. {\"action\": 0} Done."), Some(serde_json::json!({"action": 0})));
        assert_eq!(json_in_text("no json here"), None);
        assert_eq!(json_in_text("[1, 2]"), None);
    }
}
