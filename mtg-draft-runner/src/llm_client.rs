use std::env;

use reqwest::blocking::Client;

use mtg_draft::draft::DraftPick;

/// Per-model token usage tracking. Thread-safe via Mutex.
use std::sync::Mutex;
use std::collections::HashMap;
use std::fmt::Write;
use mtg_player::llm::Cost;

#[derive(Default, Debug)]
pub struct ModelUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
    pub calls: u64,
}

static MODEL_USAGE: std::sync::LazyLock<Mutex<HashMap<String, ModelUsage>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn record_model_usage(model: &str, input: u64, output: u64, cache_read: u64, cache_create: u64) {
    let mut map = MODEL_USAGE.lock().unwrap();
    let entry = map.entry(model.to_string()).or_default();
    entry.calls += 1;
    entry.input += input;
    entry.output += output;
    entry.cache_read += cache_read;
    entry.cache_create += cache_create;
}

/// Record token usage from an Anthropic API response.
pub fn record_anthropic_usage(model: &str, usage: &serde_json::Value) {
    record_model_usage(
        model,
        usage["input_tokens"].as_u64().unwrap_or(0),
        usage["output_tokens"].as_u64().unwrap_or(0),
        usage["cache_read_input_tokens"].as_u64().unwrap_or(0),
        usage["cache_creation_input_tokens"].as_u64().unwrap_or(0),
    );
}

/// Record token usage from a Gemini Interactions API response.
pub fn record_gemini_usage(model: &str, usage: &serde_json::Value) {
    let input_tokens = usage["total_input_tokens"].as_u64().unwrap_or(0);
    let cached_tokens = usage["total_cached_tokens"].as_u64().unwrap_or(0);
    let uncached_input = input_tokens.saturating_sub(cached_tokens);
    record_model_usage(
        model,
        uncached_input,
        usage["total_output_tokens"].as_u64().unwrap_or(0),
        cached_tokens,
        0, // Gemini interactions API manages caching server-side
    );
}

/// Get the per-model usage map for external reporting.
pub fn get_model_usage() -> HashMap<String, ModelUsage> {
    MODEL_USAGE.lock().unwrap().clone()
}

impl Clone for ModelUsage {
    fn clone(&self) -> Self {
        Self { input: self.input, output: self.output, cache_read: self.cache_read, cache_create: self.cache_create, calls: self.calls }
    }
}

/// Known model pricing ($/`MTok`). (input, output, `cache_read`, `cache_write`)
/// Anthropic: platform.claude.com/docs/en/about-claude/pricing (verified 2026-04-08)

/// This crate's usage record as the shared cost model reads it. The two
/// structs are the same five counters kept once per crate.
fn as_llm_usage(u: &ModelUsage) -> mtg_player::llm::LlmModelUsage {
    mtg_player::llm::LlmModelUsage {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_create: u.cache_create,
        calls: u.calls,
    }
}

fn usage_cost(u: &ModelUsage, model: &str) -> Cost {
    mtg_player::llm::cost(model, &as_llm_usage(u))
}

/// What a phase's usage came to, summed the way the shared model sums a run:
/// plan-quota seats contribute no dollars, and one model with no rate on file
/// makes the phase total unknown rather than an understatement.
fn phase_cost(usage: &HashMap<String, ModelUsage>) -> Cost {
    let converted: HashMap<String, mtg_player::llm::LlmModelUsage> =
        usage.iter().map(|(m, u)| (m.clone(), as_llm_usage(u))).collect();
    mtg_player::llm::total_cost(&converted)
}

