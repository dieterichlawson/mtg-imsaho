use std::env;
use std::sync::atomic::{AtomicU64, Ordering};

use reqwest::blocking::Client;

use mtg_draft::draft::DraftPick;

/// Global token usage counters, safe to accumulate from multiple threads.
pub static TOTAL_INPUT_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_OUTPUT_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_CACHE_READ_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_CACHE_CREATION_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_API_CALLS: AtomicU64 = AtomicU64::new(0);

/// Record token usage from an Anthropic API response's usage object.
pub fn record_anthropic_usage(usage: &serde_json::Value) {
    TOTAL_API_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Some(n) = usage["input_tokens"].as_u64() {
        TOTAL_INPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["output_tokens"].as_u64() {
        TOTAL_OUTPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["cache_read_input_tokens"].as_u64() {
        TOTAL_CACHE_READ_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["cache_creation_input_tokens"].as_u64() {
        TOTAL_CACHE_CREATION_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
}

/// Record token usage from a Gemini API response's usageMetadata object.
pub fn record_gemini_usage(usage_metadata: &serde_json::Value) {
    TOTAL_API_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Some(n) = usage_metadata["promptTokenCount"].as_u64() {
        TOTAL_INPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage_metadata["candidatesTokenCount"].as_u64() {
        TOTAL_OUTPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage_metadata["cachedContentTokenCount"].as_u64() {
        TOTAL_CACHE_READ_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
}

/// Print a summary of all token usage and estimated cost.
/// Combines draft client counters with LlmPlayer game counters.
pub fn print_usage_summary() {
    use mtg_player::llm;

    // Draft client (picks + deck building)
    let draft_input = TOTAL_INPUT_TOKENS.load(Ordering::Relaxed);
    let draft_output = TOTAL_OUTPUT_TOKENS.load(Ordering::Relaxed);
    let draft_cache_read = TOTAL_CACHE_READ_TOKENS.load(Ordering::Relaxed);
    let draft_cache_create = TOTAL_CACHE_CREATION_TOKENS.load(Ordering::Relaxed);
    let draft_calls = TOTAL_API_CALLS.load(Ordering::Relaxed);

    // Game player (tournament)
    let game_input = llm::LLM_INPUT_TOKENS.load(Ordering::Relaxed);
    let game_output = llm::LLM_OUTPUT_TOKENS.load(Ordering::Relaxed);
    let game_cache_read = llm::LLM_CACHE_READ_TOKENS.load(Ordering::Relaxed);
    let game_cache_create = llm::LLM_CACHE_CREATION_TOKENS.load(Ordering::Relaxed);
    let game_calls = llm::LLM_API_CALLS.load(Ordering::Relaxed);

    // Combined
    let input = draft_input + game_input;
    let output = draft_output + game_output;
    let cache_read = draft_cache_read + game_cache_read;
    let cache_create = draft_cache_create + game_cache_create;
    let calls = draft_calls + game_calls;

    // Sonnet pricing
    let input_cost = (input as f64) * 3.0 / 1_000_000.0;
    let output_cost = (output as f64) * 15.0 / 1_000_000.0;
    let cache_read_cost = (cache_read as f64) * 0.30 / 1_000_000.0;
    let cache_create_cost = (cache_create as f64) * 3.75 / 1_000_000.0;
    let total_cost = input_cost + output_cost + cache_read_cost + cache_create_cost;

    eprintln!("\n=== Token Usage ===");
    eprintln!("  Draft phase:  {} calls, {}in/{}out/{}cache_read/{}cache_write",
        draft_calls, draft_input, draft_output, draft_cache_read, draft_cache_create);
    eprintln!("  Game phase:   {} calls, {}in/{}out/{}cache_read/{}cache_write",
        game_calls, game_input, game_output, game_cache_read, game_cache_create);
    eprintln!("  ---");
    eprintln!("  Total calls:          {}", calls);
    eprintln!("  Input tokens:         {:>8} (${:.4})", input, input_cost);
    eprintln!("  Output tokens:        {:>8} (${:.4})", output, output_cost);
    eprintln!("  Cache read tokens:    {:>8} (${:.4})", cache_read, cache_read_cost);
    eprintln!("  Cache creation tokens:{:>8} (${:.4})", cache_create, cache_create_cost);
    eprintln!("  Estimated cost:       ${:.2}", total_cost);
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

        let response = self.call_llm(&self.conversation.clone());

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

        let response = self.call_llm(&self.deck_conversation.clone());

        self.deck_conversation.push(serde_json::json!({
            "role": "assistant",
            "content": &response
        }));

        response
    }

    fn call_llm(&self, messages: &[serde_json::Value]) -> String {
        match self.provider {
            Provider::Anthropic => self.call_anthropic(messages),
            Provider::Gemini => self.call_gemini(messages),
        }
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
                        record_anthropic_usage(&json["usage"]);
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
                        record_gemini_usage(&json["usageMetadata"]);
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
