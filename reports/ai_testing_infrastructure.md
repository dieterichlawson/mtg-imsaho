# AI Testing Infrastructure Report (2026-03-29)

## Current State

### Player Trait
```rust
pub trait Player {
    fn name(&self) -> &str;
    fn choose_action(&mut self, view: &GameView, legal: &LegalActions) -> Action;
    fn choose_cards_to_bottom(&mut self, view: &GameView, hand: &[CardView], count: usize) -> Vec<ObjectId>;
}
```

### Three Existing Implementations
1. **RandomPlayer** (`mtg-player/src/random.rs`) -- randomly selects from legal actions
2. **LlmPlayer** (`mtg-player/src/llm.rs`, ~860 lines) -- calls Claude or Gemini APIs
3. **CliPlayer** (`mtg-player/src/cli.rs`) -- interactive terminal UI

### No Mock/Fake/Scripted Implementation Exists

### 56 AI Scenario Tests
Located in `mtg-runner/tests/ai_*.rs`:
- `ai_counterspell.rs` (1 test)
- `ai_keywords.rs` (4 tests)
- `ai_mechanics.rs` (4 tests)
- `ai_tier2.rs` (14 tests)
- `ai_tier3.rs` (15 tests)
- `ai_tier4.rs` (18 tests)

All are `#[ignore]`d, require API keys, and call real LLM APIs.

### Test Pattern
Each test:
1. Builds a specific game scenario (board state, hands, life totals)
2. Creates an LlmPlayer
3. Loops: get legal actions -> ask AI -> submit action -> check state
4. Asserts the AI made the correct decision

### What a ScriptedPlayer Needs
Implement `Player` trait with a pre-scripted action queue:
- `choose_action()` pops the next action from the queue
- `choose_cards_to_bottom()` returns pre-scripted cards
- Tests provide the "correct" sequence upfront
- Makes all 56 tests fast, deterministic, and always-run (no `#[ignore]`)

### Action Types the AI Chooses From
```rust
pub enum Action {
    PassPriority,
    PlayLand { object_id },
    CastSpell { object_id, targets },
    ActivateManaAbility { object_id, ability_index },
    ActivateAbility { object_id, ability_index },
    DeclareAttackers { attackers },
    DeclareBlockers { assignments },
    DiscardCards { cards },
    Concede,
    ResolveChoice { choice },
}
```

### Combat is Special-Cased
When `legal.combat_prompt.is_some()`, both LlmPlayer and RandomPlayer use a separate `choose_combat()` method. ScriptedPlayer will need the same.

### LegalActions Structure
```rust
pub struct LegalActions {
    pub actions: Vec<Action>,
    pub combat_prompt: Option<CombatPrompt>,
    pub castable_spells: Vec<CastableSpell>,
}
```
