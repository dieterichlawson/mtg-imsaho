# Code Simplicity & Cleanup Report (2026-03-29)

## 1. Complexity Hotspots

### 1.1 engine.rs -- Bloated Action Handler & Legal Action Generation (CRITICAL)

**File:** `mtg-engine/src/engine.rs` (1,400+ lines)

**Issues:**

- `submit_action()` (lines 644-965) is a massive single match statement handling 10 different action types with deeply nested control flow. Mixes mana payment logic, state mutations, target handling, mid-resolution choices, and event generation.

- `legal_actions()` (lines 40-335) contains massive procedural code that:
  - Duplicates logic for mana abilities and activated abilities twice (lines 139-153 and 158-189)
  - Duplicates spell casting logic between regular spells and flashback spells (lines 216-277 vs 279-329)
  - Handles 6 different awaiting action types inline with complex nested matching
  - Has hardcoded oracle text pattern matching (lines 57-59 for "attacks each combat if able")

- Lines 800-872: DeclareAttackers action has hardcoded forced attacker logic checking for auras containing "attacks each combat if able"
- Lines 351-502: `generate_cast_actions_with_targets()` generates Cartesian products for two-target spells

**Cleanup Suggestions:**
1. Extract `Action::ResolveChoice` handling into separate `apply_resolution_choice()`
2. Extract combat actions into `handle_combat_action()`
3. Move forced attacker detection into combat.rs
4. Move spell casting logic to a separate module

**Suggested structure:**
```
engine/
  mod.rs               [150 lines - public API]
  actions.rs           [350 lines - submit_action, related]
  legal_actions.rs     [300 lines - legal_actions, casting]
  game_loop.rs         [200 lines - run_game_loop, advance_step]
  helpers.rs           [100 lines - draw_cards, etc.]
```

### 1.2 state.rs -- Bonus Calculation is Scattered & Text-Based

**File:** `mtg-engine/src/state.rs` (lines 348-502)

- `effective_power()` and `effective_toughness()` iterate through aura bonuses, anthem bonuses, counters, until-end-of-turn effects, and opponent debuffs
- `anthem_power_bonus()` and `anthem_toughness_bonus()` are nearly identical except they call different parsing functions
- Text-based matching anti-patterns:
  - Lines 375-382: Hardcoded check for "with flying" in oracle text
  - Lines 461-469: String contains checks for anthem text
  - Lines 489-496: Same anthem checks duplicated
  - Lines 584-589: Vigilance anthem check using oracle text

**Cleanup Suggestions:**
1. Consolidate `anthem_power_bonus()` + `anthem_toughness_bonus()` into single method returning `(i32, i32)`
2. Replace oracle text checks with structured data in `CardData`
3. Create enum for bonus sources instead of text parsing

### 1.3 Combat Resolution -- Complex Damage Distribution

**File:** `mtg-engine/src/combat.rs` (lines 138-229)

- `deal_damage_step()` has deeply nested control flow
- First strike/double strike conditional logic appears twice
- Trample overflow interleaved with regular damage
- `declare_attackers()` and `declare_attackers_with_registry()` nearly identical

**Cleanup Suggestions:**
1. Extract `DamageStep` struct to eliminate repeated keyword checks
2. Extract trample/lethal calculation into `CombatCalculator`
3. Merge the two `declare_attackers` variants

### 1.4 SBA -- Loop Structure & Classification

**File:** `mtg-engine/src/sba.rs` (lines 14-193)

- Creatures classified twice: once to identify, once to process
- Registry vs non-registry path has different code

**Cleanup Suggestions:**
1. Create `StateBasedAction` enum to represent what to do
2. Use single-pass collection and processing
3. Merge registry and non-registry paths with Option parameter

---

## 2. Special-Case Sprawl

### 2.1 Oracle Text Matching Throughout Engine (HIGH PRIORITY)

Hardcoded oracle text checks that should be card-specific:
- `engine.rs:57-59` -- `"attacks each combat if able"` hardcoded
- `engine.rs:831` -- Same check repeated in DeclareAttackers
- `state.rs:377` -- `"with flying"` hardcoded
- `state.rs:461-469` -- `"Creatures you control get"` and `"Creature tokens you control get"`
- `state.rs:584` -- `"have vigilance"` check
- `combat.rs:192` -- `"doesn't untap"` check
- `combat.rs:236` -- `"Prevent all combat damage"` check
- `combat.rs:245` -- `"protection from"` check

**Impact:** Adding a new mechanic requires changes in 3+ locations across different modules.

**Cleanup Suggestion:** Create `CardBehavior` trait methods to expose capabilities:
```rust
fn forces_attack_each_combat(&self) -> bool { false }
fn grants_vigilance(&self) -> bool { false }
fn prevents_untap(&self) -> bool { false }
fn prevents_damage(&self) -> bool { false }
```

### 2.2 Hardcoded Subtype Filtering

Multiple cards filter targets by subtype using the same verbose pattern. Add helper:
```rust
impl GameState {
    pub fn has_subtype(&self, object_id: ObjectId, subtype: &str, registry: &CardRegistry) -> bool;
}
```