/// Print a summary of all token usage and estimated cost, broken down by model and phase.
pub fn print_usage_summary(total_games: usize) {
    let draft_usage = get_model_usage();
    let game_usage = mtg_player::llm::get_llm_model_usage();

    // Draft phase cost
    let draft_cost = phase_cost(&draft_usage);
    let draft_calls: u64 = draft_usage.values().map(|u| u.calls).sum();

    // Game phase cost
    let game_usage: HashMap<String, ModelUsage> = game_usage.iter().map(|(m, u)| (m.clone(), ModelUsage {
        calls: u.calls, input: u.input, output: u.output, cache_read: u.cache_read, cache_create: u.cache_create,
    })).collect();
    let game_cost = phase_cost(&game_usage);
    let game_calls: u64 = game_usage.values().map(|u| u.calls).sum();

    // Combined per-model
    let mut combined: HashMap<String, ModelUsage> = HashMap::new();
    for (model, u) in &draft_usage {
        let entry = combined.entry(model.clone()).or_default();
        entry.calls += u.calls;
        entry.input += u.input;
        entry.output += u.output;
        entry.cache_read += u.cache_read;
        entry.cache_create += u.cache_create;
    }
    for (model, u) in &game_usage {
        let entry = combined.entry(model.clone()).or_default();
        entry.calls += u.calls;
        entry.input += u.input;
        entry.output += u.output;
        entry.cache_read += u.cache_read;
        entry.cache_create += u.cache_create;
    }

    // Build summary string for both stderr and log file
    let mut summary = String::from("=== Token Usage ===\n");

    writeln!(summary, "  Draft:  {draft_calls} calls, {draft_cost}").unwrap();
    // A per-game average only means anything for a metered phase; a
    // plan-quota or unpriced one has no dollars to divide.
    let per_game = match (game_cost, total_games) {
        (Cost::Usd(v), n) if n > 0 => format!(" ({n} games, ${:.4}/game avg)",
            v / f64::from(u32::try_from(n).unwrap_or(u32::MAX))),
        _ => String::new(),
    };
    writeln!(summary, "  Games:  {game_calls} calls, {game_cost}{per_game}").unwrap();
    summary.push('\n');

    let mut models: Vec<_> = combined.iter().collect();
    models.sort_by_key(|(name, _)| (*name).clone());
    let total_cost = phase_cost(&combined);

    for (model, u) in &models {
        let cost = usage_cost(u, model);
        writeln!(summary, "  {}: {} calls, {}in/{}out/{}cached = ${:.4}",
            model, u.calls, u.input, u.output, u.cache_read, cost
        ).unwrap();
    }
    writeln!(summary, "  ---\n  Total: {} calls, {total_cost}", draft_calls + game_calls).unwrap();

    // Write to stderr
    eprint!("\n{summary}");

    // Write to log file
    mtg_player::game_log::write(file!(), line!(), "TOKEN USAGE", &summary);
}

/// LLM client for draft picks and deck building.
/// Maintains a multi-turn conversation for the draft process.
/// Backend trait for draft LLM communication. Both `send_pick` and
/// `send_deck_building` return the raw JSON response (pretty-printed)
/// from the model, with the schema enforced via structured output on
/// both Gemini and Anthropic.
trait DraftBackend: Send {
    /// Send a draft pick message. `num_cards` is the number of cards in
    /// the current pack, used to build an enum-constrained `pick` index
    /// schema. Returns the raw JSON response text.
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String;
    /// Send a deck building message. `pool` is the full set of card
    /// names the model can pick from, used to build an enum-constrained
    /// `maindeck` schema. Returns the raw JSON response text.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String;
    /// The full system prompt this backend will send to the model.
    fn system_prompt(&self) -> &str;
}

/// Build a pick schema with the `pick` field constrained to the valid
/// `0..num_cards` range via an enum. This means the model cannot emit an
/// out-of-range index under constrained decoding.
fn pick_schema_for(num_cards: usize) -> serde_json::Value {
    let valid_indices: Vec<serde_json::Value> = (0..num_cards)
        .map(|i| serde_json::json!(i))
        .collect();
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {
                "type": "string",
                "description": "Concise but complete summary of your reasoning for this pick"
            },
            "pick": {
                "type": "integer",
                "enum": valid_indices,
                "description": "0-indexed number of the card to pick"
            }
        },
        "required": ["thoughts", "pick"]
    })
}

