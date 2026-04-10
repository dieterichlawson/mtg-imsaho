use std::env;
use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::types::{CardType, Step};
use mtg_engine::view::GameView;
use reqwest::blocking::Client;

use crate::Player;

/// Per-model token usage tracking for LlmPlayer game calls.
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct LlmModelUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
    pub calls: u64,
}

static LLM_MODEL_USAGE: std::sync::LazyLock<Mutex<HashMap<String, LlmModelUsage>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn record_llm_usage(model: &str, input: u64, output: u64, cache_read: u64, cache_create: u64) {
    let mut map = LLM_MODEL_USAGE.lock().unwrap();
    let entry = map.entry(model.to_string()).or_default();
    entry.calls += 1;
    entry.input += input;
    entry.output += output;
    entry.cache_read += cache_read;
    entry.cache_create += cache_create;
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
pub fn get_llm_model_usage() -> HashMap<String, LlmModelUsage> {
    LLM_MODEL_USAGE.lock().unwrap().clone()
}

/// Format a reqwest::Error with kind tags and the underlying source chain.
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

/// Shared game rules and strategy — used by all backends.
const GAME_RULES: &str = r#"## Prompt format

Each prompt you receive has these sections (in order):

**Recent events** (top): a delta log of game events since your last decision — lands played, spells cast, triggers, damage, draws, etc. Use this to understand what changed. Includes both your actions and your opponent's.

**Header line**: `Turn N - <step> (your turn|opp's turn)`. The step is one of: Untap, Upkeep, Draw, Main 1, Begin Combat, Declare Attackers, Declare Blockers, Combat Damage, End Combat, Main 2, End Step, Cleanup.

**Player status**:
```
You: 20hp, 7cards, 33lib, 0gy, 0exile
Opp: 20hp, 7cards, 33lib, 0gy, 0exile
```
Fields: hp=life total, cards=hand size, lib=library size, gy=graveyard count, exile=exile zone count.

**Mana pool** (only if non-empty): `Mana pool: Green:1, Red:2`

**Boards** (only if non-empty):
```
Your board: 2x Forest, 1x Mountain (tapped), Grizzly Bears 2/2
Opp board: 1x Plains, Savannah Lions 2/1 [S]
```
Lands are grouped by name. `(tapped)` or `(N tapped)` shows tap status. Creatures show CURRENT effective P/T including bonuses. Status flags after creatures:
- `[T]` = tapped
- `[S]` = summoning sick (entered this turn, can't attack)
- `[Ndmg]` = N damage marked on it

**Stack** (only if non-empty): `Stack: Lightning Bolt targeting Goblin Piker (opp's)` — shows pending spells/abilities with controller tag and targets.

**Hand**: `Hand: Forest, Grizzly Bears {1}{G} 2/2, Lightning Bolt {R}` — your hand with mana costs and (for creatures) base P/T.

**Graveyards** (only if non-empty): one line per player listing the cards.

**Flashback available** (only if relevant): cards in your graveyard you can cast for their flashback cost.

**Context line**: a `[CONTEXT]` marker showing the current game state:
- `[MAIN PHASE 1]` / `[MAIN PHASE 2]` — your main phases. Cast sorceries, creatures, enchantments, artifacts here. Also play lands here.
- `[BEGIN COMBAT]` — just before declaring attackers. Last chance for instants before combat.
- `[AFTER ATTACKERS DECLARED]` — attackers are declared, blockers haven't been chosen yet. Instant window — cast pump spells on attackers, removal on blockers.
- `[AFTER BLOCKERS DECLARED]` — blockers chosen, before damage. Instant window — cast pump spells, removal, etc.
- `[UPKEEP]`, `[DRAW]`, `[END STEP]` — utility steps. Usually pass unless you have a specific instant to cast (e.g. removing a creature at end of turn so you don't expose your own removal).
- `[OPPONENT'S TURN: <step>]` — it's the opponent's turn and you have priority. You can cast instants and activate abilities.
- `[RESPOND TO <controller>'s <spell>]` — something is on the stack waiting to resolve. You can pass to let it resolve, or respond with an instant/ability (e.g. Counterspell).

**Action list** (final line): numbered options separated by spaces, like `0:Pass 1:Tap Forest 2:Cast Kalonian Tusker (tap 2x Forest) 3:Concede`. Pick one by its index. Targeted spells show their chosen target inline (e.g. `Cast Lightning Bolt → Goblin Piker 2/1`); for spells with multiple choices you'll get a follow-up "Choose targets" prompt.

## Key rules

- **Auto-tap**: When you pick a `Cast [spell]` option, the engine taps the right lands for you automatically. The action label shows which lands will be tapped, e.g. `Cast Doom Blade (tap Swamp, Swamp)`. You almost never need to tap lands manually before casting.
- **Manual tapping**: Useful for floating mana to bluff an instant, using a mana ability with a side effect (e.g. Deranged Assistant mills a card), or overriding the auto-tap to preserve a specific land. Otherwise just pick the Cast option.
- **Mana pools empty between steps**: You can tap lands at any time you have priority, but the mana disappears when the step ends. Only tap if you'll spend the mana in the same step (cast a sorcery/creature in main, or an instant in any step).
- **Spells use the stack**: Your spell goes on the stack and resolves only after both players pass priority. Opponents can respond. The Stack section shows what's pending.
- **Land drops**: One land per turn, only during your main phase.
- **Sorcery speed**: Sorceries, creatures, enchantments, artifacts can only be cast during YOUR main phase with an empty stack.
- **Instant speed**: Instants can be cast anytime you have priority — your turn, opponent's turn, during combat, in response to spells.
- **Summoning sickness**: Creatures with `[S]` can't attack or use tap-abilities the turn they enter. Goes away on your next untap.

## Keyword abilities

Creatures display their keywords after P/T (e.g. `Abbey Griffin 2/2 flying, vigilance`). Combat-relevant keywords:

- **flying**: Only blocked by flying or reach. Huge in combat.
- **reach**: Can block flying (doesn't grant flying).
- **deathtouch**: Any damage it deals to a creature destroys it. A 1/1 deathtouch kills a 10/10.
- **first strike**: Deals damage before non-first-strike creatures. Can kill a blocker before it hits back.
- **double strike**: Deals first strike AND normal damage.
- **lifelink**: Damage dealt = life gained. Changes race math.
- **trample**: Excess damage hits the defending player.
- **vigilance**: Doesn't tap when attacking — can still block.
- **hexproof**: Can't be targeted by opponent's spells/abilities. Don't waste removal on it.
- **defender**: Can't attack.
- **intimidate**: Only blocked by artifact creatures or creatures sharing a color.
- **menace**: Must be blocked by 2+ creatures.
- **haste**: Can attack the turn it enters (ignores summoning sickness).
- **indestructible**: Can't be destroyed by damage or destroy effects.

## Flashback

Cards with flashback can be cast from your graveyard for their flashback cost. After resolving they're exiled. Look for `Flashback <card>` in the action list. The engine auto-taps for flashback costs.

## London mulligan

At the start of the game, before turn 1, you'll be asked two pre-game decisions:

1. **Keep or mulligan** — context `[MULLIGAN DECISION]`. You'll see your seven-card hand numbered with mana costs and P/T. Respond with `{"thoughts": "...", "mull": true|false}` — `true` to mulligan, `false` to keep. This is the London mulligan: you always draw exactly seven cards, but each mulligan you take costs you one card that you'll put on the bottom of your library when you finally keep. House rule: capped at mull-to-4, so after three mulligans you are forced to keep. Mulligan a 0- or 7-lander, or a hand with no plays in the first three turns; keep if you have 2–4 lands and a reasonable curve.
2. **Bottom N cards** — context `[BOTTOM N CARD(S) AFTER MULLIGAN]`. You'll see your seven-card hand numbered 0..6 and must pick exactly N distinct indices to put on the bottom of your library. Respond with `{"thoughts": "...", "bottom_indices": [0, 3, 5]}`. Do not include duplicates or out-of-range indices; the response will be rejected and a fallback used.

## Examples

### Example: main phase, build mana and cast a creature

```
Recent events:
p0 drew a card

Turn 3 - Main 1 (your turn)
You: 20hp, 6cards, 31lib, 0gy, 0exile
Opp: 20hp, 6cards, 32lib, 0gy, 0exile
Your board: 2x Forest
Hand: Forest, Kalonian Tusker {G}{G} 3/3, Kalonian Tusker {G}{G} 3/3, Lightning Bolt {R}
[MAIN PHASE 1]

0:Pass 1:Tap Forest 2:Tap Forest 3:Play Forest 4:Cast Kalonian Tusker (tap 2x Forest) 5:Concede
```
**Pick 4** — auto-tap handles mana, just cast directly. Don't bother with Tap Forest manually.

### Example: utility step, nothing to do

```
Recent events:
Step: Upkeep

Turn 4 - Upkeep (your turn)
You: 20hp, 5cards, 30lib, 0gy, 0exile
Opp: 20hp, 6cards, 32lib, 0gy, 0exile
Your board: 3x Forest, Kalonian Tusker 3/3
Hand: Forest, Lightning Bolt {R}
[UPKEEP]

0:Pass 1:Tap Forest 2:Tap Forest 3:Tap Forest 4:Concede
```
**Pick 0** — no instants you want to cast right now. Tapping a Forest in Upkeep just wastes it (mana pool empties when Upkeep ends).

### Example: combat trick after attackers are declared

```
Recent events:
p0 declared attackers: Grizzly Bears (#27)

Turn 5 - Declare Attackers (your turn)
You: 20hp, 4cards, 28lib, 1gy, 0exile
Opp: 18hp, 5cards, 29lib, 0gy, 0exile
Your board: 2x Forest, Grizzly Bears 2/2 [T]
Opp board: 2x Plains, Savannah Lions 2/1
Hand: Giant Growth {G}
[AFTER ATTACKERS DECLARED]

0:Pass 1:Tap Forest 2:Cast Giant Growth (tap Forest) 3:Concede
```
**Pick 2** — cast Giant Growth on your attacking Bears. After it resolves they're 5/5, so even if Savannah Lions blocks, the Bears survive (5 toughness vs 2 power) and trade up.

### Example: respond to opponent's spell

```
Recent events:
p1 cast Lightning Bolt (#41) targeting Kalonian Tusker (#30)

Turn 5 - Main 1 (opp's turn)
You: 20hp, 5cards, 28lib, 1gy, 0exile
Opp: 18hp, 4cards, 29lib, 1gy, 0exile
Your board: 3x Island, Kalonian Tusker 3/3
Stack: Lightning Bolt targeting Kalonian Tusker (your) (opp's)
Hand: Counterspell {U}{U}, Island
[RESPOND TO p1's Lightning Bolt]

0:Pass 1:Tap Island 2:Tap Island 3:Tap Island 4:Cast Counterspell (tap 2x Island) 5:Concede
```
**Pick 4** — counter the Bolt to save your 3/3. The Tusker would die to 3 damage.

### Example: declare attackers

```
Recent events:
Step: Declare Attackers

Turn 6 - Declare Attackers (your turn)
You: 20hp, 5cards, 28lib, 0gy, 0exile
Opp: 14hp, 5cards, 29lib, 1gy, 0exile
Your board: 3x Forest, Kalonian Tusker 3/3, Kalonian Tusker 3/3
Opp board: 2x Mountain, Goblin Piker 2/1
Choose attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Respond with attacker_indices (list of numbers 0-1), or empty list for none.
```
**Respond `{"attacker_indices": [0, 1]}`** — attack with both 3/3s. Opponent's 2/1 can only block one, so 3 damage gets through and the blocked Tusker survives (3 toughness vs 2 power).

### Example: declare blockers

```
Recent events:
p0 declared attackers: Kalonian Tusker (#30), Kalonian Tusker (#31)

Turn 6 - Declare Blockers (opp's turn)
You: 17hp, 5cards, 27lib, 0gy, 0exile
Opp: 14hp, 4cards, 28lib, 0gy, 0exile
Your board: 3x Mountain, Goblin Piker 2/1, Goblin Piker 2/1
Opp board: 3x Forest (tapped), Kalonian Tusker 3/3 [T], Kalonian Tusker 3/3 [T]
Attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Your blockers: 0:Goblin Piker 2/1 1:Goblin Piker 2/1
For each blocker, respond with the attacker index to block, or -1 for no block.
```
**Respond `{"0": 0, "1": 1}`** — chump-block both Tuskers. Your 2/1s die but you prevent 6 damage. Better than taking 6 to the face when you're at 17.
"#;

/// Backend trait for LLM API communication.
/// Separates provider-specific API mechanics from shared game logic.
trait LlmBackend {
    /// Send a message and get a response. Manages conversation state internally.
    fn send(&mut self, message: &str) -> String;
    /// Send a message with a custom JSON response schema. Returns the parsed JSON.
    /// Default implementation: calls send() and wraps the text in a JSON string.
    fn send_with_schema(&mut self, message: &str, _schema: &serde_json::Value) -> serde_json::Value {
        let text = self.send(message);
        serde_json::Value::String(text)
    }
    /// Initialize with a system prompt (rules + decklists).
    fn init(&mut self, system_prompt: &str);
    /// Resume from a game log recap.
    fn resume(&mut self, recap: &str);
    /// Set thinking level (Gemini only, no-op for others).
    fn set_thinking_level(&mut self, _level: &str) {}
    /// Get the conversation length (for tests).
    fn conversation_len(&self) -> usize { 0 }
    /// Get the system prompt (for tests).
    fn system_prompt(&self) -> &str;
    /// Get the model identifier.
    fn model_name(&self) -> &str;
    /// Return and clear the thinking text from the last API call, if any.
    fn take_thinking(&mut self) -> Option<String> { None }
}

/// Shared response-format intro that comes before GAME_RULES in every system
/// prompt. Sets the role, summarises what the model will see and be asked,
/// and documents every JSON schema it might be expected to return.
const RESPONSE_FORMAT_INTRO: &str = r#"You are playing Magic: The Gathering against an opponent in a one-on-one
Limited (draft) match — each player has a 40-card deck built from a draft pool.
The goal is to reduce your opponent's life total from 20 to 0 by attacking with
creatures and casting damaging spells, while protecting your own life total.

## What you'll be asked

For every decision the game requires, you'll receive a prompt describing the
current game state — recent events, turn and step, both players' life and
hand/library/graveyard counts, the contents of each battlefield, the stack,
your mana pool, your hand, and a context tag (main phase, combat, response
window, mulligan, etc.). The "Prompt format" section below documents every
field in detail. Depending on the context, you'll be asked to:

- Pick one of N legal actions during a turn step (play a land, cast a spell,
  tap mana, pass priority, etc.)
- Declare which of your creatures attack
- Assign blockers to incoming attackers
- Pick targets for a spell or ability
- Decide whether to mulligan, and if so which cards to put on the bottom
- Confirm a concession

## How you respond

You always respond with structured JSON. The exact schema depends on what was
asked, and every schema includes a "thoughts" field — use it to think through
the game state, weigh alternatives, and explain your choice. Thoughts are
private (your opponent does not see them), so be candid about your plan.

The possible schemas are:

1. **Action selection**: `{"thoughts": "...", "action": N}` — pick the 0-indexed action number from the action list.
2. **Declare attackers**: `{"thoughts": "...", "attacker_indices": [0, 1, 2]}` — list of creature indices to attack with (empty list = no attacks).
3. **Declare blockers**: `{"thoughts": "...", "0": 1, "1": -1}` — for each of your blockers (by index), the attacker index it blocks, or -1 for no block.
4. **Choose targets**: `{"thoughts": "...", "target_indices": [0]}` — list of target indices to select for a spell or ability.
5. **Confirm concede**: `{"thoughts": "...", "confirm": true|false}` — confirm or cancel a concession.
6. **Mulligan decision**: `{"thoughts": "...", "mull": true|false}` — London mulligan: keep your opening hand or shuffle and redraw.
7. **Bottom cards after mulligan**: `{"thoughts": "...", "bottom_indices": [0, 3, 5]}` — exactly N distinct 0-indexed hand positions to bottom.

The detailed game rules and prompt format follow.

"#;

const ANTHROPIC_RESPONSE_FORMAT: &str = RESPONSE_FORMAT_INTRO;
const GEMINI_RESPONSE_FORMAT: &str = RESPONSE_FORMAT_INTRO;

/// Anthropic Claude backend using the Messages API with prompt caching.
struct AnthropicBackend {
    client: Client,
    api_key: String,
    model: String,
    system_prompt: String,
    conversation: Vec<serde_json::Value>,
    last_thinking: Option<String>,
}

impl AnthropicBackend {
    fn new(model: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable must be set");
        Self {
            client: Client::new(),
            api_key,
            model: model.to_string(),
            system_prompt: format!("{}{}", ANTHROPIC_RESPONSE_FORMAT, GAME_RULES),
            conversation: Vec::new(),
            last_thinking: None,
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
            if let Some(content) = msgs[idx].get("content").and_then(|c| c.as_str()).map(|s| s.to_string()) {
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
    fn call_api(&mut self, body: &serde_json::Value) -> String {
        const MAX_ATTEMPTS: u32 = 3;
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt));
                std::thread::sleep(delay);
            }

            let started = std::time::Instant::now();
            let response = self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .timeout(std::time::Duration::from_secs(120))
                .json(body)
                .send();
            let elapsed_ms = started.elapsed().as_millis();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        record_anthropic_llm_usage(&self.model, &json);
                        self.last_thinking = None;
                        let mut text_content = String::from("0");
                        if let Some(content) = json["content"].as_array() {
                            for block in content {
                                match block["type"].as_str() {
                                    Some("thinking") => {
                                        if let Some(thinking) = block["thinking"].as_str() {
                                            self.last_thinking = Some(thinking.to_string());
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
                        return text_content;
                    }
                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    let snippet = &text[..text.len().min(200)];
                    if code == 529 || code == 429 {
                        let msg = format!(
                            "Anthropic HTTP {} (attempt {}/{}, {}ms): {}",
                            code, attempt + 1, MAX_ATTEMPTS, elapsed_ms, snippet
                        );
                        eprintln!("{}", msg);
                        crate::game_log::write(file!(), line!(), "API_RETRY", &msg);
                        continue;
                    }
                    let msg = format!(
                        "Anthropic HTTP {} (attempt {}/{}, {}ms): {}",
                        code, attempt + 1, MAX_ATTEMPTS, elapsed_ms, snippet
                    );
                    eprintln!("{}", msg);
                    crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                    return "0".to_string();
                }
                Err(e) => {
                    let msg = format!(
                        "Anthropic request failed (attempt {}/{}, {}ms): {}",
                        attempt + 1, MAX_ATTEMPTS, elapsed_ms, format_reqwest_error(&e)
                    );
                    eprintln!("{}", msg);
                    crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                    continue;
                }
            }
        }
        "0".to_string()
    }

    fn call_with_messages(&mut self, messages: &[serde_json::Value]) -> String {
        let (system, msgs) = self.prepare_request(messages);
        let body = serde_json::json!({
            "model": self.model,
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
        self.call_api(&body)
    }

    /// Transform a JSON schema to be Anthropic-compatible:
    /// - Add "additionalProperties": false to all objects
    /// - Strip unsupported numeric constraints (minimum, maximum, multipleOf)
    /// - Strip "thoughts" field — reasoning happens in thinking blocks
    fn sanitize_schema(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, val) in map {
                    // Strip unsupported numeric constraints.
                    if key == "minimum" || key == "maximum" || key == "multipleOf" {
                        continue;
                    }
                    new_map.insert(key.clone(), Self::sanitize_schema(val));
                }
                // Add additionalProperties: false to object types.
                if new_map.get("type").and_then(|t| t.as_str()) == Some("object") {
                    new_map.entry("additionalProperties".to_string())
                        .or_insert(serde_json::Value::Bool(false));
                    // Strip "thoughts" — thinking blocks provide reasoning.
                    if let Some(props) = new_map.get_mut("properties").and_then(|p| p.as_object_mut()) {
                        props.remove("thoughts");
                    }
                    if let Some(req) = new_map.get_mut("required").and_then(|r| r.as_array_mut()) {
                        req.retain(|v| v.as_str() != Some("thoughts"));
                    }
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::sanitize_schema).collect())
            }
            other => other.clone(),
        }
    }

    fn call_with_messages_structured(&mut self, messages: &[serde_json::Value], schema: &serde_json::Value) -> serde_json::Value {
        let (system, msgs) = self.prepare_request(messages);
        let sanitized = Self::sanitize_schema(schema);
        let body = serde_json::json!({
            "model": self.model,
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
        let text = self.call_api(&body);
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({}))
    }
}

impl LlmBackend for AnthropicBackend {
    fn send(&mut self, message: &str) -> String {
        self.conversation.push(serde_json::json!({"role": "user", "content": message}));
        let result = self.call_with_messages(&self.conversation.clone());
        self.conversation.push(serde_json::json!({"role": "assistant", "content": &result}));
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
        result
    }

    fn init(&mut self, deck_info: &str) {
        self.system_prompt = format!("{}{}{}", ANTHROPIC_RESPONSE_FORMAT, GAME_RULES, deck_info);
        self.conversation.clear();
    }

    fn resume(&mut self, recap: &str) {
        self.conversation.push(serde_json::json!({"role": "user", "content": recap}));
        self.conversation.push(serde_json::json!({
            "role": "assistant",
            "content": "Understood. I've reviewed the game history and I'm ready to continue playing."
        }));
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
            system_prompt: format!("{}{}", GEMINI_RESPONSE_FORMAT, GAME_RULES),
            interaction_id: None,
            last_thinking: None,
        }
    }

    /// Core interactions API call. Sends a message with a custom JSON schema
    /// and returns the parsed JSON response. Handles retries, rate limits, etc.
    fn call_interactions_structured(&mut self, user_message: &str, schema: &serde_json::Value) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": &self.model,
            "input": user_message,
            "response_mime_type": "application/json",
            "response_format": schema,
        });

        if let Some(ref level) = self.thinking_level {
            body["generation_config"] = serde_json::json!({"thinking_level": level});
        }

        // Only chain if we have a non-empty previous interaction ID.
        if let Some(ref prev_id) = self.interaction_id.as_ref().filter(|s| !s.is_empty()) {
            body["previous_interaction_id"] = serde_json::json!(prev_id);
        } else {
            body["system_instruction"] = serde_json::json!(&self.system_prompt);
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/interactions?key={}",
            self.api_key
        );

        const MAX_ATTEMPTS: u32 = 6;
        let mut fresh_retry = false;
        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt.min(4)));
                std::thread::sleep(delay);
            }

            let started = std::time::Instant::now();
            let response = self.client
                .post(&url)
                .header("content-type", "application/json")
                .timeout(std::time::Duration::from_secs(120))
                .json(&body)
                .send();
            let elapsed_ms = started.elapsed().as_millis();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        record_gemini_llm_usage(&self.model, &json["usage"]);

                        self.interaction_id = json["id"].as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());

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
                            self.last_thinking = parsed["thoughts"].as_str().map(|s| s.to_string());
                            return parsed;
                        }

                        let msg = format!("Gemini returned non-JSON response: {:?}", &output_text[..output_text.len().min(100)]);
                        eprintln!("WARN: {}", msg);
                        crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                        return serde_json::json!({});
                    }

                    let code = resp.status().as_u16();
                    let text = resp.text().unwrap_or_default();
                    let snippet = &text[..text.len().min(200)];
                    if code == 429 || code == 503 || code == 529 {
                        let msg = format!(
                            "Gemini HTTP {} (attempt {}/{}, {}ms): {}",
                            code, attempt + 1, MAX_ATTEMPTS, elapsed_ms, snippet
                        );
                        eprintln!("{}", msg);
                        crate::game_log::write(file!(), line!(), "API_RETRY", &msg);
                        continue;
                    }
                    // If the interaction ID is invalid, fall back to a fresh conversation.
                    if code == 400 && text.contains("previous_interaction_id") && !fresh_retry {
                        let msg = "Invalid interaction ID, falling back to fresh conversation";
                        eprintln!("WARN: {}", msg);
                        crate::game_log::write(file!(), line!(), "API_WARN", msg);
                        body.as_object_mut().unwrap().remove("previous_interaction_id");
                        body["system_instruction"] = serde_json::json!(&self.system_prompt);
                        self.interaction_id = None;
                        fresh_retry = true;
                        continue;
                    }
                    // Fatal config errors — abort loudly so we don't silently produce garbage.
                    if code == 400 && (text.contains("thinking level") || text.contains("not a supported")) {
                        let msg = format!("Gemini config error: {}", &text[..text.len().min(300)]);
                        eprintln!("FATAL: {}", msg);
                        crate::game_log::write(file!(), line!(), "API_FATAL", &msg);
                        std::process::exit(1);
                    }
                    let msg = format!(
                        "Gemini HTTP {} (attempt {}/{}, {}ms): {}",
                        code, attempt + 1, MAX_ATTEMPTS, elapsed_ms, snippet
                    );
                    eprintln!("{}", msg);
                    crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                    return serde_json::json!({});
                }
                Err(e) => {
                    let msg = format!(
                        "Gemini request failed (attempt {}/{}, {}ms): {}",
                        attempt + 1, MAX_ATTEMPTS, elapsed_ms, format_reqwest_error(&e)
                    );
                    eprintln!("{}", msg);
                    crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
                    continue;
                }
            }
        }
        let msg = format!("Gemini API exhausted all {} retries", MAX_ATTEMPTS);
        eprintln!("WARN: {}", msg);
        crate::game_log::write(file!(), line!(), "API_ERROR", &msg);
        serde_json::json!({})
    }

    /// Convenience wrapper: sends with the default action schema, returns just
    /// the action number as a string (backward-compatible with existing callers).
    fn call_interactions(&mut self, user_message: &str) -> String {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Brief reasoning for this action"},
                "action": {"type": "integer", "minimum": 0}
            },
            "required": ["thoughts", "action"]
        });
        let parsed = self.call_interactions_structured(user_message, &schema);
        parsed["action"].as_u64().map(|n| n.to_string()).unwrap_or_else(|| "0".to_string())
    }
}

