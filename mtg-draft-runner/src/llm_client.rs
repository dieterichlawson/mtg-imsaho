use std::env;

use reqwest::blocking::Client;

use mtg_draft::draft::DraftPick;

/// Per-model token usage tracking. Thread-safe via Mutex.
use std::sync::Mutex;
use std::collections::HashMap;

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

/// Known model pricing ($/MTok). (input, output, cache_read, cache_write)
/// Anthropic: platform.claude.com/docs/en/about-claude/pricing (verified 2026-04-08)
/// Gemini: ai.google.dev/pricing (verified 2026-04-08)
fn model_pricing(model: &str) -> (f64, f64, f64, f64) {
    match model {
        // Anthropic models (cache_read = 0.1x input, cache_write = 1.25x input for 5-min TTL)
        m if m.contains("opus-4-6") => (5.00, 25.00, 0.50, 6.25),
        m if m.contains("opus-4-5") => (5.00, 25.00, 0.50, 6.25),
        m if m.contains("opus-4-1") => (15.00, 75.00, 1.50, 18.75),
        m if m.contains("sonnet-4-6") => (3.00, 15.00, 0.30, 3.75),
        m if m.contains("sonnet-4-5") => (3.00, 15.00, 0.30, 3.75),
        m if m.contains("sonnet-4-0") | m.contains("sonnet-4-2") => (3.00, 15.00, 0.30, 3.75),
        m if m.contains("haiku-4-5") => (1.00, 5.00, 0.10, 1.25),
        m if m.contains("haiku-3-5") => (0.80, 4.00, 0.08, 1.00),
        // Gemini models (cache_read = 0.1x input, implicit caching has no write cost)
        m if m.contains("gemini-2.5-flash-lite") => (0.10, 0.40, 0.01, 0.0),
        m if m.contains("gemini-2.5-flash") && !m.contains("lite") => (0.30, 2.50, 0.03, 0.0),
        m if m.contains("gemini-2.5-pro") => (1.25, 10.00, 0.125, 0.0),
        m if m.contains("gemini-3.1-flash-lite") => (0.25, 1.50, 0.025, 0.0),
        m if m.contains("gemini-3.1-pro") => (2.00, 12.00, 0.20, 0.0),
        m if m.contains("gemini-3-flash") || m.contains("gemini-3.0-flash") => (0.50, 3.00, 0.05, 0.0),
        m if m.contains("gemini-3-pro") || m.contains("gemini-3.0-pro") => (2.00, 12.00, 0.20, 0.0),
        m if m.contains("gemini") => (0.30, 2.50, 0.03, 0.0), // default gemini → 2.5 flash pricing
        _ => (3.00, 15.00, 0.30, 3.75), // unknown model → sonnet pricing
    }
}

fn usage_cost(u: &ModelUsage, model: &str) -> f64 {
    let (in_p, out_p, cache_r_p, cache_w_p) = model_pricing(model);
    (u.input as f64) * in_p / 1_000_000.0
        + (u.output as f64) * out_p / 1_000_000.0
        + (u.cache_read as f64) * cache_r_p / 1_000_000.0
        + (u.cache_create as f64) * cache_w_p / 1_000_000.0
}