/// Build a deck-building schema with one property per card in the pool.
/// Each card's value is an integer enum `[0..copies_drafted]`, so
/// constrained decoding prevents the model from including more copies
/// than it drafted. The `lands` object has fixed keys for the five
/// basic lands with non-negative integer values.
fn deck_schema_for(pool: &[String]) -> serde_json::Value {
    // Count copies of each card in the pool (DFC: use front face name)
    let mut pool_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for card in pool {
        let name = card.split(" // ").next().unwrap_or(card).to_string();
        *pool_counts.entry(name).or_insert(0) += 1;
    }

    let mut maindeck_props = serde_json::Map::new();
    let mut sorted_cards: Vec<_> = pool_counts.into_iter().collect();
    sorted_cards.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, count) in sorted_cards {
        let valid_counts: Vec<serde_json::Value> = (0..=count)
            .map(|i| serde_json::json!(i))
            .collect();
        maindeck_props.insert(name, serde_json::json!({
            "type": "integer",
            "enum": valid_counts
        }));
    }

    let basic_land_count = serde_json::json!({ "type": "integer" });
    serde_json::json!({
        "type": "object",
        "properties": {
            "thoughts": {
                "type": "string",
                "description": "Concise but complete summary of your deck construction reasoning"
            },
            "maindeck": {
                "type": "object",
                "properties": maindeck_props,
                "description": "How many copies of each card to include in the maindeck (0 to skip)"
            },
            "lands": {
                "type": "object",
                "properties": {
                    "Plains":   basic_land_count.clone(),
                    "Island":   basic_land_count.clone(),
                    "Swamp":    basic_land_count.clone(),
                    "Mountain": basic_land_count.clone(),
                    "Forest":   basic_land_count.clone()
                },
                "description": "Number of each basic land to include (typically 16-18 total for a 40-card deck)"
            }
        },
        "required": ["thoughts", "maindeck", "lands"]
    })
}

/// Transform a JSON schema to be Anthropic-compatible:
/// - add `additionalProperties: false` to every object
/// - strip unsupported numeric constraints (`minimum`, `maximum`, `multipleOf`)
/// - strip the `thoughts` field from `properties` and `required` (reasoning
///   happens in the extended-thinking channel, not inside the JSON payload)
fn sanitize_schema_for_anthropic(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                if key == "minimum" || key == "maximum" || key == "multipleOf" {
                    continue;
                }
                new_map.insert(key.clone(), sanitize_schema_for_anthropic(val));
            }
            if new_map.get("type").and_then(|t| t.as_str()) == Some("object") {
                new_map
                    .entry("additionalProperties".to_string())
                    .or_insert(serde_json::Value::Bool(false));
                if let Some(props) = new_map.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    props.remove("thoughts");
                }
                if let Some(req) = new_map.get_mut("required").and_then(|r| r.as_array_mut()) {
                    req.retain(|v| v.as_str() != Some("thoughts"));
                }
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter().map(sanitize_schema_for_anthropic).collect(),
        ),
        other => other.clone(),
    }
}

/// Shared draft rules (used by all backends).
fn build_draft_rules(set_name: &str, guide: Option<&str>, card_reference: &str) -> String {
    let guide_section = guide
        .map(|g| format!("\n## Draft Guide\n\n{g}\n"))
        .unwrap_or_default();
    format!(
        r#"You are drafting Magic: The Gathering cards from {set_name}.
{guide_section}
## How drafting works
- You'll open 3 packs of ~14 cards each
- For each pack, pick one card, then the remaining cards pass to the next player
- Pack 1 passes left, Pack 2 passes right, Pack 3 passes left
- After drafting, you'll build a 40-card deck from your picks plus basic lands

## How to pick
- Build toward 2 colors (sometimes splashing a 3rd)
- Value bombs (powerful rares), removal, evasion (flying), then curve fillers
- Read signals: if strong cards of a color keep coming, that color is open
- Cards with "//" are double-faced cards; evaluate the front face for drafting

## Card reference

{card_reference}"#,
    )
}

/// Build a card reference string with oracle text for all cards in the set.
pub fn build_card_reference(
    card_names: &[String],
    registry: &mtg_engine::cards::CardRegistry,
) -> String {
    use mtg_engine::types::CardType;

    let mut s = String::new();
    for name in card_names {
        let lookup = name.split(" // ").next().unwrap_or(name);
        let Some(id) = registry.get_id_by_name(lookup) else { continue };
        let Some(data) = registry.card_data(id) else { continue };

        let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
        let types: Vec<&str> = data.card_types.iter().map(|t| match t {
            CardType::Creature => "Creature",
            CardType::Instant => "Instant",
            CardType::Sorcery => "Sorcery",
            CardType::Enchantment => "Enchantment",
            CardType::Artifact => "Artifact",
            CardType::Land => "Land",
            CardType::Planeswalker => "Planeswalker",
        }).collect();
        let subtypes = if data.subtypes.is_empty() { String::new() }
            else { format!(" — {}", data.subtypes.join(" ")) };
        let pt = match (data.power, data.toughness) {
            (Some(p), Some(t)) => format!(" {p}/{t}"),
            _ => String::new(),
        };
        writeln!(s, "{}{} | {}{}{}", name, cost, types.join(" "), subtypes, pt).unwrap();
        if !data.oracle_text.is_empty() {
            writeln!(s, "  {}", data.oracle_text.replace('\n', "\n  ")).unwrap();
        }
    }
    s
}

