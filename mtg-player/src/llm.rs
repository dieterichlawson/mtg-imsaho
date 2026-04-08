use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::ids::ObjectId;
use mtg_engine::types::{CardType, Step};
use mtg_engine::view::GameView;
use reqwest::blocking::Client;

use crate::Player;

/// Global token usage counters for LlmPlayer API calls.
pub static LLM_INPUT_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static LLM_OUTPUT_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static LLM_CACHE_READ_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static LLM_CACHE_CREATION_TOKENS: AtomicU64 = AtomicU64::new(0);
pub static LLM_API_CALLS: AtomicU64 = AtomicU64::new(0);

fn record_llm_usage(json: &serde_json::Value) {
    let usage = &json["usage"];
    LLM_API_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Some(n) = usage["input_tokens"].as_u64() {
        LLM_INPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["output_tokens"].as_u64() {
        LLM_OUTPUT_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["cache_read_input_tokens"].as_u64() {
        LLM_CACHE_READ_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
    if let Some(n) = usage["cache_creation_input_tokens"].as_u64() {
        LLM_CACHE_CREATION_TOKENS.fetch_add(n, Ordering::Relaxed);
    }
}

const SYSTEM_PROMPT: &str = r#"You are playing Magic: The Gathering. Respond with ONLY your choice. No explanation, no reasoning, just the answer.

## Response format
- Action selection: a single number (e.g. "3")
- Choosing attackers: space-separated numbers, "all", or "none"
- Choosing blockers: "blocker:attacker" pairs (e.g. "0:0 1:2"), or "none"

You may briefly reason about your decision. Your FINAL LINE must be ONLY your answer — a single number, space-separated numbers, "all", or "none". Nothing else on that line.

Example response for action selection:
I should tap my Forest to build toward casting Kalonian Tusker next action.
ANSWER: 1

Example response for attackers:
I have two 3/3s and opponent has no blockers. Attack with everything.
ANSWER: all

Example response for blockers:
Block the 3/3 with my 2/1 to prevent damage.
ANSWER: 0:0

The system parses ONLY the last line. If the last line isn't a valid number/format, you default to passing.

## Key rules
- Mana pools empty at EVERY step boundary. Tap lands and cast spells in the same step.
- The "Cast" option only appears AFTER you have enough mana in pool. Tap lands first.
- Generic mana (numbers like {1}, {2}) can be paid with ANY color. For example, {1}{G} can be paid with {G}{G} — the first {G} pays the generic {1} cost. So if a spell costs {1}{G}, tapping two Forests ({G}{G}) is enough.
- Spells go on the stack and resolve when both players pass priority.
- Creatures have summoning sickness — can't attack the turn they enter. [S] means sick.
- Play one land per turn, only during your main phase.
- Instants can be cast anytime you have priority (including during combat or opponent's turn).
- Sorceries, creatures, enchantments, and artifacts can only be cast during your main phase with an empty stack.
- Targeted spells show their target in the action (e.g. "Cast Lightning Bolt → Goblin Piker 2/1").
- Attack to win! Creatures deal damage to the opponent when unblocked.

## Flashback
Cards with flashback can be cast from your graveyard for their flashback cost. After resolving, they are exiled (not returned to graveyard). Look for "Flashback" in the action list — these are graveyard casts. Tap lands to get mana, then the Flashback option appears.

## Strategy tips
- Save instants for combat! Giant Growth during DeclareBlockers makes your 2/2 into a 5/5. Lightning Bolt during DeclareAttackers can kill a would-be blocker.
- Don't cast Giant Growth during your main phase — it wears off at end of turn and wastes it if there's no combat.
- Use removal (Doom Blade, Swords, Lightning Bolt) on your opponent's biggest creatures, especially before they attack you.
- Cast creatures and play lands during PrecombatMain. Use PostcombatMain for spells you want to cast after seeing how combat went.
- Don't tap lands unless you have something to cast with the mana. Tapping 1 land when you need 2 for a spell just wastes it.
- TAP THE RIGHT COLORS! Look at the costs of cards in your hand. If you need {G}{G}, tap Forests not Mountains. If you need {1}{R}, tap one Mountain and one of anything. The action list shows "Tap Forest" vs "Tap Mountain" — pick the ones matching your spell's colored requirements first.

## CRITICAL RULE: Only tap lands during PrecombatMain or PostcombatMain

The step name is shown at the top of every prompt (e.g. "T3 PrecombatMain", "T3 Upkeep", "T3 Draw").
Mana pools empty between EVERY step. If you tap during Upkeep, Draw, BeginCombat, EndStep, or any non-main step, the mana disappears before you can use it and your lands are tapped for nothing.

EXCEPTION: You MAY tap lands during combat steps (DeclareAttackers, DeclareBlockers) to cast instants like Lightning Bolt or Giant Growth. This is useful for removing blockers or pumping attackers.

## Mistake example: Tapping during Upkeep or Draw

```
T3 Upkeep You:20hp Opp:20hp,6cards
Your board: 2xForest
0:Pass 1:Tap Forest 2:Tap Forest 3:Concede
```
If you answer 1 here (tap Forest), you get {G} in your pool. But when Upkeep ends and Draw begins, YOUR MANA POOL EMPTIES. By the time you reach PrecombatMain, the mana is gone and your lands are tapped for nothing.
Answer: 0

Same applies to Draw step — don't tap unless you have an instant to cast RIGHT NOW.

## Correct example: Full main phase sequence

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Your board: 2xForest
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Tap Forest 2:Tap Forest 3:Play Forest 4:Play Forest 5:Concede
```
Answer: 1 (tap Forest → {G})

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Pool: {Green: 1}
Your board: 2xForest(1tapped)
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Tap Forest 2:Play Forest 3:Play Forest 4:Concede
```
Answer: 1 (tap Forest → {G}{G})

```
T3 PrecombatMain You:20hp Opp:20hp,6cards
Pool: {Green: 2}
Your board: 2xForest(tapped)
Hand: Kalonian Tusker{G}{G} 3/3, Forest, Forest
0:Pass 1:Cast Kalonian Tusker 2:Play Forest 3:Play Forest 4:Concede
```
Answer: 1 (cast!)

## Correct example: Using an instant during combat

```
T5 DeclareBlockers You:20hp Opp:18hp,4cards
Pool: {Green: 1}
Your board: 2xForest(1tapped), Grizzly Bears 2/2[T]
Opp board: 2xPlains, Savannah Lions 2/1
Hand: Giant Growth{G}
Attackers: 0:Grizzly Bears 2/2
0:Pass 1:Cast Giant Growth → Grizzly Bears 2/2 2:Concede
```
Answer: 1 (pump your attacking creature to 5/5 before blockers deal damage!)

## Correct example: Declare attackers

```
T5 DeclareAttackers You:20hp Opp:20hp,5cards
Your board: 3xForest(tapped), Kalonian Tusker 3/3, Kalonian Tusker 3/3
Opp board: 2xMountain, Goblin Piker 2/1
Choose attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Numbers, 'all', or 'none'
```
Answer: all

## Correct example: Declare blockers

```
T6 DeclareBlockers You:17hp Opp:20hp,5cards
Your board: 3xMountain, Goblin Piker 2/1, Goblin Piker 2/1
Opp board: 3xForest(tapped), Kalonian Tusker 3/3[T], Kalonian Tusker 3/3[T]
Attackers: 0:Kalonian Tusker 3/3 1:Kalonian Tusker 3/3
Your blockers: 0:Goblin Piker 2/1 1:Goblin Piker 2/1
Format: 'blocker:attacker' pairs, or 'none'
```
Answer: 0:0 1:1
(Block both. Your 2/1s die but prevent 6 damage.)

IMPORTANT: For blocking, the format is BLOCKER_NUMBER:ATTACKER_NUMBER (e.g. "0:0" NOT "0:" or "0>0"). Both numbers are required.
"#;

#[derive(Clone)]
pub enum Provider {
    Anthropic,
    Gemini,
}

pub struct LlmPlayer {
    name: String,
    client: Client,
    api_key: String,
    model: String,
    provider: Provider,
    log_file: Option<String>,
    /// System prompt (rules + decklists). Set by init_conversation.
    system_prompt: String,
    /// Multi-turn conversation history for Anthropic API.
    /// Each entry is a {"role": "user"|"assistant", "content": "..."} JSON object.
    conversation: Vec<serde_json::Value>,
    /// Index into the game log — tracks which log entries have been sent.
    last_log_index: usize,
}

impl LlmPlayer {
    pub fn new(name: &str) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY environment variable must be set");

        Self {
            name: name.to_string(),
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-6".to_string(),
            provider: Provider::Anthropic,
            log_file: None,
            system_prompt: SYSTEM_PROMPT.to_string(),
            conversation: Vec::new(),
            last_log_index: 0,
        }
    }

    pub fn new_gemini(name: &str) -> Self {
        let api_key = env::var("GEMINI_API_KEY")
            .expect("GEMINI_API_KEY environment variable must be set");

        Self {
            name: name.to_string(),
            client: Client::new(),
            api_key,
            model: "gemini-2.5-flash".to_string(),
            provider: Provider::Gemini,
            log_file: None,
            system_prompt: SYSTEM_PROMPT.to_string(),
            conversation: Vec::new(),
            last_log_index: 0,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_log(mut self, path: &str) -> Self {
        // Truncate the file at start.
        let _ = std::fs::write(path, "");
        self.log_file = Some(path.to_string());
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
        self.system_prompt = format!("{}{}", SYSTEM_PROMPT, deck_info);
        self.conversation.clear();
        self.last_log_index = 0;
        self.log("SYSTEM", &self.system_prompt);
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

        // Add as a user message with a synthetic assistant acknowledgment.
        self.conversation.push(serde_json::json!({
            "role": "user",
            "content": recap,
        }));
        self.conversation.push(serde_json::json!({
            "role": "assistant",
            "content": "Understood. I've reviewed the game history and I'm ready to continue playing.",
        }));
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

    /// Expose system prompt for testing.
    pub fn system_prompt_for_test(&self) -> &str {
        &self.system_prompt
    }

    /// Expose conversation length for testing.
    pub fn conversation_len_for_test(&self) -> usize {
        self.conversation.len()
    }

    /// Expose last_log_index for testing.
    pub fn last_log_index_for_test(&self) -> usize {
        self.last_log_index
    }

    fn log(&self, label: &str, content: &str) {
        if let Some(path) = &self.log_file {
            if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
                let _ = writeln!(f, "=== {} [{}] ===", label, self.name);
                let _ = writeln!(f, "{}", content);
                let _ = writeln!(f);
            }
        }
    }

    /// Check if the AI should auto-pass (nothing interesting to do).
    fn should_auto_pass(_view: &GameView, actions: &[Action]) -> bool {
        // Auto-pass when the only options are Pass and Concede.
        let has_pass = actions.iter().any(|a| matches!(a, Action::PassPriority));
        if !has_pass {
            return false;
        }
        actions.iter().all(|a| matches!(a, Action::PassPriority | Action::Concede))
    }

    /// Compact game state — much shorter than the CLI version.
    fn format_state_compact(view: &GameView) -> String {
        let mut s = String::new();

        // Turn, phase, whose turn
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
                    format!("{} ({})", i.name, who)
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
                let names: Vec<&str> = cards.iter().map(|c| c.name.as_str()).collect();
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

        // Collect aura names by what they're attached to — search ALL permanents
        // so we find auras that cross controller boundaries (e.g., opponent's
        // Pacifism on your creature).
        let mut aura_map: std::collections::HashMap<mtg_engine::ids::ObjectId, Vec<String>> = std::collections::HashMap::new();
        for o in all_perms {
            if o.attached_to.is_some() && !o.card_types.contains(&CardType::Land) && !o.card_types.contains(&CardType::Creature) {
                if let Some(target_id) = o.attached_to {
                    aura_map.entry(target_id).or_default().push(o.name.clone());
                }
            }
        }

        for c in &creatures {
            let power = c.effective_power.or(c.power).unwrap_or(0);
            let toughness = c.effective_toughness.or(c.toughness).unwrap_or(0);
            let t = if c.tapped { "T" } else { "" };
            let s = if c.summoning_sick { "S" } else { "" };
            let d = if c.damage_marked > 0 { format!("{}dmg", c.damage_marked) } else { String::new() };
            let flags = format!("{}{}{}", t, s, d);
            let flags_str = if flags.is_empty() { String::new() } else { format!(" [{}]", flags) };
            let auras = aura_map.get(&c.object_id)
                .map(|names| format!(" ({})", names.join(", ")))
                .unwrap_or_default();
            parts.push(format!("{} {}/{}{}{}", c.name, power, toughness, flags_str, auras));
        }

        // Show non-aura other permanents.
        for o in &other {
            if o.attached_to.is_some() { continue; } // skip auras, shown with creature
            let t = if o.tapped { "[T]" } else { "" };
            parts.push(format!("{}{}", o.name, t));
        }

        parts.join(", ")
    }

    /// Format a single non-CastSpell action for the collapsed display.
    fn format_single_action(view: &GameView, action: &Action) -> String {
        match action {
            Action::PassPriority => "Pass".into(),
            Action::PlayLand { object_id } => format!("Play {}", Self::obj_name(view, *object_id)),
            Action::ActivateManaAbility { object_id, .. } => format!("Tap {}", Self::obj_name(view, *object_id)),
            Action::ActivateAbility { object_id, .. } => format!("Activate {}", Self::obj_name(view, *object_id)),
            Action::Concede => "Concede".into(),
            Action::DiscardCards { cards } => format!("Discard {} cards", cards.len()),
            Action::ResolveChoice { choice } => {
                use mtg_engine::actions::ResolvedChoice;
                match choice {
                    ResolvedChoice::PayDecision(true) => "Pay {1}".into(),
                    ResolvedChoice::PayDecision(false) => "Don't pay (countered)".into(),
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

        match &spell.target_spec {
            CastTargetSpec::NoTargets => {
                Action::CastSpell { object_id: spell.object_id, targets: vec![], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
            }
            CastTargetSpec::SingleTarget(options) => {
                if options.len() == 1 {
                    return Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() };
                }
                let target = self.prompt_target_selection(view, &format!("{}: select a target", spell.name), options);
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
                    "{}: select up to {} targets (you may choose fewer):\n{}\nRespond with space-separated numbers (e.g. '0' for one target, '0 2' for two)",
                    spell.name, max, target_list
                );
                self.log("TARGETS", &prompt);
                let response = self.call_api(&prompt);
                self.log("TARGET-RESPONSE", &response);

                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim();

                let chosen: Vec<Target> = answer.split_whitespace()
                    .filter_map(|s| s.parse::<usize>().ok())
                    .filter(|&i| i < options.len())
                    .map(|i| options[i].clone())
                    .collect();

                if chosen.is_empty() {
                    // Pick at least one — use the first option.
                    Action::CastSpell { object_id: spell.object_id, targets: vec![options[0].clone()], sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
                } else {
                    Action::CastSpell { object_id: spell.object_id, targets: chosen, sacrifice: None, exile_count: None, exile_ids: vec![], alternative_cost: None, tap_plan: spell.tap_plan.clone() }
                }
            }
        }
    }

    /// Make a second API call to select one target from a list.
    fn prompt_target_selection(&mut self, view: &GameView, spell_name: &str, options: &[mtg_engine::actions::Target]) -> mtg_engine::actions::Target {
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
        let idx = self.choose_with_retry(&prompt, options.len(), &[]);
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
            .unwrap_or_else(|| format!("{}", id))
    }

    /// Build a user message with log delta + board state + prompt, append to
    /// conversation, send to API, append assistant response, return the text.
    fn send_message(&mut self, user_message: &str) -> String {
        self.log("PROMPT", user_message);

        // Append user message to conversation.
        self.conversation.push(serde_json::json!({
            "role": "user",
            "content": user_message,
        }));

        let result = match self.provider {
            Provider::Anthropic => self.call_anthropic_conv(),
            Provider::Gemini => self.call_gemini_conv(),
        };

        // Append assistant response.
        self.conversation.push(serde_json::json!({
            "role": "assistant",
            "content": result,
        }));

        result
    }

    /// Build a prompt that includes new log entries + board state + the action prompt.
    fn build_prompt(&mut self, view: &GameView, action_prompt: &str) -> String {
        // Collect new log entries since last message.
        let new_logs: Vec<String> = view.full_log.iter()
            .skip(self.last_log_index)
            .cloned()
            .collect();
        self.last_log_index = view.full_log.len();

        let mut prompt = String::new();
        if !new_logs.is_empty() {
            prompt.push_str("Recent events:\n");
            for entry in &new_logs {
                prompt.push_str(entry);
                prompt.push('\n');
            }
            prompt.push('\n');
        }
        prompt.push_str(&Self::format_state_compact(view));
        prompt.push_str(action_prompt);
        prompt
    }

    /// Legacy one-shot API call (for sub-prompts like target selection within a turn).
    fn call_api(&self, user_message: &str) -> String {
        self.log("PROMPT", user_message);

        match self.provider {
            Provider::Anthropic => {
                // Use conversation context for sub-prompts too.
                let mut messages = self.conversation.clone();
                messages.push(serde_json::json!({"role": "user", "content": user_message}));
                self.call_anthropic_with_messages(&messages)
            }
            Provider::Gemini => self.call_gemini_oneshot(user_message),
        }
    }

    fn call_anthropic_conv(&self) -> String {
        self.call_anthropic_with_messages(&self.conversation)
    }

    fn call_anthropic_with_messages(&self, messages: &[serde_json::Value]) -> String {
        // Set cache_control on the system prompt (always cached) and on
        // the second-to-last message (conversation prefix cached).
        let system = serde_json::json!([{
            "type": "text",
            "text": self.system_prompt,
            "cache_control": {"type": "ephemeral"}
        }]);

        let mut msgs = messages.to_vec();
        // Set cache_control on the second-to-last message if there are at least 2.
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

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": msgs
        });

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                self.log("RETRY", &format!("Retrying in {}s...", delay.as_secs()));
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
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        record_llm_usage(&json);
                        let result = json["content"][0]["text"]
                            .as_str()
                            .unwrap_or("0")
                            .trim()
                            .to_string();
                        self.log("RESPONSE", &result);
                        return result;
                    }

                    let status = resp.status();
                    let text = resp.text().unwrap_or_default();
                    self.log("API-ERROR", &format!("{}: {}", status, text));
                    self.log("ERROR", &format!("API {} - {}", status, text));

                    // Retry on overload (529) or rate limit (429).
                    let code = status.as_u16();
                    if code == 529 || code == 429 {
                        continue;
                    }
                    // Non-retryable error.
                    return "0".to_string();
                }
                Err(e) => {
                    self.log("REQUEST-FAILED", &format!("{}", e));
                    self.log("ERROR", &format!("Request failed: {}", e));
                    continue;
                }
            }
        }

        self.log("ERROR", "All retries exhausted, defaulting to 0");
        "0".to_string()
    }

    fn call_gemini_conv(&self) -> String {
        // Convert conversation to Gemini format.
        let contents: Vec<serde_json::Value> = self.conversation.iter().map(|msg| {
            let role = msg["role"].as_str().unwrap_or("user");
            let gemini_role = if role == "assistant" { "model" } else { "user" };
            serde_json::json!({"role": gemini_role, "parts": [{"text": msg["content"].as_str().unwrap_or("")}]})
        }).collect();
        self.call_gemini_with_contents(&contents)
    }

    fn call_gemini_oneshot(&self, user_message: &str) -> String {
        let mut contents: Vec<serde_json::Value> = self.conversation.iter().map(|msg| {
            let role = msg["role"].as_str().unwrap_or("user");
            let gemini_role = if role == "assistant" { "model" } else { "user" };
            serde_json::json!({"role": gemini_role, "parts": [{"text": msg["content"].as_str().unwrap_or("")}]})
        }).collect();
        contents.push(serde_json::json!({"role": "user", "parts": [{"text": user_message}]}));
        self.call_gemini_with_contents(&contents)
    }

    fn call_gemini_with_contents(&self, contents: &[serde_json::Value]) -> String {
        let body = serde_json::json!({
            "contents": contents,
            "systemInstruction": {"parts": [{"text": self.system_prompt}]},
            "generationConfig": {"maxOutputTokens": 512}
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        for attempt in 0..3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(2u64.pow(attempt as u32));
                self.log("RETRY", &format!("Retrying in {}s...", delay.as_secs()));
                std::thread::sleep(delay);
            }

            let response = self.client
                .post(&url)
                .header("content-type", "application/json")
                .json(&body)
                .send();

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp.json().unwrap_or_default();
                        // Gemini uses usageMetadata instead of usage
                        record_llm_usage(&json["usageMetadata"]);
                        let result = json["candidates"][0]["content"]["parts"][0]["text"]
                            .as_str()
                            .unwrap_or("0")
                            .trim()
                            .to_string();
                        self.log("RESPONSE", &result);
                        return result;
                    }

                    let status = resp.status();
                    let text = resp.text().unwrap_or_default();
                    self.log("API-ERROR", &format!("{}: {}", status, text));
                    self.log("ERROR", &format!("API {} - {}", status, text));

                    let code = status.as_u16();
                    if code == 429 || code == 503 || code == 529 {
                        continue;
                    }
                    return "0".to_string();
                }
                Err(e) => {
                    self.log("REQUEST-FAILED", &format!("{}", e));
                    self.log("ERROR", &format!("Request failed: {}", e));
                    continue;
                }
            }
        }

        self.log("ERROR", "All retries exhausted, defaulting to 0");
        "0".to_string()
    }

    fn parse_action_index(&self, response: &str, max: usize) -> Option<usize> {
        // Check the last non-empty line first (where the model puts its final answer).
        let last_line = response.lines().rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(response)
            .trim();

        // Strip "ANSWER:" prefix if present.
        let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
            .unwrap_or(last_line)
            .trim();

        for word in answer.split_whitespace() {
            if let Ok(n) = word.parse::<usize>() {
                if n < max {
                    return Some(n);
                }
            }
        }

        // Fallback: scan the entire response for any valid number.
        for word in response.split_whitespace() {
            if let Ok(n) = word.parse::<usize>() {
                if n < max {
                    return Some(n);
                }
            }
        }
        None
    }

    /// Send a message via conversation and parse the action index.
    fn choose_with_retry_conv(&mut self, prompt: &str, max: usize, actions: &[Action]) -> usize {
        for attempt in 0..2 {
            let response = self.send_message(prompt);
            if let Some(idx) = self.parse_action_index(&response, max) {
                // Double-check concede: ask the model to confirm.
                if matches!(actions.get(idx), Some(Action::Concede)) {
                    self.log("CONCEDE-CHECK", "AI chose Concede, confirming...");
                    let confirm = self.send_message(
                        "You chose to CONCEDE the game. Are you sure? Reply ONLY 'yes' or 'no'."
                    );
                    let last = confirm.lines().rev()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("")
                        .trim()
                        .to_lowercase();
                    if !last.contains("yes") {
                        self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
                        return 0;
                    }
                }
                self.log("CHOSE", &format!("action {}", idx));
                return idx;
            }
            if attempt == 0 {
                self.log("MALFORMED", &format!("'{}', retrying...", response));
            } else {
                self.log("MALFORMED", &format!("Retry also malformed '{}', defaulting to 0", response));
            }
        }
        0
    }

    /// Call the API with a retry on malformed response (legacy one-shot).
    fn choose_with_retry(&self, prompt: &str, max: usize, actions: &[Action]) -> usize {
        for attempt in 0..2 {
            let response = self.call_api(prompt);
            if let Some(idx) = self.parse_action_index(&response, max) {
                if matches!(actions.get(idx), Some(Action::Concede)) {
                    self.log("CONCEDE-CHECK", "AI chose Concede, confirming...");
                    let confirm = self.call_api(
                        "You chose to CONCEDE the game. Are you sure? Reply ONLY 'yes' or 'no'."
                    );
                    let last = confirm.lines().rev()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("")
                        .trim()
                        .to_lowercase();
                    if !last.contains("yes") {
                        self.log("CONCEDE-CHECK", "Concede cancelled, passing instead");
                        return 0;
                    }
                }
                self.log("CHOSE", &format!("action {}", idx));
                return idx;
            }
            if attempt == 0 {
                self.log("MALFORMED", &format!("'{}', retrying...", response));
            } else {
                self.log("MALFORMED", &format!("Retry also malformed '{}', defaulting to 0", response));
            }
        }
        0
    }
}