### 2.3 Fiend Hunter Hack: Using `targets` Field for Exile Storage

Fiend Hunter stores exiled creature ID in `targets` field because there's no persistent card-specific state. Need a `card_state: HashMap<String, Value>` on GameObject.

---

## 3. Duplication

### 3.1 27 Cards with Identical Aura Attachment Pattern (~540 LOC)

Cards: pacifism, claustrophobia, dead_weight, sensory_deprivation, spectral_flight, furor_of_the_bitten, gruesome_deformity, skeletal_grimace, bonds_of_faith, ghostly_possession, holy_strength, and more.

All share identical `on_resolve`:
```rust
fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
    if let Some(Target::Object(target_id)) = targets.first() {
        if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.move_object(object_id, Zone::Battlefield);
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*target_id);
                obj.summoning_sick = false;
            }
            return;
        }
    }
    state.move_object(object_id, Zone::Graveyard);
}
```

**Fix:** Shared `resolve_aura()` helper function. Each aura card calls it in 1-2 lines.

### 3.2 Damage-Dealing Spells Duplicated (8+ cards, ~200+ LOC)

Lightning Bolt, Geistflame, Lava Axe, Shock, etc. all share identical target-matching + damage logic.

**Fix:** `resolve_damage_spell(state, spell_id, targets, amount)` helper. Each card becomes ~3 lines.

### 3.3 Destruction-Based Spells (8+ cards, ~150+ LOC)

Doom Blade, Victim of Night, Murder, etc. all share identical `try_destroy` boilerplate.

**Fix:** `resolve_destroy_spell(state, spell_id, targets, registry)` helper.

---

## 4. Abstraction Level Issues

### 4.1 Too Many Layers of Indirection in Bonus Calculation

Current path: `effective_power()` -> `aura_power_bonus()` -> `parse_aura_power_bonus()` -> `parse_plus_minus()` -> string splitting.

Better: P/T modifications stored as structured data, not parsed from text.

### 4.2 legal_actions() Returns Triplicated Data

Returns `Vec<Action>` + `Option<CombatPrompt>` + `Vec<CastableSpell>`. Should be a single unified `Vec<LegalAction>` with categories.

---

## 5. Error Handling Issues

### 5.1 41 `.unwrap()` Calls Across Engine Code

Locations:
- **engine.rs (18 unwraps):** Lines 650, 657, 676, 684, 685, 689, 697, 717, 725, 744, 745, 769, 1040, 1065, 1206, 1218, 1287, 1440
- **state.rs (4 unwraps):** All in tests
- **sba.rs (4 unwraps)**
- **combat.rs (9 unwraps)**
- **cards/*.rs (5 unwraps)**

Problematic ones:
- `new_state.priority_player.unwrap()` -- panic if None
- `new_state.get_object(*object_id).unwrap().card_id` -- panic if object removed
- `registry.get(card_id).unwrap().card_data()` -- panic if card not in registry

**Fix:** Replace with Result-based error handling in `submit_action`.

---

## 6. Naming Inconsistencies

| Current Name | Issue | Better Name |
|---|---|---|
| `priority_player` | Can be None | `priority_holder: Option<PlayerId>` |
| `summoning_sick` | Only creatures get this | `entered_this_turn: bool` |
| `consecutive_passes` | Unclear what it counts | `priority_passes_by_all_players: u32` |
| `dealt_deathtouch_damage` | Misleading | `damaged_by_deathtouch: bool` |
| `can_be_targeted()` | Only checks hexproof | `can_be_targeted_by_opponent()` |

---

## 7. Dead Code & Incomplete Work

### Orphaned Functions in combat.rs (lines 232-250)
- `has_damage_prevention()` -- exists but not integrated
- `has_protection_from()` -- exists but not called
- `is_subtype()` -- function stub

### Limited Test Helpers
`common/mod.rs` is only 33 lines. Could expand with `spell_in_hand()`, `pay_mana_for_spell()`, etc.

---

## 8. Priority Cleanup Recommendations (Ranked by Impact)

| Priority | Item | Impact | Effort |
|----------|------|--------|--------|
| CRITICAL | Split engine.rs from 1400 to <700 lines | Maintainability | High |
| CRITICAL | Replace 27 aura boilerplate with shared helper | -540 LOC | Low |
| CRITICAL | Replace .unwrap() with Result types | Reliability | High |
| HIGH | Move oracle text checks to CardBehavior methods | Flexibility | High |
| HIGH | Add resolve_damage_spell() helper | -200+ LOC | Low |
| HIGH | Add resolve_destroy_spell() helper | -150+ LOC | Low |
| MEDIUM | Extract bonus calculation to state/bonuses.rs | Clarity | High |
| MEDIUM | Consolidate anthem power/toughness | -100 LOC | Low |
| MEDIUM | Add persistent card state to GameObject | Architecture | Medium |
| LOW | Expand test helpers | DX | Low |
| LOW | Rename confusing field names | Clarity | Low |
