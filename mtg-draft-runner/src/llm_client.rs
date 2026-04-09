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

/// Record token usage from a Gemini API response.
pub fn record_gemini_usage(model: &str, usage_metadata: &serde_json::Value) {
    record_model_usage(
        model,
        usage_metadata["promptTokenCount"].as_u64().unwrap_or(0),
        usage_metadata["candidatesTokenCount"].as_u64().unwrap_or(0),
        usage_metadata["cachedContentTokenCount"].as_u64().unwrap_or(0),
        0,
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
fn model_pricing(model: &str) -> (f64, f64, f64, f64) {
    match model {
        m if m.contains("opus-4-6") => (5.00, 25.00, 0.50, 6.25),
        m if m.contains("sonnet-4-6") => (3.00, 15.00, 0.30, 3.75),
        m if m.contains("haiku-4-5") => (1.00, 5.00, 0.10, 1.25),
        m if m.contains("gemini-2.5-flash-lite") => (0.10, 0.40, 0.025, 0.0),
        m if m.contains("gemini-2.5-flash") => (0.30, 2.50, 0.075, 0.0),
        m if m.contains("gemini-3.1-flash-lite") => (0.25, 1.50, 0.0625, 0.0),
        m if m.contains("gemini-3-flash") || m.contains("gemini-3.0-flash") => (0.50, 3.00, 0.125, 0.0),
        m if m.contains("gemini") => (0.30, 2.50, 0.075, 0.0), // default gemini
        _ => (3.00, 15.00, 0.30, 3.75), // default to sonnet pricing
    }
}

/// Print a summary of all token usage and estimated cost, broken down by model.
pub fn print_usage_summary() {
    // Merge draft client usage and LlmPlayer game usage
    let draft_usage = get_model_usage();
    let game_usage = mtg_player::llm::get_llm_model_usage();

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

    eprintln!("\n=== Token Usage by Model ===");
    let mut total_cost = 0.0;
    let mut total_calls = 0u64;

    let mut models: Vec<_> = combined.iter().collect();
    models.sort_by_key(|(name, _)| (*name).clone());

    for (model, u) in &models {
        let (in_p, out_p, cache_r_p, cache_w_p) = model_pricing(model);
        let cost = (u.input as f64) * in_p / 1_000_000.0
            + (u.output as f64) * out_p / 1_000_000.0
            + (u.cache_read as f64) * cache_r_p / 1_000_000.0
            + (u.cache_create as f64) * cache_w_p / 1_000_000.0;
        total_cost += cost;
        total_calls += u.calls;

        eprintln!(
            "  {}: {} calls, {}in + {}out + {}cache_rd + {}cache_wr = ${:.4}",
            model, u.calls, u.input, u.output, u.cache_read, u.cache_create, cost
        );
    }
    eprintln!("  ---");
    eprintln!("  Total: {} calls, ${:.2}", total_calls, total_cost);
}

/// LLM client for draft picks and deck building.
/// Maintains a multi-turn conversation for the draft process.
pub struct DraftLlmClient {
    client: Client,
    api_key: String,
    model: String,
    provider: Provider,
    system_prompt: String,
    /// Draft conversation history (picks).
    conversation: Vec<serde_json::Value>,
    /// Separate conversation for deck building.
    deck_conversation: Vec<serde_json::Value>,
}

enum Provider {
    Anthropic,
    Gemini,
}

impl DraftLlmClient {
    pub fn new(model_spec: &str, set_name: &str, guide: Option<&str>) -> Self {
        let parts: Vec<&str> = model_spec.splitn(2, ':').collect();
        let provider_name = parts[0];
        let model_override = parts.get(1).copied();

        let (provider, api_key, default_model) = match provider_name {
            "gemini" => (
                Provider::Gemini,
                env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set"),
                "gemini-2.5-flash",
            ),
            _ => (
                Provider::Anthropic,
                env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set"),
                "claude-sonnet-4-6",
            ),
        };

        let model = model_override
            .unwrap_or(default_model)
            .to_string();

        let guide_section = guide
            .map(|g| format!("\n## Draft Guide\n\n{}\n", g))
            .unwrap_or_default();

        let system_prompt = format!(
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

## Response format
Think through your pick, then on your final line write ONLY:
PICK: <number>
where <number> is the 0-indexed number of the card you want."#,
            set_name = set_name,
            guide_section = guide_section,
        );

        Self {
            client: Client::new(),
            api_key,
            model,
            provider,
            system_prompt,
            conversation: Vec::new(),
            deck_conversation: Vec::new(),
        }
    }

    /// Build the prompt for a draft pick.
    pub fn build_pick_prompt(
        &self,
        pack_number: usize,
        pick_number: usize,
        available: &[String],
        pool: &[String],
        history: &[DraftPick],
    ) -> String {
        let direction = if pack_number % 2 == 1 { "LEFT" } else { "RIGHT" };
        let mut prompt = format!(
            "Pack {}, Pick {} ({} cards). Passing {}.\n\nAvailable:\n",
            pack_number,
            pick_number,
            available.len(),
            direction,
        );

        for (i, card) in available.iter().enumerate() {
            let name = card.split(" // ").next().unwrap_or(card);
            prompt.push_str(&format!("{}: {}\n", i, name));
        }

        if !pool.is_empty() {
            prompt.push_str(&format!("\nYour pool ({} cards):\n", pool.len()));
            for card in pool {
                let name = card.split(" // ").next().unwrap_or(card);
                prompt.push_str(&format!("- {}\n", name));
            }
        }

        if !history.is_empty() {
            prompt.push_str("\nPick history:\n");
            for pick in history {
                let chosen_name = pick.chosen.split(" // ").next().unwrap_or(&pick.chosen);
                prompt.push_str(&format!(
                    "P{}P{}: {} (from {} cards)\n",
                    pick.pack_number,
                    pick.pick_number,
                    chosen_name,
                    pick.available.len(),
                ));
            }
        }

        prompt.push_str(&format!(
            "\nPick a card (0-{}):",
            available.len().saturating_sub(1)
        ));
        prompt
    }

    /// Send a message to the LLM and get a response. Adds to conversation history.
    pub fn send_message(&mut self, user_message: &str) -> String {
        self.conversation.push(serde_json::json!({
            "role": "user",
            "content": user_message
        }));

        let response = match self.provider {
            Provider::Anthropic => self.call_anthropic(&self.conversation.clone()),
            Provider::Gemini => self.call_gemini(&self.conversation.clone()),
        };

        self.conversation.push(serde_json::json!({
            "role": "assistant",
            "content": &response
        }));

        response
    }

    /// Record a pick in the conversation (abbreviated to save tokens).
    pub fn record_pick(&mut self, _chosen: &str) {
        // The pick is already recorded via the send_message/response cycle.
        // No additional recording needed.
    }

    /// Send a deck building message (uses separate conversation).
    pub fn send_deck_building_message(&mut self, user_message: &str) -> String {
        self.deck_conversation.push(serde_json::json!({
            "role": "user",
            "content": user_message
        }));

        let response = match self.provider {
            Provider::Anthropic => self.call_anthropic(&self.deck_conversation.clone()),
            Provider::Gemini => self.call_gemini(&self.deck_conversation.clone()),
        };

        self.deck_conversation.push(serde_json::json!({
            "role": "assistant",
            "content": &response
        }));

        response
    }

    fn call_anthropic(&self, messages: &[serde_json::Value]) -> String {
        let system = serde_json::json!([{
            "type": "text",
            "text": &self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        // Set cache_control on second-to-last message for prompt caching
        if msgs.len() >= 2 {
            let idx = msgs.len() - 2;
            if let Some(content) = msgs[idx]
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
            {
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

        let body = serde_json::json!({
            "model": &self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": msgs
        });

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                std::thread::sleep(delay);
            }

            let response = self
                .client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        record_anthropic_usage(&self.model, &json["usage"]);
                        return json["content"][0]["text"]
                            .as_str()
                            .unwrap_or("PICK: 0")
                            .trim()
                            .to_string();
                    }

                    let code = resp.status().as_u16();
                    if code == 529 || code == 429 {
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    eprintln!("Anthropic API error {}: {}", code, &text[..text.len().min(200)]);
                    return "PICK: 0".to_string();
                }
                Err(e) => {
                    eprintln!("Request failed: {}", e);
                    continue;
                }
            }
        }

        "PICK: 0".to_string()
    }

    fn call_gemini(&self, messages: &[serde_json::Value]) -> String {
        let contents: Vec<serde_json::Value> = messages
            .iter()
            .map(|msg| {
                let role = msg["role"].as_str().unwrap_or("user");
                let gemini_role = if role == "assistant" { "model" } else { "user" };
                serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": msg["content"].as_str().unwrap_or("")}]
                })
            })
            .collect();

        let body = serde_json::json!({
            "contents": contents,
            "systemInstruction": {"parts": [{"text": &self.system_prompt}]},
            "generationConfig": {"maxOutputTokens": 4096}
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                std::thread::sleep(delay);
            }

            let response = self
                .client
                .post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        record_gemini_usage(&self.model, &json["usageMetadata"]);
                        return json["candidates"][0]["content"]["parts"][0]["text"]
                            .as_str()
                            .unwrap_or("PICK: 0")
                            .trim()
                            .to_string();
                    }

                    let code = resp.status().as_u16();
                    if code == 429 || code == 503 {
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    eprintln!("Gemini API error {}: {}", code, &text[..text.len().min(200)]);
                    return "PICK: 0".to_string();
                }
                Err(e) => {
                    eprintln!("Request failed: {}", e);
                    continue;
                }
            }
        }

        "PICK: 0".to_string()
    }
}