impl Player for LlmPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn choose_action(&mut self, view: &GameView, legal: &mtg_engine::engine::LegalActions) -> Action {
        let legal_actions = &legal.actions;
        // Auto-pass when there's nothing interesting to do.
        if Self::should_auto_pass(view, legal_actions) {
            self.log("AUTO-PASS", &format!("Step: {:?}, active: p#{}", view.step, view.active_player.0));
            return Action::PassPriority;
        }

        // Build collapsed display: non-CastSpell actions + one per castable spell.
        let mut display_labels = Vec::new();
        enum DisplayEntry {
            Direct(usize),   // index into legal_actions
            Cast(usize),     // index into legal.castable_spells
        }
        let mut display_entries: Vec<DisplayEntry> = Vec::new();
        let mut seen_spell_objects: Vec<mtg_engine::ids::ObjectId> = Vec::new();

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
                            let label = if tap_str.is_empty() {
                                format!("{} {}", verb, cs.name)
                            } else {
                                format!("{} {} (tap {})", verb, cs.name, tap_str)
                            };
                            // Deduplicate identical cast labels (e.g. two copies of same spell).
                            if seen_cast_labels.contains(&label) { continue; }
                            seen_cast_labels.push(label.clone());
                            display_labels.push(label);
                            display_entries.push(DisplayEntry::Cast(cs_idx));
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