/// Anthropic draft backend.
struct AnthropicDraftBackend {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: String,
    conversation: Vec<serde_json::Value>,
}

impl AnthropicDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, card_reference: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        let system_prompt = format!(
            "{}\n\n## Response format\nYour responses are constrained by a JSON schema provided via the API's structured output mode. Always reply with exactly the JSON object matching the schema — no surrounding prose, no markdown fences.\n\nYour private reasoning happens in the model's extended-thinking channel — think through the situation there before producing the JSON. The JSON payload itself should contain ONLY the response fields in the schema; do NOT add a \"thoughts\" key, it will be rejected by the schema validator.",
            build_draft_rules(set_name, guide, card_reference)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            system_prompt,
            conversation: Vec::new(),
        }
    }

    /// Send a message to Anthropic with the given JSON schema as
    /// structured output. Returns the raw response text (which should
    /// be valid JSON matching the schema).
    fn call_api_structured(
        &self,
        messages: &[serde_json::Value],
        schema: &serde_json::Value,
    ) -> String {
        let system = serde_json::json!([{
            "type": "text",
            "text": &self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        if msgs.len() >= 2 {
            let idx = msgs.len() - 2;
            if let Some(content) = msgs[idx].get("content").and_then(|c| c.as_str()).map(std::string::ToString::to_string) {
                msgs[idx] = serde_json::json!({
                    "role": msgs[idx]["role"],
                    "content": [{"type": "text", "text": content, "cache_control": {"type": "ephemeral"}}]
                });
            }
        }

        let sanitized = sanitize_schema_for_anthropic(schema);
        let body = serde_json::json!({
            "model": &self.model,
            "max_tokens": 8192,
            "thinking": {
                "type": "enabled",
                "budget_tokens": 4096
            },
            "system": system,
            "messages": msgs,
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": sanitized
                }
            }
        });

        for attempt in 0..6 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(u32::try_from(attempt.min(4)).unwrap_or(0)));
                std::thread::sleep(delay);
            }
            let response = self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_anthropic_usage(&self.model, &json["usage"]);
                    let text = json["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if text.is_empty() {
                        let msg = format!("Anthropic returned empty text (attempt {}/6)", attempt + 1);
                        eprintln!("WARN: {msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_WARN", &msg);
                        continue;
                    }
                    return text;
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if code == 529 || code == 429 {
                        let msg = format!("Anthropic {} (attempt {}/6)", code, attempt + 1);
                        eprintln!("{msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_RETRY", &msg);
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    let msg = format!("Anthropic API error {}: {}", code, &text[..text.len().min(200)]);
                    mtg_player::game_log::write(file!(), line!(), "API_FATAL", &msg);
                    panic!("{}", msg);
                }
                Err(e) => {
                    let msg = format!("Anthropic request failed (attempt {}/6): {}", attempt + 1, e);
                    eprintln!("{msg}");
                    mtg_player::game_log::write(file!(), line!(), "API_ERROR", &msg);
                }
            }
        }
        let msg = "Anthropic draft API exhausted all 6 retries";
        mtg_player::game_log::write(file!(), line!(), "API_FATAL", msg);
        panic!("{}", msg)
    }

    fn send_conv_structured(
        &mut self,
        conv: &mut Vec<serde_json::Value>,
        message: &str,
        schema: &serde_json::Value,
    ) -> String {
        conv.push(serde_json::json!({"role": "user", "content": message}));
        let resp = self.call_api_structured(conv, schema);
        conv.push(serde_json::json!({"role": "assistant", "content": &resp}));
        resp
    }
}

impl DraftBackend for AnthropicDraftBackend {
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String {
        let schema = pick_schema_for(num_cards);
        let mut conv = std::mem::take(&mut self.conversation);
        let result = self.send_conv_structured(&mut conv, message, &schema);
        self.conversation = conv;
        result
    }

    // Deckbuilding shares the same conversation as pick selection so the model
    // can look back at every pack it saw and every pick it made.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String {
        let schema = deck_schema_for(pool);
        let mut conv = std::mem::take(&mut self.conversation);
        let result = self.send_conv_structured(&mut conv, message, &schema);
        self.conversation = conv;
        result
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

}