/// Print a summary of all token usage and estimated cost, broken down by model and phase.
pub fn print_usage_summary(total_games: usize) {
    let draft_usage = get_model_usage();
    let game_usage = mtg_player::llm::get_llm_model_usage();

    // Draft phase cost
    let mut draft_cost = 0.0;
    let mut draft_calls = 0u64;
    for (model, u) in &draft_usage {
        draft_cost += usage_cost(u, model);
        draft_calls += u.calls;
    }

    // Game phase cost
    let mut game_cost = 0.0;
    let mut game_calls = 0u64;
    for (model, u) in &game_usage {
        let (in_p, out_p, cache_r_p, cache_w_p) = model_pricing(model);
        game_cost += (u.input as f64) * in_p / 1_000_000.0
            + (u.output as f64) * out_p / 1_000_000.0
            + (u.cache_read as f64) * cache_r_p / 1_000_000.0
            + (u.cache_create as f64) * cache_w_p / 1_000_000.0;
        game_calls += u.calls;
    }

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

    summary.push_str(&format!("  Draft:  {} calls, ${:.4}\n", draft_calls, draft_cost));
    summary.push_str(&format!("  Games:  {} calls, ${:.4}{}\n", game_calls, game_cost,
        if total_games > 0 { format!(" ({} games, ${:.4}/game avg)", total_games, game_cost / total_games as f64) } else { String::new() }));
    summary.push('\n');

    let mut models: Vec<_> = combined.iter().collect();
    models.sort_by_key(|(name, _)| (*name).clone());
    let total_cost = draft_cost + game_cost;

    for (model, u) in &models {
        let cost = usage_cost(u, model);
        summary.push_str(&format!(
            "  {}: {} calls, {}in/{}out/{}cached = ${:.4}\n",
            model, u.calls, u.input, u.output, u.cache_read, cost
        ));
    }
    summary.push_str(&format!("  ---\n  Total: {} calls, ${:.2}\n", draft_calls + game_calls, total_cost));

    // Write to stderr
    eprint!("\n{}", summary);

    // Write to log file
    mtg_player::game_log::write(file!(), line!(), "TOKEN USAGE", &summary);
}

/// LLM client for draft picks and deck building.
/// Maintains a multi-turn conversation for the draft process.
/// Backend trait for draft LLM communication.
trait DraftBackend: Send {
    /// Send a draft pick message. Returns "PICK: N" format text.
    fn send_pick(&mut self, message: &str) -> String;
    /// Send a deck building message. Returns "MAINDECK:\n...\nLANDS:\n..." format text.
    fn send_deck_building(&mut self, message: &str) -> String;
    /// The full system prompt this backend will send to the model.
    fn system_prompt(&self) -> &str;
}

/// Shared draft rules (used by all backends).
fn build_draft_rules(set_name: &str, guide: Option<&str>, card_reference: &str) -> String {
    let guide_section = guide
        .map(|g| format!("\n## Draft Guide\n\n{}\n", g))
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
        set_name = set_name,
        guide_section = guide_section,
        card_reference = card_reference,
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

        let cost = data.cost.as_ref().map(|c| format!(" {}", c)).unwrap_or_default();
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
            (Some(p), Some(t)) => format!(" {}/{}", p, t),
            _ => String::new(),
        };
        s.push_str(&format!("{}{} | {}{}{}\n", name, cost, types.join(" "), subtypes, pt));
        if !data.oracle_text.is_empty() {
            s.push_str(&format!("  {}\n", data.oracle_text.replace('\n', "\n  ")));
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
    deck_conversation: Vec<serde_json::Value>,
}

impl AnthropicDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, card_reference: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        let system_prompt = format!(
            "{}\n\n## Response format\nThink through your pick, then on your final line write ONLY:\nPICK: <number>\nwhere <number> is the 0-indexed number of the card you want.",
            build_draft_rules(set_name, guide, card_reference)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            system_prompt,
            conversation: Vec::new(),
            deck_conversation: Vec::new(),
        }
    }

    fn call_api(&self, messages: &[serde_json::Value]) -> String {
        let system = serde_json::json!([{
            "type": "text",
            "text": &self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        if msgs.len() >= 2 {
            let idx = msgs.len() - 2;
            if let Some(content) = msgs[idx].get("content").and_then(|c| c.as_str()).map(|s| s.to_string()) {
                msgs[idx] = serde_json::json!({
                    "role": msgs[idx]["role"],
                    "content": [{"type": "text", "text": content, "cache_control": {"type": "ephemeral"}}]
                });
            }
        }

        let body = serde_json::json!({
            "model": &self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": msgs
        });

        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_secs(2u64.pow(attempt as u32)));
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
                    return json["content"][0]["text"].as_str().unwrap_or("PICK: 0").trim().to_string();
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if code == 529 || code == 429 { continue; }
                    let text = resp.text().unwrap_or_default();
                    eprintln!("Anthropic API error {}: {}", code, &text[..text.len().min(200)]);
                    return "PICK: 0".to_string();
                }
                Err(e) => { eprintln!("Request failed: {}", e); continue; }
            }
        }
        "PICK: 0".to_string()
    }

    fn send_conv(&mut self, conv: &mut Vec<serde_json::Value>, message: &str) -> String {
        conv.push(serde_json::json!({"role": "user", "content": message}));
        let resp = self.call_api(conv);
        conv.push(serde_json::json!({"role": "assistant", "content": &resp}));
        resp
    }
}