        self.log("THINKING", &format!("{} actions (collapsed from {})", display_labels.len(), legal_actions.len()));
        let idx = self.choose_with_retry_conv(&prompt, display_labels.len(), legal_actions);

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
        }
    }

    fn choose_cards_to_bottom(
        &mut self,
        _view: &GameView,
        hand: &[mtg_engine::view::CardView],
        count: usize,
    ) -> Vec<ObjectId> {
        hand.iter().take(count).map(|c| c.object_id).collect()
    }
}

impl LlmPlayer {
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
                            let p = view.battlefield.iter().find(|p| p.object_id == id);
                            let name = p.map(|p| format!("{} {}/{}", p.name, p.power.unwrap_or(0), p.toughness.unwrap_or(0)))
                                .unwrap_or_else(|| format!("{}", id));
                            combat_text.push_str(&format!("{}:{} ", idx, name));
                        }
                    }
                    combat_text.push('\n');
                }
                combat_text.push_str("Choose attackers: ");
                for (i, &id) in eligible.iter().enumerate() {
                    let p = view.battlefield.iter().find(|p| p.object_id == id);
                    let name = p.map(|p| format!("{} {}/{}", p.name, p.power.unwrap_or(0), p.toughness.unwrap_or(0)))
                        .unwrap_or_else(|| format!("{}", id));
                    let forced = if must_attack.contains(&id) { " [MUST]" } else { "" };
                    combat_text.push_str(&format!("{}:{}{} ", i, name, forced));
                }
                combat_text.push_str("\nNumbers, 'all', or 'none' (forced attackers are auto-included)");

                self.log("THINKING", "attackers...");
                let full_prompt = self.build_prompt(view, &combat_text);
                let response = self.send_message(&full_prompt);
                self.log("ATTACKERS", &response);

                // Parse from last line, strip "ANSWER:" prefix.
                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim()
                    .to_lowercase();

                if answer.contains("none") || answer.is_empty() {
                    return Action::DeclareAttackers { attackers: vec![] };
                }
                if answer.contains("all") {
                    return Action::DeclareAttackers {
                        attackers: eligible.iter().map(|&id| (id, *defending_player)).collect(),
                    };
                }

                let attackers = answer.split_whitespace()
                    .filter_map(|s| s.parse::<usize>().ok())
                    .filter(|&i| i < eligible.len())
                    .map(|i| (eligible[i], *defending_player))
                    .collect();
                Action::DeclareAttackers { attackers }
            }

            CombatPrompt::ChooseBlockers { eligible_blockers, attackers } => {
                if eligible_blockers.is_empty() || attackers.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }

                let mut combat_text = String::from("Attackers: ");
                for (i, &id) in attackers.iter().enumerate() {
                    let name = Self::obj_name(view, id);
                    combat_text.push_str(&format!("{}:{} ", i, name));
                }
                combat_text.push_str("\nYour blockers: ");
                for (i, &id) in eligible_blockers.iter().enumerate() {
                    let name = Self::obj_name(view, id);
                    combat_text.push_str(&format!("{}:{} ", i, name));
                }
                combat_text.push_str("\nFormat: 'blocker:attacker' pairs, or 'none'");

                self.log("THINKING", "blockers...");
                let full_prompt = self.build_prompt(view, &combat_text);
                let response = self.send_message(&full_prompt);
                self.log("BLOCKERS", &response);

                // Parse from last line, strip "ANSWER:" prefix.
                let last_line = response.lines().rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or(&response)
                    .trim();
                let answer = last_line.strip_prefix("ANSWER:").or_else(|| last_line.strip_prefix("Answer:"))
                    .unwrap_or(last_line)
                    .trim();

                let lower = answer.to_lowercase();
                if lower.contains("none") || lower.is_empty() {
                    return Action::DeclareBlockers { assignments: vec![] };
                }

                let mut assignments = Vec::new();
                for pair in answer.split_whitespace() {
                    // Accept both "0:0" and "0->0" formats
                    let parts: Vec<&str> = if pair.contains("->") {
                        pair.split("->").collect()
                    } else {
                        pair.split(':').collect()
                    };
                    if parts.len() == 2 {
                        if let (Ok(b), Ok(a)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            if b < eligible_blockers.len() && a < attackers.len() {
                                assignments.push((eligible_blockers[b], attackers[a]));
                            }
                        }
                    }
                }
                Action::DeclareBlockers { assignments }
            }
        }
    }
}