impl LlmBackend for GeminiBackend {
    fn send(&mut self, message: &str) -> String {
        self.call_interactions(message)
    }

    fn send_with_schema(&mut self, message: &str, schema: &serde_json::Value) -> serde_json::Value {
        self.call_interactions_structured(message, schema)
    }

    fn init(&mut self, deck_info: &str) {
        self.system_prompt = format!("{}{}{}", GEMINI_RESPONSE_FORMAT, GAME_RULES, deck_info);
        self.interaction_id = None;
    }

    fn resume(&mut self, recap: &str) {
        self.call_interactions(recap);
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

pub struct LlmPlayer {
    name: String,
    /// Index into the game log — tracks which log entries have been sent.
    last_log_index: usize,
    /// Provider-specific API backend.
    backend: Box<dyn LlmBackend>,
}

impl LlmPlayer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            backend: Box::new(AnthropicBackend::new("claude-sonnet-4-6")),
        }
    }

    pub fn new_gemini(name: &str) -> Self {
        Self {
            name: name.to_string(),
            last_log_index: 0,
            backend: Box::new(GeminiBackend::new("gemini-2.5-flash")),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        // Recreate the backend with the new model name.
        // We check the current backend type by trying to downcast.
        if model.contains("gemini") {
            self.backend = Box::new(GeminiBackend::new(model));
        } else {
            self.backend = Box::new(AnthropicBackend::new(model));
        }
        self
    }

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

    /// Initialize the conversation with decklists and oracle text.
    /// Call this once before the game starts.
    pub fn init_conversation(
        &mut self,
        your_deck: &[(String, u32)],
        opp_deck: &[(String, u32)],
        registry: &mtg_engine::cards::CardRegistry,
    ) {
        let mut deck_info = String::new();
        deck_info.push_str("\n\n## Your decklist\n\n");
        deck_info.push_str(&Self::format_decklist(your_deck, registry));
        deck_info.push_str("\n\n## Opponent's decklist\n\n");
        deck_info.push_str(&Self::format_decklist(opp_deck, registry));
        self.backend.init(&deck_info);
        self.last_log_index = 0;
        self.log("SYSTEM", self.backend.system_prompt());
    }

    /// Resume conversation from an existing game state.
    /// Sends the full game log as a catch-up message so the AI has context
    /// about what happened before the reload.
    pub fn resume_from_log(&mut self, game_log: &[String]) {
        if game_log.is_empty() {
            return;
        }
        // Build a catch-up message with the full game history.
        let mut recap = String::from("Game resumed. Here is the complete game log so far:\n\n");
        for entry in game_log {
            recap.push_str(entry);
            recap.push('\n');
        }
        recap.push_str("\nThe game continues from this point. You will be prompted for your next action.");

        self.backend.resume(&recap);
        // Set log index to current length so we don't re-send these entries.
        self.last_log_index = game_log.len();
        self.log("RESUME", &format!("Resumed with {} log entries", game_log.len()));
    }

    fn format_decklist(entries: &[(String, u32)], registry: &mtg_engine::cards::CardRegistry) -> String {
        let mut s = String::new();
        let mut seen = std::collections::HashSet::new();
        for (name, count) in entries {
            s.push_str(&format!("{}x {}\n", count, name));
            if !seen.contains(name) {
                seen.insert(name.clone());
                if let Some(id) = registry.get_id_by_name(name) {
                    if let Some(data) = registry.card_data(id) {
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
                        s.push_str(&format!("  {}{} {}{}{}\n", name, cost, types.join(" "), subtypes, pt));
                        if !data.oracle_text.is_empty() {
                            s.push_str(&format!("  {}\n", data.oracle_text.replace('\n', "\n  ")));
                        }
                    }
                }
            }
        }
        s
    }

    // ── Test helpers ──────────────────────────────────────────────

    /// Expose format_decklist for testing.
    pub fn format_decklist_for_test(entries: &[(String, u32)], registry: &mtg_engine::cards::CardRegistry) -> String {
        Self::format_decklist(entries, registry)
    }

    /// Expose short_effect_summary for testing.
    pub fn short_effect_summary_for_test(oracle_text: &str) -> String {
        Self::short_effect_summary(oracle_text)
    }

    /// Expose system prompt for testing.
    pub fn system_prompt_for_test(&self) -> &str {
        self.backend.system_prompt()
    }

    /// Expose conversation length for testing.
    pub fn conversation_len_for_test(&self) -> usize {
        self.backend.conversation_len()
    }

    /// Expose last_log_index for testing.
    pub fn last_log_index_for_test(&self) -> usize {
        self.last_log_index
    }

    /// Get the model identifier (e.g. "claude-sonnet-4-6", "gemini-2.5-flash").
    pub fn model_name(&self) -> &str {
        self.backend.model_name()
    }

    fn log(&self, label: &str, content: &str) {
        let full_label = format!("{} [{}]", label, self.name);
        crate::game_log::write(file!(), line!(), &full_label, content);
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

    /// Compact game state — much shorter than the CLI version.
    fn format_state_compact(view: &GameView, header_override: Option<&str>) -> String {
        let mut s = String::new();

        // Header line. During pre-game phases (mulligan / bottoming) the
        // engine still reports turn=1 step=Untap, which is misleading; the
        // caller passes a phase label override in that case.
        if let Some(h) = header_override {
            s.push_str(h);
            s.push('\n');
        } else {
            let step_name = match view.step {
                Step::PrecombatMain => "Main 1",
                Step::PostcombatMain => "Main 2",
                Step::BeginCombat => "Begin Combat",
                Step::DeclareAttackers => "Declare Attackers",
                Step::DeclareBlockers => "Declare Blockers",
                Step::CombatDamage => "Combat Damage",
                Step::EndCombat => "End Combat",
                Step::Upkeep => "Upkeep",
                Step::Draw => "Draw",
                Step::EndStep => "End Step",
                Step::Untap => "Untap",
                Step::Cleanup => "Cleanup",
            };
            let whose_turn = if view.active_player == view.you { "your turn" } else { "opp's turn" };
            s.push_str(&format!("Turn {} - {} ({})\n", view.turn_number, step_name, whose_turn));
        }

        // Zone counts
        let your_gy_count: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards.len()).sum();
        let your_exile_count = view.exile.iter().filter(|c| c.owner == view.you).count();
        let opp_gy_count: usize = view.graveyards.iter()
            .filter(|(pid, _)| *pid != view.you)
            .map(|(_, cards)| cards.len()).sum();
        let opp_exile_count = view.exile.iter().filter(|c| c.owner != view.you).count();

        s.push_str(&format!("You: {}hp, {}cards, {}lib, {}gy, {}exile\n",
            view.your_life, view.your_hand.len(), view.your_library_size,
            your_gy_count, your_exile_count));
        for opp in &view.opponents {
            s.push_str(&format!("Opp: {}hp, {}cards, {}lib, {}gy, {}exile\n",
                opp.life, opp.hand_size, opp.library_size,
                opp_gy_count, opp_exile_count));
        }

        if !view.your_mana_pool.is_empty() {
            let pool_parts: Vec<String> = view.your_mana_pool.mana.iter()
                .filter(|(_, &v)| v > 0)
                .map(|(t, v)| format!("{:?}:{}", t, v))
                .collect();
            s.push_str(&format!("Mana pool: {}\n", pool_parts.join(", ")));
        }

        // Battlefield — compact
        let your_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller == view.you).collect();
        let opp_perms: Vec<_> = view.battlefield.iter().filter(|p| p.controller != view.you).collect();
        let all_perms: Vec<_> = view.battlefield.iter().collect();

        if !your_perms.is_empty() {
            s.push_str("Your board: ");
            s.push_str(&Self::format_perms_compact(&your_perms, &all_perms));
            s.push('\n');
        }
        if !opp_perms.is_empty() {
            s.push_str("Opp board: ");
            s.push_str(&Self::format_perms_compact(&opp_perms, &all_perms));
            s.push('\n');
        }

        // Stack
        if !view.stack.is_empty() {
            s.push_str("Stack: ");
            let items: Vec<String> = view.stack.iter()
                .map(|i| {
                    let who = if i.controller == view.you { "your" } else { "opp's" };
                    let targets_str = if i.targets.is_empty() {
                        String::new()
                    } else {
                        let target_names: Vec<String> = i.targets.iter()
                            .map(|t| match t {
                                mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                                mtg_engine::actions::Target::Player(pid) => {
                                    if *pid == view.you { "you".into() } else { "opponent".into() }
                                }
                            })
                            .collect();
                        format!(" targeting {}", target_names.join(", "))
                    };
                    format!("{}{} ({})", i.name, targets_str, who)
                })
                .collect();
            s.push_str(&items.join(", "));
            s.push('\n');
        }

        // Hand
        if !view.your_hand.is_empty() {
            s.push_str("Hand: ");
            let cards: Vec<String> = view.your_hand.iter()
                .map(|c| {
                    let cost = c.cost.as_ref().map(|co| format!(" {}", co)).unwrap_or_default();
                    let pt = match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!(" {}/{}", p, t),
                        _ => String::new(),
                    };
                    format!("{}{}{}", c.name, cost, pt)
                })
                .collect();
            s.push_str(&cards.join(", "));
            s.push('\n');
        }

        // Graveyard contents (both players)
        for (pid, cards) in &view.graveyards {
            if !cards.is_empty() {
                let whose = if *pid == view.you { "Your" } else { "Opp" };
                let names: Vec<String> = cards.iter().map(|c| {
                    match (c.power, c.toughness) {
                        (Some(p), Some(t)) => format!("{} {}/{}", c.name, p, t),
                        _ => c.name.clone(),
                    }
                }).collect();
                s.push_str(&format!("{} graveyard: {}\n", whose, names.join(", ")));
            }
        }

        // Show flashback-eligible cards in your graveyard.
        let your_gy = view.graveyards.iter()
            .find(|(pid, _)| *pid == view.you)
            .map(|(_, cards)| cards);
        if let Some(gy_cards) = your_gy {
            let fb_cards: Vec<String> = gy_cards.iter()
                .filter(|c| c.flashback_cost.is_some())
                .map(|c| {
                    let fb = c.flashback_cost.as_ref().unwrap();
                    format!("{} (flashback {})", c.name, fb)
                })
                .collect();
            if !fb_cards.is_empty() {
                s.push_str(&format!("Flashback available: {}\n", fb_cards.join(", ")));
            }
        }

        s
    }

    /// Compact a card's oracle_text into a short inline effect summary for the
    /// board display. Drops the leading "Enchant <type>" targeting line (not
    /// useful once the aura is attached), strips reminder text in parentheses,
    /// collapses whitespace, and joins remaining lines with "; ". Returns an
    /// empty string for cards with no oracle text.
    fn short_effect_summary(oracle_text: &str) -> String {
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
        // Hard cap so a single permanent can't blow up the board line.
        const MAX: usize = 200;
        if joined.chars().count() > MAX {
            let mut out: String = joined.chars().take(MAX - 3).collect();
            out.push_str("...");
            out
        } else {
            joined
        }
    }

    fn format_perms_compact(perms: &[&mtg_engine::view::PermanentView], all_perms: &[&mtg_engine::view::PermanentView]) -> String {
        // Group lands by name with tapped count.
        let lands: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Land)).collect();
        let creatures: Vec<_> = perms.iter().filter(|p| p.card_types.contains(&CardType::Creature)).collect();
        let other: Vec<_> = perms.iter().filter(|p|
            !p.card_types.contains(&CardType::Land) && !p.card_types.contains(&CardType::Creature)
        ).collect();

        let mut parts = Vec::new();

        if !lands.is_empty() {
            let mut land_groups: Vec<(String, usize, usize)> = Vec::new();
            for land in &lands {
                if let Some(entry) = land_groups.iter_mut().find(|(n, _, _)| *n == land.name) {
                    if land.tapped { entry.2 += 1; } else { entry.1 += 1; }
                } else {
                    let (u, t) = if land.tapped { (0, 1) } else { (1, 0) };
                    land_groups.push((land.name.clone(), u, t));
                }
            }
            for (name, untapped, tapped) in &land_groups {
                let total = untapped + tapped;
                if *tapped == 0 {
                    parts.push(format!("{}x {}", total, name));
                } else if *untapped == 0 {
                    parts.push(format!("{}x {} (tapped)", total, name));
                } else {
                    parts.push(format!("{}x {} ({} tapped)", total, name, tapped));
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
            let kw = Self::format_keywords(&c.keywords);
            let kw_str = if kw.is_empty() { String::new() } else { format!(" {}", kw) };
            let mut flag_parts: Vec<String> = Vec::new();
            if c.tapped { flag_parts.push("T".into()); }
            if c.summoning_sick { flag_parts.push("S".into()); }
            if c.damage_marked > 0 { flag_parts.push(format!("{}dmg", c.damage_marked)); }
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
            parts.push(format!("{} {}/{}{}{}{}", c.name, power, toughness, kw_str, flags_str, auras));
        }

        // Show non-aura other permanents. For unattached equipment include
        // the short effect summary; planeswalkers surface their loyalty
        // counter count via format_counters.
        for o in &other {
            if o.attached_to.is_some() { continue; } // skip auras, shown with creature
            let mut flag_parts: Vec<String> = Vec::new();
            if o.tapped { flag_parts.push("T".into()); }
            if let Some(suffix) = Self::format_counters(&o.counters) {
                flag_parts.push(suffix);
            }
            let flags_str = if flag_parts.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flag_parts.join(","))
            };
            let desc = Self::short_effect_summary(&o.oracle_text);
            if desc.is_empty() {
                parts.push(format!("{}{}", o.name, flags_str));
            } else {
                parts.push(format!("{}{} ({})", o.name, flags_str, desc));
            }
        }

        parts.join(", ")
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
            if n > 0 { bits.push(format!("+1+1x{}", n)); }
        }
        if let Some(&n) = counters.get(&CounterType::MinusOneMinusOne) {
            if n > 0 { bits.push(format!("-1-1x{}", n)); }
        }
        if let Some(&n) = counters.get(&CounterType::Loyalty) {
            if n > 0 { bits.push(format!("LOYx{}", n)); }
        }
        if bits.is_empty() { None } else { Some(bits.join(",")) }
    }

    /// Format a single non-CastSpell action for the collapsed display.
    fn format_single_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass".into(),
            Action::PlayLand { object_id } => format!("Play {}", Self::obj_name(view, *object_id)),
            Action::ActivateManaAbility { object_id, .. } => format!("Tap {}", Self::obj_name(view, *object_id)),
            Action::ActivateAbility { object_id, .. } => format!("Activate {}", Self::obj_name(view, *object_id)),
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
                        }
                    }
                    ResolvedChoice::ChosenTarget(None) => "Decline".into(),
                    ResolvedChoice::ChosenCard(id) => Self::obj_name(view, *id),
                    ResolvedChoice::ChosenIndex(i) => format!("Option {}", i),
                    ResolvedChoice::ChosenSubset(ids) => {
                        let names: Vec<String> = ids.iter()
                            .map(|id| Self::obj_name(view, *id))
                            .collect();
                        format!("Pile 1: [{}]", if names.is_empty() { "empty".into() } else { names.join(", ") })
                    }
                }
            }
            other => format!("{}", other),
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

        match &spell.target_spec {
            CastTargetSpec::NoTargets => {
                if let Some(a) = pick_expanded(&[]) { return a; }
                Action::CastSpell { object_id: spell.object_id, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
            }
            CastTargetSpec::SingleTarget(options) => {
                if options.len() == 1 {
                    if let Some(a) = pick_expanded(std::slice::from_ref(&options[0])) { return a; }
                    return Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() };
                }
                let target = self.prompt_target_selection(view, &format!("{}: select a target", spell.name), options);
                if let Some(a) = pick_expanded(std::slice::from_ref(&target)) { return a; }
                Action::CastSpell { object_id: spell.object_id, targets: vec![target], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
            }
            CastTargetSpec::TwoTargets(options1, options2) => {
                let t1 = self.prompt_target_selection(view, &format!("{}: select first of two targets", spell.name), options1);
                let remaining: Vec<_> = options2.iter().filter(|t| **t != t1).cloned().collect();
                if remaining.is_empty() {
                    // Fallback: find any matching expanded action
                    return self.fallback_to_expanded(spell.object_id, legal_actions);
                }
                let t2 = self.prompt_target_selection(view, &format!("{}: select second of two targets", spell.name), &remaining);
                Action::CastSpell { object_id: spell.object_id, targets: vec![t1, t2], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
            }
            CastTargetSpec::UpToTargets { max, options } => {
                // For the LLM, present all options and ask to pick numbers.
                let target_list: String = options.iter().enumerate()
                    .map(|(i, t)| {
                        let desc = match t {
                            Target::Object(id) => Self::obj_name(view, *id),
                            Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                        };
                        format!("{}:{}", i, desc)
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let prompt = format!(
                    "{}: select up to {} targets (you may choose fewer):\n{}\nRespond with target_indices (list of numbers 0-{}).",
                    spell.name, max, target_list, options.len() - 1
                );
                self.log("TARGETS", &prompt);

                let valid_indices: Vec<serde_json::Value> = (0..options.len())
                    .map(|i| serde_json::json!(i))
                    .collect();
                let schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "thoughts": {"type": "string", "description": "Brief reasoning about target selection"},
                        "target_indices": {
                            "type": "array",
                            "items": {"type": "integer", "enum": valid_indices},
                            "maxItems": *max,
                            "description": format!("Indices of chosen targets (each in 0..{}, up to {} entries)", options.len() - 1, max)
                        }
                    },
                    "required": ["thoughts", "target_indices"]
                });

                let response = self.send_message_structured(&prompt, &schema);
                self.log("TARGET-RESPONSE", &response.to_string());

                let chosen: Vec<Target> = response["target_indices"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .filter(|&i| i < options.len())
                        .take(*max)
                        .map(|i| options[i].clone())
                        .collect())
                    .unwrap_or_default();

                if chosen.is_empty() {
                    // Pick at least one — use the first option.
                    Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
                } else {
                    Action::CastSpell { object_id: spell.object_id, targets: chosen, sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
                }
            }
        }
    }

    /// Choose targets for an activated ability, prompting the LLM if needed.
    fn choose_ability_targets(&mut self, view: &GameView, ab: &mtg_engine::actions::ActivatableAbility, legal_actions: &[Action]) -> Action {
        if ab.target_options.is_empty() {
            // Untargeted ability — find the matching action directly
            return legal_actions.iter()
                .find(|a| matches!(a, Action::ActivateAbility { object_id, ability_index, .. }
                    if *object_id == ab.object_id && *ability_index == ab.ability_index))
                .cloned()
                .unwrap_or(Action::PassPriority);
        }

        if ab.target_options.len() == 1 {
            // Single valid target — no need to prompt
            return Action::ActivateAbility {
                object_id: ab.object_id,
                ability_index: ab.ability_index,
                targets: vec![ab.target_options[0].clone()],
            };
        }

        // Multiple targets — prompt for selection
        let target = self.prompt_target_selection(
            view,
            &format!("{}: select target for {}", ab.name, ab.description),
            &ab.target_options,
        );
        Action::ActivateAbility {
            object_id: ab.object_id,
            ability_index: ab.ability_index,
            targets: vec![target],
        }
    }

    /// Make a second API call to select one target from a list.
    fn prompt_target_selection(&mut self, view: &GameView, spell_name: &str, options: &[mtg_engine::actions::Target]) -> mtg_engine::actions::Target {
        assert!(!options.is_empty(), "prompt_target_selection called with no options for {}", spell_name);
        let target_list: String = options.iter().enumerate()
            .map(|(i, t)| {
                let desc = match t {
                    mtg_engine::actions::Target::Object(id) => Self::obj_name(view, *id),
                    mtg_engine::actions::Target::Player(pid) => if *pid == view.you { "you".into() } else { "opponent".into() },
                };
                format!("{}:{}", i, desc)
            })
            .collect::<Vec<_>>()
            .join(" ");
        let prompt = format!("{}:\n{}", spell_name, target_list);
        self.log("TARGETS", &prompt);
        let idx = self.pick_action_index(&prompt, options.len(), &[]);
        options[idx.min(options.len() - 1)].clone()
    }

    /// Fallback: find the first matching expanded action for this spell.
    fn fallback_to_expanded(&self, object_id: ObjectId, legal_actions: &[Action]) -> Action {
        legal_actions.iter()
            .find(|a| matches!(a, Action::CastSpell { object_id: oid, .. } if *oid == object_id))
            .cloned()
            .unwrap_or(Action::PassPriority)
    }

    /// Format a tap plan as a compact string like "2x Plains, Hinterland Harbor".
    fn format_tap_plan(view: &GameView, tap_plan: &[(ObjectId, usize)]) -> String {
        if tap_plan.is_empty() { return String::new(); }
        // Collect names, count duplicates.
        let mut name_counts: Vec<(String, usize)> = Vec::new();
        for &(source_id, _) in tap_plan {
            let name = Self::obj_name(view, source_id);
            if let Some(entry) = name_counts.iter_mut().find(|(n, _)| *n == name) {
                entry.1 += 1;
            } else {
                name_counts.push((name, 1));
            }
        }
        name_counts.iter()
            .map(|(name, count)| {
                if *count > 1 { format!("{}x {}", count, name) } else { name.clone() }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn obj_name(view: &GameView, id: ObjectId) -> String {
        if let Some(p) = view.battlefield.iter().find(|p| p.object_id == id) {
            let is_land = p.card_types.iter().all(|t| matches!(t, mtg_engine::types::CardType::Land));
            if !is_land {
                let owner = if p.controller == view.you { "your" } else { "opponent's" };
                return format!("{} ({})", p.name, owner);
            }
            return p.name.clone();
        }
        view.your_hand.iter()
            .find(|c| c.object_id == id)
            .map(|c| c.name.clone())
            .or_else(|| view.stack.iter()
                .find(|s| s.object_id == id)
                .map(|s| s.name.clone()))
            .or_else(|| view.graveyards.iter()
                .flat_map(|(_, cards)| cards.iter())
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.exile.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.your_library_cards.iter()
                .find(|c| c.object_id == id)
                .map(|c| c.name.clone()))
            .or_else(|| view.revealed_names.get(&id).cloned())
            .unwrap_or_else(|| format!("{}", id))
    }

    /// Log thinking from the last backend call, if any.
    fn log_thinking(&mut self) {
        if let Some(thinking) = self.backend.take_thinking() {
            self.log("THOUGHT", &thinking);
        }
    }

    /// Send a message with a custom JSON response schema, returning parsed JSON.
    fn send_message_structured(&mut self, user_message: &str, schema: &serde_json::Value) -> serde_json::Value {
        self.log("PROMPT", user_message);
        let result = self.backend.send_with_schema(user_message, schema);
        self.log_thinking();
        self.log("RESPONSE", &result.to_string());
        result
    }

    /// Build a prompt that includes new log entries + board state + the action prompt.
    fn build_prompt(&mut self, view: &GameView, action_prompt: &str) -> String {
        self.build_prompt_with_header(view, action_prompt, None)
    }

    fn build_prompt_with_header(
        &mut self,
        view: &GameView,
        action_prompt: &str,
        header_override: Option<&str>,
    ) -> String {
        // Use display_log (Info level and above) to skip debug noise like
        // "passes priority" and "Step: Draw" entries that add no information.
        let new_logs: Vec<String> = view.display_log.iter()
            .skip(self.last_log_index)
            .cloned()
            .collect();
        self.last_log_index = view.display_log.len();

        let mut prompt = String::new();
        if !new_logs.is_empty() {
            prompt.push_str("Recent events:\n");
            for entry in &new_logs {
                prompt.push_str(entry);
                prompt.push('\n');
            }
            prompt.push('\n');
        }
        prompt.push_str(&Self::format_state_compact(view, header_override));
        prompt.push_str(action_prompt);
        prompt
    }

    /// Confirm concede via structured output. Returns true if the AI confirms.
    fn confirm_concede(&mut self) -> bool {
        self.log("CONCEDE-CHECK", "AI chose Concede, confirming...");

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Brief reasoning about whether to concede"},
                "confirm": {"type": "boolean", "description": "true to concede, false to cancel"}
            },
            "required": ["thoughts", "confirm"]
        });

        let response = self.send_message_structured(
            "You chose to CONCEDE the game. Are you sure? Respond with confirm: true to concede, or false to cancel.",
            &schema
        );
        let confirmed = response["confirm"].as_bool().unwrap_or(false);
        if !confirmed {
            self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
        } else {
            self.log("CONCEDE-CHECK", "Concede confirmed");
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
                "thoughts": {"type": "string", "description": "Brief reasoning for this choice"},
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
    fn pick_action_index(&mut self, prompt: &str, max: usize, actions: &[Action]) -> usize {
        assert!(max > 0, "pick_action_index requires at least one option");
        let schema = Self::enum_action_schema(max, "action", "Index of the chosen action");
        let response = self.send_message_structured(prompt, &schema);
        let idx = response["action"].as_u64().map(|n| n as usize)
            .filter(|n| *n < max)
            .unwrap_or_else(|| {
                self.log("MALFORMED", &format!("response missing valid 'action' field ({}), defaulting to 0", response));
                0
            });
        if matches!(actions.get(idx), Some(Action::Concede)) {
            if !self.confirm_concede() {
                return 0;
            }
        }
        self.log("CHOSE", &format!("action {}", idx));
        idx
    }
}

impl Player for LlmPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let legal_actions = &legal.actions;

        // London mulligan keep/mull decision.
        if legal_actions.iter().any(|a| matches!(a, Action::MulliganKeep)) {
            return self.choose_mulligan(view, legal_actions);
        }
        // London mulligan bottoming decision.
        if matches!(legal_actions.first(), Some(Action::BottomCards { .. })) {
            return self.choose_mulligan_bottom(view, legal_actions);
        }

        // Auto-pass when there's nothing interesting to do.
        if Self::should_auto_pass(view, legal_actions) {
            self.log("AUTO-PASS", &format!("Step: {:?}, active: p#{}", view.step, view.active_player.0));
            return Action::PassPriority;
        }

        // Build collapsed display: non-CastSpell/ActivateAbility actions + one per
        // castable spell + one per activatable ability.
        let mut display_labels = Vec::new();
        enum DisplayEntry {
            Direct(usize),   // index into legal_actions
            Cast(usize),     // index into legal.castable_spells
            Ability(usize),  // index into legal.activatable_abilities
        }
        let mut display_entries: Vec<DisplayEntry> = Vec::new();
        let mut seen_spell_objects: Vec<mtg_engine::ids::ObjectId> = Vec::new();
        let mut seen_ability_keys: Vec<(mtg_engine::ids::ObjectId, usize)> = Vec::new();

        let mut seen_cast_labels: Vec<String> = Vec::new();
        for (i, action) in legal_actions.iter().enumerate() {
            match action {
                Action::CastSpell { object_id, .. } => {
                    if !seen_spell_objects.contains(object_id) {
                        if let Some(cs_idx) = legal.castable_spells.iter()
                            .position(|cs| cs.object_id == *object_id)
                        {
                            seen_spell_objects.push(*object_id);
                            let cs = &legal.castable_spells[cs_idx];
                            let verb = if cs.is_flashback { "Flashback" } else { "Cast" };
                            let tap_str = Self::format_tap_plan(view, &cs.tap_plan);
                            // For ExileXFromGraveyard spells (Harvest Pyre), show the
                            // effective X derived from the current graveyard size and
                            // the resulting damage, so the LLM doesn't waste the spell
                            // on an empty graveyard.
                            let x_suffix = cs.exile_x_from_gy_max
                                .map(|n| format!(" X={} ({} damage)", n, n))
                                .unwrap_or_default();
                            let label = if tap_str.is_empty() {
                                format!("{} {}{}", verb, cs.name, x_suffix)
                            } else {
                                format!("{} {}{} (tap {})", verb, cs.name, x_suffix, tap_str)
                            };
                            // Deduplicate identical cast labels (e.g. two copies of same spell).
                            if seen_cast_labels.contains(&label) { continue; }
                            seen_cast_labels.push(label.clone());
                            display_labels.push(label);
                            display_entries.push(DisplayEntry::Cast(cs_idx));
                        }
                    }
                }
                Action::ActivateAbility { object_id, ability_index, .. } => {
                    let key = (*object_id, *ability_index);
                    if !seen_ability_keys.contains(&key) {
                        if let Some(ab_idx) = legal.activatable_abilities.iter()
                            .position(|ab| ab.object_id == *object_id && ab.ability_index == *ability_index)
                        {
                            seen_ability_keys.push(key);
                            let ab = &legal.activatable_abilities[ab_idx];
                            let label = format!("Activate {} ({})", ab.name, ab.description);
                            display_labels.push(label);
                            display_entries.push(DisplayEntry::Ability(ab_idx));
                        }
                    }
                }
                _ => {
                    display_labels.push(Self::format_single_action(view, action));
                    display_entries.push(DisplayEntry::Direct(i));
                }
            }
        }

        let context_str = legal.context.as_ref()
            .map(|c| format!("[{}]\n", c))
            .unwrap_or_default();
        let actions_str: String = display_labels.iter().enumerate()
            .map(|(i, label)| format!("{}:{} ", i, label))
            .collect();
        let action_prompt = format!("{}\n{}", context_str, actions_str);
        let prompt = self.build_prompt(view, &action_prompt);

        if display_labels.len() != legal_actions.len() {
            self.log("COLLAPSED", &format!("{} actions → {} options", legal_actions.len(), display_labels.len()));
        }
        let idx = self.pick_action_index(&prompt, display_labels.len(), legal_actions);

        if idx >= display_entries.len() {
            return Action::PassPriority;
        }

        match &display_entries[idx] {
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
}

impl LlmPlayer {
    /// Format a card for the mulligan prompt: `Name {cost}[ P/T]`.
    fn format_hand_card(c: &mtg_engine::view::CardView) -> String {
        let cost = c.cost.as_ref().map(|co| format!(" {}", co)).unwrap_or_default();
        let pt = match (c.power, c.toughness) {
            (Some(p), Some(t)) => format!(" {}/{}", p, t),
            _ => String::new(),
        };
        format!("{}{}{}", c.name, cost, pt)
    }

    /// Decide keep or mulligan for the London opening-hand phase.
    /// Sends a structured-JSON prompt with the current hand and the
    /// mulligan count. Falls back to MulliganKeep on malformed responses.
    fn choose_mulligan(&mut self, view: &GameView, legal_actions: &[Action]) -> Action {
        let mull_allowed = legal_actions.iter().any(|a| matches!(a, Action::MulliganMull));

        let numbered: Vec<String> = view.your_hand.iter().enumerate()
            .map(|(i, c)| format!("  {}: {}", i, Self::format_hand_card(c)))
            .collect();
        let hand_text = if numbered.is_empty() {
            "<empty>".to_string()
        } else {
            numbered.join("\n")
        };

        let mulls_taken = view.your_mulligan_count;
        let keep_size = (7_i32 - mulls_taken as i32).max(0);
        let mut action_prompt = String::new();
        action_prompt.push_str("[MULLIGAN DECISION]\n");
        action_prompt.push_str(&format!(
            "Mulligans taken so far: {}. If you keep now you will bottom {} card{} and play with {} in hand.\n",
            mulls_taken,
            mulls_taken,
            if mulls_taken == 1 { "" } else { "s" },
            keep_size,
        ));
        action_prompt.push_str("Your opening hand:\n");
        action_prompt.push_str(&hand_text);
        action_prompt.push('\n');
        if mull_allowed {
            action_prompt.push_str("Respond with {\"thoughts\": \"...\", \"mull\": true|false}.\n");
        } else {
            action_prompt.push_str("You have reached the mulligan cap (mull-to-4). You must keep. ");
            action_prompt.push_str("Respond with {\"thoughts\": \"...\", \"mull\": false}.\n");
        }

        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Private reasoning about whether to keep or mulligan"},
                "mull": {"type": "boolean", "description": "true = mulligan, false = keep"}
            },
            "required": ["thoughts", "mull"]
        });

        let full_prompt = self.build_prompt_with_header(
            view,
            &action_prompt,
            Some("Pre-game — London mulligan phase"),
        );
        let response = self.send_message_structured(&full_prompt, &schema);
        let choice = response["mull"].as_bool();
        match choice {
            Some(true) if mull_allowed => {
                self.log("CHOSE", "mulligan");
                Action::MulliganMull
            }
            Some(true) => {
                // Requested to mulligan past the cap — log and force keep.
                self.log("MALFORMED", "Requested mulligan past cap — forcing keep");
                Action::MulliganKeep
            }
            Some(false) => {
                self.log("CHOSE", "keep");
                Action::MulliganKeep
            }
            None => {
                self.log("MALFORMED", &format!("Mulligan response missing 'mull' bool ({}), defaulting to keep", response));
                Action::MulliganKeep
            }
        }
    }

    /// Decide which cards to put on the bottom after all mulligans.
    /// Sends a structured-JSON prompt with the numbered hand and expected
    /// count. Falls back to the first enumerated legal BottomCards option
    /// if the response is malformed.
    fn choose_mulligan_bottom(&mut self, view: &GameView, legal_actions: &[Action]) -> Action {
        // Determine N from the legal actions (every option has the same
        // length — enumerated combinations).
        let n = match legal_actions.iter().find_map(|a| match a {
            Action::BottomCards { cards } => Some(cards.len()),
            _ => None,
        }) {
            Some(n) => n,
            None => return legal_actions[0].clone(),
        };

        let numbered: Vec<String> = view.your_hand.iter().enumerate()
            .map(|(i, c)| format!("  {}: {}", i, Self::format_hand_card(c)))
            .collect();
        let hand_text = if numbered.is_empty() {
            "<empty>".to_string()
        } else {
            numbered.join("\n")
        };

        let mut action_prompt = String::new();
        action_prompt.push_str(&format!("[BOTTOM {} CARD{} AFTER MULLIGAN]\n", n,
            if n == 1 { "" } else { "S" }));
        action_prompt.push_str("Your opening hand:\n");
        action_prompt.push_str(&hand_text);
        action_prompt.push('\n');
        action_prompt.push_str(&format!(
            "Respond with {{\"thoughts\": \"...\", \"bottom_indices\": [..]}} — exactly {} distinct index(es) from 0 to {}.\n",
            n, view.your_hand.len().saturating_sub(1)));

        let valid_indices: Vec<serde_json::Value> = (0..view.your_hand.len())
            .map(|i| serde_json::json!(i))
            .collect();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "thoughts": {"type": "string", "description": "Private reasoning about which cards to bottom"},
                "bottom_indices": {
                    "type": "array",
                    "items": {"type": "integer", "enum": valid_indices},
                    "minItems": n,
                    "maxItems": n,
                    "description": format!("Exactly {} distinct 0-indexed positions in your hand to put on the bottom of your library", n)
                }
            },
            "required": ["thoughts", "bottom_indices"]
        });

        let full_prompt = self.build_prompt_with_header(
            view,
            &action_prompt,
            Some("Pre-game — Bottoming after mulligan"),
        );
        let response = self.send_message_structured(&full_prompt, &schema);

        // Parse and validate bottom_indices.
        let indices: Option<Vec<usize>> = response["bottom_indices"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_i64())
                .filter(|i| *i >= 0)
                .map(|i| i as usize)
                .collect()
        });
        let fallback = || -> Action {
            self.log("MALFORMED", "Invalid bottom_indices — defaulting to first legal bottom option");
            legal_actions[0].clone()
        };

        let indices = match indices {
            Some(v) if v.len() == n => v,
            _ => return fallback(),
        };
        // Check distinct and in range.
        let mut seen = std::collections::HashSet::new();
        for &i in &indices {
            if i >= view.your_hand.len() || !seen.insert(i) {
                return fallback();
            }
        }
        let cards: Vec<ObjectId> = indices.iter()
            .map(|&i| view.your_hand[i].object_id)
            .collect();
        self.log("CHOSE", &format!("bottom indices {:?}", indices));
        Action::BottomCards { cards }
    }

    /// Format keyword abilities as a comma-separated lowercase string.
    fn format_keywords(keywords: &[mtg_engine::types::Keyword]) -> String {
        use mtg_engine::types::Keyword;
        keywords.iter().map(|kw| match kw {
            Keyword::Flying => "flying",
            Keyword::FirstStrike => "first strike",
            Keyword::DoubleStrike => "double strike",
            Keyword::Trample => "trample",
            Keyword::Deathtouch => "deathtouch",
            Keyword::Lifelink => "lifelink",
            Keyword::Vigilance => "vigilance",
            Keyword::Flash => "flash",
            Keyword::Reach => "reach",
            Keyword::Haste => "haste",
            Keyword::Defender => "defender",
            Keyword::Hexproof => "hexproof",
            Keyword::Intimidate => "intimidate",
            Keyword::Menace => "menace",
            Keyword::Indestructible => "indestructible",
        }).collect::<Vec<_>>().join(", ")
    }

    /// Format a permanent for combat display: "Name P/T keywords" using effective values.
    fn format_combat_creature(view: &GameView, id: ObjectId) -> String {
        if let Some(p) = view.battlefield.iter().find(|p| p.object_id == id) {
            let power = p.effective_power.or(p.power).unwrap_or(0);
            let toughness = p.effective_toughness.or(p.toughness).unwrap_or(0);
            let kw = Self::format_keywords(&p.keywords);
            if kw.is_empty() {
                format!("{} {}/{}", p.name, power, toughness)
            } else {
                format!("{} {}/{} {}", p.name, power, toughness, kw)
            }
        } else {
            Self::obj_name(view, id)
        }
    }

    pub fn choose_combat(&mut self, view: &GameView, prompt: &CombatPrompt) -> Action {
        match prompt {
            CombatPrompt::ChooseAttackers { eligible, must_attack, defending_player } => {
                if eligible.is_empty() {
                    return Action::DeclareAttackers { attackers: vec![] };
                }

                let mut combat_text = String::new();
                if !must_attack.is_empty() {
                    combat_text.push_str("MUST ATTACK: ");
                    for &id in must_attack.iter() {
                        if let Some(idx) = eligible.iter().position(|&e| e == id) {
                            combat_text.push_str(&format!("{}:{} ", idx, Self::format_combat_creature(view, id)));
                        }
                    }
                    combat_text.push('\n');
                }
                combat_text.push_str("Choose attackers: ");
                for (i, &id) in eligible.iter().enumerate() {
                    let forced = if must_attack.contains(&id) { " [MUST]" } else { "" };
                    combat_text.push_str(&format!("{}:{}{} ", i, Self::format_combat_creature(view, id), forced));
                }
                combat_text.push_str(&format!(
                    "\nRespond with attacker_indices (list of numbers 0-{}), or empty list for none. Forced attackers are auto-included.",
                    eligible.len() - 1
                ));

                let full_prompt = self.build_prompt(view, &combat_text);

                let schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "thoughts": {"type": "string", "description": "Brief reasoning about which creatures to attack with"},
                        "attacker_indices": {
                            "type": "array",
                            "items": {"type": "integer", "minimum": 0},
                            "description": "Indices of creatures to attack with (empty array for none)"
                        }
                    },
                    "required": ["thoughts", "attacker_indices"]
                });

                let response = self.send_message_structured(&full_prompt, &schema);

                let mut indices: Vec<usize> = response["attacker_indices"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .filter(|&i| i < eligible.len())
                        .collect())
                    .unwrap_or_default();

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

                let attackers = indices.iter()
                    .map(|&i| (eligible[i], *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers }
            }

            CombatPrompt::ChooseBlockers { eligible_blockers, attackers, legal_blocks } => {
                self.choose_blockers_structured(view, eligible_blockers, attackers, legal_blocks)
            }
        }
    }

    /// Validate blocker assignments and return a list of error messages.
    /// Returns an empty vec if assignments are valid.
    fn validate_blocker_assignments(
        view: &GameView,
        assignments: &[(ObjectId, ObjectId)],
        attackers: &[ObjectId],
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // Menace: if an attacker with menace is blocked, it must have 2+ blockers.
        use mtg_engine::types::Keyword;
        let mut blocker_counts: std::collections::HashMap<ObjectId, Vec<ObjectId>> = std::collections::HashMap::new();
        for &(blocker, attacker) in assignments {
            blocker_counts.entry(attacker).or_default().push(blocker);
        }
        for (att_idx, &attacker_id) in attackers.iter().enumerate() {
            if let Some(attacker) = view.battlefield.iter().find(|p| p.object_id == attacker_id) {
                if attacker.keywords.contains(&Keyword::Menace) {
                    if let Some(blockers) = blocker_counts.get(&attacker_id) {
                        if blockers.len() == 1 {
                            errors.push(format!(
                                "Attacker {} ({}) has MENACE and must be blocked by at least 2 creatures, but you only assigned 1 blocker. Either assign more blockers to it or set all blockers to -1 (don't block it at all).",
                                att_idx, Self::format_combat_creature(view, attacker_id)
                            ));
                        }
                    }
                }
            }
        }

        errors
    }

    /// Declare blockers using structured output with per-blocker integer enum
    /// constraints and a validation retry loop.
    fn choose_blockers_structured(
        &mut self,
        view: &GameView,
        eligible_blockers: &[ObjectId],
        attackers: &[ObjectId],
        legal_blocks: &std::collections::HashMap<ObjectId, Vec<ObjectId>>,
    ) -> Action {
        if eligible_blockers.is_empty() || attackers.is_empty() {
            return Action::DeclareBlockers { assignments: vec![] };
        }

        // Build per-blocker integer enum of legal attacker indices.
        // -1 means "don't block".
        let mut schema_properties = serde_json::json!({
            "thoughts": {"type": "string", "description": "Brief reasoning about blocking decisions"}
        });
        let mut required_fields = vec!["thoughts".to_string()];

        for (i, &blocker_id) in eligible_blockers.iter().enumerate() {
            let blocker_legal = legal_blocks.get(&blocker_id);
            let mut legal_indices: Vec<serde_json::Value> = attackers.iter()
                .enumerate()
                .filter(|(_, &att_id)| {
                    blocker_legal.map_or(false, |legal| legal.contains(&att_id))
                })
                .map(|(idx, _)| serde_json::json!(idx))
                .collect();
            legal_indices.push(serde_json::json!(-1));

            let key = i.to_string();
            schema_properties[&key] = serde_json::json!({
                "type": "integer",
                "enum": legal_indices,
                "description": format!("Attacker index for blocker {} to block, or -1 for no block", i)
            });
            required_fields.push(key);
        }

        let schema = serde_json::json!({
            "type": "object",
            "properties": schema_properties,
            "required": required_fields
        });

        // Build combat text for the prompt.
        let mut combat_text = String::from("Attackers: ");
        for (i, &id) in attackers.iter().enumerate() {
            let perm = view.battlefield.iter().find(|p| p.object_id == id);
            let menace = perm.map_or(false, |p| p.keywords.contains(&mtg_engine::types::Keyword::Menace));
            if menace {
                combat_text.push_str(&format!("{}:{} (MENACE) ", i, Self::format_combat_creature(view, id)));
            } else {
                combat_text.push_str(&format!("{}:{} ", i, Self::format_combat_creature(view, id)));
            }
        }
        combat_text.push_str("\nYour blockers: ");
        for (i, &id) in eligible_blockers.iter().enumerate() {
            combat_text.push_str(&format!("{}:{} ", i, Self::format_combat_creature(view, id)));
        }
        combat_text.push_str("\nFor each blocker, respond with the attacker index to block, or -1 for no block.");

        let base_prompt = self.build_prompt(view, &combat_text);

        // Retry loop with validation.
        let max_retries = 20;
        let mut retry_message: Option<String> = None;
        for attempt in 0..max_retries {
            let prompt = if let Some(ref msg) = retry_message {
                format!("{}\n\nPREVIOUS RESPONSE WAS INVALID:\n{}\nPlease try again.", base_prompt, msg)
            } else {
                base_prompt.clone()
            };

            let response = self.send_message_structured(&prompt, &schema);

            // Parse response into assignments.
            let mut assignments: Vec<(ObjectId, ObjectId)> = Vec::new();
            for (i, &blocker_id) in eligible_blockers.iter().enumerate() {
                let key = i.to_string();
                if let Some(att_idx) = response[&key].as_i64() {
                    if att_idx >= 0 && (att_idx as usize) < attackers.len() {
                        assignments.push((blocker_id, attackers[att_idx as usize]));
                    }
                }
            }

            // Validate.
            let errors = Self::validate_blocker_assignments(view, &assignments, attackers);
            if errors.is_empty() {
                return Action::DeclareBlockers { assignments };
            }

            self.log("BLOCKER_VALIDATION", &format!("attempt {} errors: {:?}", attempt + 1, errors));
            retry_message = Some(errors.join("\n"));
        }

        // Exhausted retries — return no blocks as safe fallback.
        self.log("BLOCKER_VALIDATION", "exhausted retries, defaulting to no blocks");
        Action::DeclareBlockers { assignments: vec![] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mtg_engine::types::CounterType;
    use std::collections::HashMap;

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

    #[test]
    fn format_counters_ignores_slime_and_study() {
        // Non-P/T, non-loyalty counters should not appear in the suffix
        // (they're not the point of this display flag).
        let mut counters = HashMap::new();
        counters.insert(CounterType::Slime, 5);
        counters.insert(CounterType::Study, 2);
        assert_eq!(LlmPlayer::format_counters(&counters), None);
    }
}