impl DraftBackend for AnthropicDraftBackend {
    fn send_pick(&mut self, message: &str) -> String {
        let mut conv = std::mem::take(&mut self.conversation);
        let result = self.send_conv(&mut conv, message);
        self.conversation = conv;
        result
    }

    fn send_deck_building(&mut self, message: &str) -> String {
        let mut conv = std::mem::take(&mut self.deck_conversation);
        let result = self.send_conv(&mut conv, message);
        self.deck_conversation = conv;
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
    pick_interaction_id: Option<String>,
    deck_interaction_id: Option<String>,
}

impl GeminiDraftBackend {
    fn new(model: &str, set_name: &str, guide: Option<&str>, draft_thinking: Option<String>, card_reference: &str) -> Self {
        let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
        let system_prompt = format!(
            "{}\n\nYou will respond with structured JSON. Use the \"thoughts\" field for your reasoning.",
            build_draft_rules(set_name, guide, card_reference)
        );
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            draft_thinking,
            system_prompt,
            pick_interaction_id: None,
            deck_interaction_id: None,
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
                let delay = std::time::Duration::from_secs(2u64.pow(attempt.min(4) as u32));
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
                        .map(|s| s.to_string());
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
                    if text.is_empty() { text = r#"{"pick": 0}"#.to_string(); }
                    return (text, id);
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if code == 429 || code == 503 {
                        if let Ok(err_text) = resp.text() {
                            eprintln!("Gemini {} (attempt {}/6): {}", code, attempt + 1, &err_text[..err_text.len().min(150)]);
                        }
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    // If the interaction ID is invalid, fall back to a fresh conversation.
                    if code == 400 && text.contains("previous_interaction_id") && !fresh_retry {
                        eprintln!("WARN: Invalid interaction ID, falling back to fresh conversation");
                        body.as_object_mut().unwrap().remove("previous_interaction_id");
                        body["system_instruction"] = serde_json::json!(&self.system_prompt);
                        fresh_retry = true;
                        continue;
                    }
                    // Fatal config errors — abort loudly so we don't silently produce garbage.
                    if code == 400 && (text.contains("thinking level") || text.contains("not a supported")) {
                        eprintln!("FATAL: Gemini config error: {}", &text[..text.len().min(300)]);
                        std::process::exit(1);
                    }
                    eprintln!("Gemini API error {}: {}", code, &text[..text.len().min(200)]);
                    return (r#"{"pick": 0}"#.to_string(), None);
                }
                Err(e) => { eprintln!("Request failed: {}", e); continue; }
            }
        }
        eprintln!("WARN: Gemini API exhausted all retries");
        (r#"{"pick": 0}"#.to_string(), None)
    }

    fn pick_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Brief reasoning for this pick"},
                "pick": {"type": "integer", "minimum": 0, "description": "0-indexed number of the card to pick"}
            },
            "required": ["thoughts", "pick"]
        })
    }

    fn deck_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Brief reasoning for deck construction choices"},
                "maindeck": {"type": "array", "items": {"type": "string"}, "description": "Card names for the maindeck"},
                "lands": {"type": "object", "properties": {
                    "Plains": {"type": "integer"},
                    "Island": {"type": "integer"},
                    "Swamp": {"type": "integer"},
                    "Mountain": {"type": "integer"},
                    "Forest": {"type": "integer"}
                }, "description": "Basic land counts"}
            },
            "required": ["thoughts", "maindeck", "lands"]
        })
    }

    fn parse_pick_response(raw: &str) -> String {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
            let pick = parsed["pick"].as_u64().unwrap_or(0);
            let thoughts = parsed["thoughts"].as_str().unwrap_or("");
            if thoughts.is_empty() {
                format!("PICK: {}", pick)
            } else {
                format!("Thoughts: {}\n\nPICK: {}", thoughts, pick)
            }
        } else {
            raw.to_string()
        }
    }

    fn parse_deck_response(raw: &str) -> String {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
            let thoughts = parsed["thoughts"].as_str().unwrap_or("");
            let mut text = String::new();
            if !thoughts.is_empty() {
                text.push_str(&format!("Thoughts: {}\n\n", thoughts));
            }
            text.push_str("MAINDECK:\n");
            if let Some(cards) = parsed["maindeck"].as_array() {
                for card in cards {
                    if let Some(name) = card.as_str() {
                        text.push_str(&format!("{}\n", name));
                    }
                }
            }
            text.push_str("\nLANDS:\n");
            if let Some(lands) = parsed["lands"].as_object() {
                for (name, count) in lands {
                    if let Some(n) = count.as_u64() {
                        if n > 0 {
                            text.push_str(&format!("{} {}\n", n, name));
                        }
                    }
                }
            }
            text
        } else {
            raw.to_string()
        }
    }
}