/// Gemini draft backend using the Interactions API.
struct GeminiDraftBackend {
    client: Client,
    api_key: String,
    model: String,
    draft_thinking: Option<String>,
    system_prompt: String,
    /// Single interaction chain shared by both pick and deckbuilding turns,
    /// so deckbuilding inherits the full draft history.
    interaction_id: Option<String>,
}

impl GeminiDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, draft_thinking: Option<String>, card_reference: &str) -> Self {
        let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let system_prompt = format!(
            "{}\n\nYour responses are constrained by a JSON schema provided via the API's structured output mode. Use the \"thoughts\" field for a concise but complete summary of your reasoning.",
            build_draft_rules(set_name, guide, card_reference)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            draft_thinking,
            system_prompt,
            interaction_id: None,
        }
    }

    fn call_interactions(
        &self,
        message: &str,
        schema: &serde_json::Value,
        prev_id: Option<&str>,
    ) -> (String, Option<String>) {
        let mut body = serde_json::json!({
            "model": &self.model,
            "input": message,
            "response_mime_type": "application/json",
            "response_format": schema,
        });

        if let Some(ref level) = self.draft_thinking {
            body["generation_config"] = serde_json::json!({"thinking_level": level});
        }

        // Only chain if we have a non-empty previous interaction ID.
        if let Some(prev) = prev_id.filter(|s| !s.is_empty()) {
            body["previous_interaction_id"] = serde_json::json!(prev);
        } else {
            body["system_instruction"] = serde_json::json!(&self.system_prompt);
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/interactions?key={}",
            self.api_key
        );

        let mut fresh_retry = false;
        for attempt in 0..6 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(u32::try_from(attempt.min(4)).unwrap_or(0)));
                std::thread::sleep(delay);
            }
            let response = self.client
                .post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().unwrap_or_default();
                    record_gemini_usage(&self.model, &json["usage"]);
                    let id = json["id"].as_str()
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string);
                    let mut text = String::new();
                    if let Some(outputs) = json["outputs"].as_array() {
                        for out in outputs {
                            if out["type"].as_str() == Some("text") {
                                if let Some(t) = out["text"].as_str() {
                                    text = t.trim().to_string();
                                }
                            }
                        }
                    }
                    if text.is_empty() {
                        let msg = format!("Gemini returned empty text (attempt {}/6, interaction_id: {:?})", attempt + 1, id);
                        eprintln!("WARN: {msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_WARN", &msg);
                        continue;
                    }
                    return (text, id);
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if code == 429 || code == 503 {
                        let err_text = resp.text().unwrap_or_default();
                        let msg = format!("Gemini {} (attempt {}/6): {}", code, attempt + 1, &err_text[..err_text.len().min(150)]);
                        eprintln!("{msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_RETRY", &msg);
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    // If the interaction ID is invalid, fall back to a fresh conversation.
                    if code == 400 && text.contains("previous_interaction_id") && !fresh_retry {
                        let msg = format!("Invalid interaction ID, falling back to fresh conversation ({})", &text[..text.len().min(150)]);
                        eprintln!("WARN: {msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_WARN", &msg);
                        body.as_object_mut().unwrap().remove("previous_interaction_id");
                        body["system_instruction"] = serde_json::json!(&self.system_prompt);
                        fresh_retry = true;
                        continue;
                    }
                    // Fatal config errors — abort loudly so we don't silently produce garbage.
                    if code == 400 && (text.contains("thinking level") || text.contains("not a supported")) {
                        let msg = format!("Gemini config error: {}", &text[..text.len().min(300)]);
                        eprintln!("FATAL: {msg}");
                        mtg_player::game_log::write(file!(), line!(), "API_FATAL", &msg);
                        std::process::exit(1);
                    }
                    let msg = format!("Gemini API error {}: {}", code, &text[..text.len().min(200)]);
                    mtg_player::game_log::write(file!(), line!(), "API_FATAL", &msg);
                    panic!("{}", msg);
                }
                Err(e) => {
                    let msg = format!("Gemini request failed (attempt {}/6): {}", attempt + 1, e);
                    eprintln!("{msg}");
                    mtg_player::game_log::write(file!(), line!(), "API_ERROR", &msg);
                }
            }
        }
        let msg = "Gemini draft API exhausted all 6 retries";
        mtg_player::game_log::write(file!(), line!(), "API_FATAL", msg);
        panic!("{}", msg)
    }

    /// Re-serialize the Gemini JSON response to a canonical pretty form.
    /// Returns the raw response unchanged if it's not valid JSON.
    fn normalize_response(raw: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()),
            Err(_) => raw.to_string(),
        }
    }
}

impl DraftBackend for GeminiDraftBackend {
    fn send_pick(&mut self, message: &str, num_cards: usize) -> String {
        let prev = self.interaction_id.take();
        let schema = pick_schema_for(num_cards);
        let (raw, new_id) = self.call_interactions(message, &schema, prev.as_deref());
        self.interaction_id = new_id;
        Self::normalize_response(&raw)
    }

    // Deckbuilding chains off the same interaction id as pick selection so
    // the model has the full draft history (packs, picks, prior reasoning)
    // in context when building its deck.
    fn send_deck_building(&mut self, message: &str, pool: &[String]) -> String {
        let prev = self.interaction_id.take();
        let schema = deck_schema_for(pool);
        let (raw, new_id) = self.call_interactions(message, &schema, prev.as_deref());
        self.interaction_id = new_id;
        Self::normalize_response(&raw)
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}

/// LLM client for draft picks and deck building.
pub struct DraftLlmClient {
    backend: Box<dyn DraftBackend>,
}

impl DraftLlmClient {
    /// Create a new `DraftLlmClient`.
    /// Model spec format: "`provider:model:draft_thinking:game_thinking`"
    pub fn new(model_spec: &str, set_name: &str, guide: Option<&str>, card_reference: &str) -> Self {
        let parts: Vec<&str> = model_spec.split(':').collect();
        let provider_name = parts[0];
        let model_override = parts.get(1).copied();
        let draft_thinking_override = parts.get(2).map(std::string::ToString::to_string);
        let _game_thinking_override = parts.get(3).map(std::string::ToString::to_string);

        if provider_name == "gemini" {
            let model = model_override.unwrap_or("gemini-2.5-flash");
            let draft_thinking = draft_thinking_override.unwrap_or_else(|| "high".to_string());
            Self {
                backend: Box::new(GeminiDraftBackend::new(model, set_name, guide, Some(draft_thinking), card_reference)),
            }
        } else {
            let model = model_override.unwrap_or("claude-sonnet-4-6");
            Self {
                backend: Box::new(AnthropicDraftBackend::new(model, set_name, guide, card_reference)),
            }
        }
    }

    /// Build the prompt for a draft pick.
    pub fn build_pick_prompt(
        pack_number: usize,
        pick_index: usize,
        available: &[String],
        pool: &[String],
        _history: &[DraftPick],
    ) -> String {
        let direction = if pack_number % 2 == 1 { "LEFT" } else { "RIGHT" };
        let mut prompt = format!(
            "Pack {}, Pick {} ({} cards). Passing {}.\n\nAvailable:\n",
            pack_number, pick_index, available.len(), direction,
        );
        for (i, card) in available.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            writeln!(prompt, "{i}: {name}").unwrap();
        }
        if pick_index == 1 && !pool.is_empty() {
            writeln!(prompt, "\nYour pool so far ({} cards):", pool.len()).unwrap();
            for card in pool {
                let name = card.split(" // ").next().unwrap_or(card);
                writeln!(prompt, "- {name}").unwrap();
            }
        }
        prompt
    }

    /// Whether this client's backend expects thoughts in the JSON payload.
    /// Exposed so the caller (`main.rs::build_deck_prompt`) can render the
    /// deck-building response-format hint with or without the thoughts
    /// prefix.
    /// Send a pick message with the pack size, so the backend can build
    /// an enum-constrained pick schema for structured decoding.
    pub fn send_pick_message(&mut self, user_message: &str, num_cards: usize) -> String {
        self.backend.send_pick(user_message, num_cards)
    }

    pub fn record_pick(_chosen: &str) {}

    /// Send a deck-building message with the player's full card pool, so
    /// the backend can build an enum-constrained `maindeck` schema that
    /// only accepts cards the model actually drafted.
    pub fn send_deck_building_message(&mut self, user_message: &str, pool: &[String]) -> String {
        self.backend.send_deck_building(user_message, pool)
    }

    /// The full system prompt this client will send to the model.
    pub fn system_prompt(&self) -> &str {
        self.backend.system_prompt()
    }
}