impl DraftBackend for GeminiDraftBackend {
    fn send_pick(&mut self, message: &str) -> String {
        let prev = self.pick_interaction_id.take();
        let (raw, new_id) = self.call_interactions(message, &Self::pick_schema(), prev.as_deref());
        self.pick_interaction_id = new_id;
        Self::parse_pick_response(&raw)
    }

    fn send_deck_building(&mut self, message: &str) -> String {
        let prev = self.deck_interaction_id.take();
        let (raw, new_id) = self.call_interactions(message, &Self::deck_schema(), prev.as_deref());
        self.deck_interaction_id = new_id;
        Self::parse_deck_response(&raw)
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
    /// Create a new DraftLlmClient.
    /// Model spec format: "provider:model:draft_thinking:game_thinking"
    pub fn new(model_spec: &str, set_name: &str, guide: Option<&str>, card_reference: &str) -> Self {
        let parts: Vec<&str> = model_spec.split(':').collect();
        let provider_name = parts[0];
        let model_override = parts.get(1).copied();
        let draft_thinking_override = parts.get(2).map(|s| s.to_string());
        let _game_thinking_override = parts.get(3).map(|s| s.to_string());

        match provider_name {
            "gemini" => {
                let model = model_override.unwrap_or("gemini-2.5-flash");
                let draft_thinking = draft_thinking_override.unwrap_or_else(|| "high".to_string());
                Self {
                    backend: Box::new(GeminiDraftBackend::new(model, set_name, guide, Some(draft_thinking), card_reference)),
                }
            }
            _ => {
                let model = model_override.unwrap_or("claude-sonnet-4-6");
                Self {
                    backend: Box::new(AnthropicDraftBackend::new(model, set_name, guide, card_reference)),
                }
            }
        }
    }

    /// Build the prompt for a draft pick.
    pub fn build_pick_prompt(
        &self,
        pack_number: usize,
        pick_number: usize,
        available: &[String],
        pool: &[String],
        _history: &[DraftPick],
    ) -> String {
        let direction = if pack_number % 2 == 1 { "LEFT" } else { "RIGHT" };
        let mut prompt = format!(
            "Pack {}, Pick {} ({} cards). Passing {}.\n\nAvailable:\n",
            pack_number, pick_number, available.len(), direction,
        );
        for (i, card) in available.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            prompt.push_str(&format!("{}: {}\n", i, name));
        }
        if pick_number == 1 && !pool.is_empty() {
            prompt.push_str(&format!("\nYour pool so far ({} cards):\n", pool.len()));
            for card in pool {
                let name = card.split(" // ").next().unwrap_or(card);
                prompt.push_str(&format!("- {}\n", name));
            }
        }
        prompt.push_str(&format!("\nPick a card (0-{}):", available.len().saturating_sub(1)));
        prompt
    }

    pub fn send_message(&mut self, user_message: &str) -> String {
        self.backend.send_pick(user_message)
    }

    pub fn record_pick(&mut self, _chosen: &str) {}

    pub fn send_deck_building_message(&mut self, user_message: &str) -> String {
        self.backend.send_deck_building(user_message)
    }

    /// The full system prompt this client will send to the model.
    pub fn system_prompt(&self) -> &str {
        self.backend.system_prompt()
    }
}
