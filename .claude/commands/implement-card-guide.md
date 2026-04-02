# Card Implementation Guide

This is the reference guide for implementing MTG cards in this engine. Read this file in full before writing any code.

## Principles

- **Do it right.** Do not take shortcuts. Do not simplify. Implement the MTG rules exactly as written in the oracle text.
- **Reference the true current oracle text.** The oracle text provided to you is fetched from Scryfall and is your single source of truth. Do not rely on your training data for card text, types, rulings, or any other card data.
- **Do not be afraid of large engine changes.** If implementing the card correctly requires adding new fields to actions, new resolution choice kinds, new hooks on CardBehavior, new state fields, new SBA checks, or new event types — do it. The engine should serve the cards, not the other way around.
- **Do not choose the simplest solution because it is simple or expedient.** Do the right thing that will generalize and be reusable by other cards.
- **Do not skip features because of "engine limitations" or "UI limitations" or other "known limitations."** Fix those limitations. If the engine can't express something, extend the engine. If the UI can't present a choice, add the UI support.
- **Player choice is mandatory.** If the oracle text says the player chooses, targets, sacrifices, searches, or discards, that choice MUST be presented to the player. Auto-selecting is a shortcut. The only exceptions are: (1) when there's exactly one legal option, or (2) "target opponent" in a 2-player game (unambiguous). Note: "target player" is NOT auto-selectable — the caster can target themselves or their opponent.
- **Engine additions must be generic.** When you add a new mechanism to the engine, design it as a generic facility that any card can use — not a one-off hack. No `if card_name == "CardName"` in engine code. Use trait methods, enum variants, and data-driven patterns.
- **The only acceptable simplification**: when a card says "target opponent" you can assume there is only one opponent. "Target player" you cannot assume there is only one player or that the target will always be the opponent.

## Project structure

```
mtg-engine/src/cards/           # Card implementations
mtg-engine/src/cards/mod.rs     # CardBehavior trait, CardRegistry, types
mtg-engine/src/cards/isd/       # Innistrad cards
mtg-engine/src/cards/isd/mod.rs # Module declarations for ISD
mtg-engine/src/cards/helpers.rs # Shared helper functions
mtg-engine/src/types.rs         # Core types (Zone, Color, ManaSymbol, ContinuousEffect, etc.)
mtg-engine/src/state.rs         # GameState, PendingEffect, ResolutionChoiceKind
mtg-engine/src/engine.rs        # Game engine, action processing
mtg-engine/src/triggers.rs      # Trigger dispatch system
mtg-engine/src/destruction.rs   # Destruction pipeline
mtg-engine/src/stack.rs         # Stack resolution
mtg-engine/tests/               # Integration tests
mtg-engine/tests/common/mod.rs  # Test helper functions
mtg-player/src/llm.rs           # LLM card knowledge for AI players
```

## How to implement a card

### 1. Create the implementation file

Create `mtg-engine/src/cards/isd/{card_name}.rs` (use snake_case). Define a struct that implements `CardBehavior`.

### 2. Fill in `card_data()`

Every card must return a `CardData` struct. Match the oracle text EXACTLY:

```rust
fn card_data(&self) -> CardData {
    CardData {
        name: "Card Name".into(),
        cost: Some(ManaCost::new(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Colored(Color::Black),
        ])),
        card_types: vec![CardType::Creature],      // From type line
        supertypes: vec![Supertype::Legendary],     // If applicable
        subtypes: vec!["Human".into(), "Cleric".into()], // ALL subtypes from type line
        power: Some(2),
        toughness: Some(3),
        oracle_text: "The full oracle text here.".into(),
        keywords: vec![Keyword::Flying, Keyword::Lifelink], // ALL keywords
        flashback_cost: None,                       // Or Some(ManaCost::new(...))
        continuous_effects: vec![],                 // Static abilities
        additional_cost: None,                      // Sacrifice/exile costs
        triggered_abilities: vec![                  // MUST match implemented hooks
            TriggeredAbilityDef {
                kind: TriggerKind::EntersBattlefield,
                description: "description for the stack".into(),
            },
        ],
    }
}
```

**Critical rules for card_data:**
- `triggered_abilities` MUST have an entry for EVERY hook you implement. If you implement `on_upkeep`, you need `TriggerKind::Upkeep`. If you implement `on_enter_battlefield`, you need `TriggerKind::EntersBattlefield`. Missing entries means the trigger system won't dispatch to your hook.
- `subtypes` must include ALL subtypes from the Scryfall type line. "Vampire Noble" needs both `"Vampire"` and `"Noble"`.
- `keywords` must include ALL keywords the card has (Flying, First Strike, Trample, etc.).
- `continuous_effects` declares static abilities that the engine applies automatically.

### 3. Implement behavior hooks

Only implement the hooks your card needs. Common hooks:

**For spells (instants/sorceries):**
```rust
fn target_requirement(&self) -> TargetRequirement { ... }
fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool { ... }
fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) { ... }
```

**For creatures with ETB:**
```rust
fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) { ... }
```

**For creatures with death triggers:**
```rust
fn on_dies(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) { ... }
```

**For death watchers ("whenever a/another creature dies"):**
```rust
fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, dead_id: ObjectId, registry: &CardRegistry) { ... }
```

**For upkeep triggers:**
```rust
fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) { ... }
```

**For end step triggers:**
```rust
fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) { ... }
```

**For attack triggers:**
```rust
fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) { ... }
fn on_any_creature_attacks(&self, state: &mut GameState, self_id: ObjectId, attacker_id: ObjectId, registry: &CardRegistry) { ... }
```

**For combat damage triggers:**
```rust
fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, player: PlayerId, damage: u32, registry: &CardRegistry) { ... }
fn on_deals_combat_damage_to_creature(&self, state: &mut GameState, self_id: ObjectId, target_id: ObjectId, damage: u32, registry: &CardRegistry) { ... }
```

**For spell cast triggers ("whenever a player casts a spell"):**
```rust
fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId, spell_id: ObjectId, caster: PlayerId, registry: &CardRegistry) { ... }
```

**For activated abilities:**
```rust
fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> { ... }
fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) { ... }
```

**For mana abilities:**
```rust
fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> { ... }
fn on_activate_mana_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize) { ... }
```

**For planeswalkers:**
```rust
fn starting_loyalty(&self) -> Option<u32> { ... }
fn loyalty_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<LoyaltyAbilityDef> { ... }
fn on_loyalty_ability(&self, state: &mut GameState, self_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) { ... }
```

**For DFCs (double-faced cards):**
```rust
fn back_face_data(&self) -> Option<CardData> { ... }
fn should_transform(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> bool { ... }
fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> { ... }
```

**For leave-battlefield triggers:**
```rust
fn on_leave_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) { ... }
```

### 4. Register the card

**In `mtg-engine/src/cards/isd/mod.rs`:**
Add `pub mod card_name;` in alphabetical order.

**In `mtg-engine/src/cards/mod.rs`:**
Add `reg.register(Box::new(isd::card_name::CardName));` in the `with_all_cards()` method, in the appropriate section (sorted by card type/category or alphabetically within the ISD section).

### 5. Write tests

Create or add to a test file in `mtg-engine/tests/`. Use the test helpers from `common/mod.rs`:

```rust
mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

#[test]
fn card_name_does_thing() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Set up board state
    let spell = castable_spell(&mut state, &registry, "Card Name", P0);
    let creature = ready_creature(&mut state, P1, 3, 3);

    // Cast and resolve
    state = cast_and_resolve(&state, &registry, spell, vec![Target::Object(creature)]);

    // Assert effects
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}
```

**Available test helpers:**
- `game_at_step(step, active_player)` — create a game state at a specific step
- `ready_creature(state, owner, power, toughness)` — battlefield creature, no summoning sickness
- `sick_creature(state, owner, power, toughness)` — battlefield creature with summoning sickness
- `spell_in_hand(state, registry, name, player)` — put a named card in hand
- `add_mana_for(state, registry, name, player)` — add exact mana to cast a card
- `castable_spell(state, registry, name, player)` — put card in hand + add mana
- `cast_and_resolve(state, registry, spell_id, targets)` — cast and resolve in one step
- `named_creature(state, registry, name, owner)` — put a named card on battlefield (ready)
- `named_equipment(state, registry, name, owner)` — put a named equipment on battlefield
- `P0`, `P1` — player ID constants

**Test coverage requirements:**
- At least one test for the card's main effect
- For targeted spells: fizzle test (target leaves before resolution)
- For "you may" abilities: test declining
- For triggered abilities: test through the trigger system
- For flashback: cast from graveyard + verify exiled after
- For each ruling from oracle lookup: a test
- Verify mechanism, not just outcome

### 6. Add LLM card knowledge

If the card has strategic significance for AI play, add an entry to the card knowledge section in `mtg-player/src/llm.rs`. Follow the existing format:

```
- Card Name ({cost} creature P/T keyword): Brief strategic description.
```

For complex cards (planeswalkers, cards with multiple abilities), write a longer description explaining how to use each ability and when.

**Important**: The description must be accurate to the oracle text. Do not say "opponent's creature" if the card targets "a creature" (any creature). Do not omit "you may" optionality.

## Key types reference

### TriggerKind — must match your implemented hooks
```
EntersBattlefield    → on_enter_battlefield
SelfDies             → on_dies
AnyCreatureDies      → on_any_creature_dies
AnyCreatureEnters    → on_any_creature_enters
CombatDamageToPlayer → on_combat_damage_to_player
AnyCombatDamageToPlayer → on_any_combat_damage_to_player
AnyDamageToPlayer    → on_any_damage_to_player
Upkeep               → on_upkeep
EndStep              → on_end_step
SpellCast            → on_spell_cast
Attacks              → on_attacks
Blocks               → on_blocks
BecomesBlocked       → on_becomes_blocked
AnyCreatureAttacks   → on_any_creature_attacks
DealsCombatDamageToCreature → on_deals_combat_damage_to_creature
EndCombat            → on_end_combat
LeavesBattlefield    → on_leave_battlefield
```

### TargetRequirement
```
None                          — no targets
AnyTarget                     — creature or player
Creature                      — any creature on battlefield
CreatureWithFilter(filter)    — creature matching a filter
PlayerOnly                    — player only
Spell                         — spell on the stack
PermanentWithFilter(filter)   — permanent matching a filter
TwoTargets(req1, req2)        — two separate targets
UpToTargets(n, req)           — up to N targets
GraveyardCard                 — card in any graveyard
GraveyardCreature             — creature card in any graveyard
GraveyardCardOwnedByCaster    — card in caster's graveyard
ModalChoice(vec)              — choose a mode
```

### TargetFilter
```
Any, YouControl, YouDontControl, Nonblack, NotSubtypes(vec),
PowerAtLeast(n), Attacking, Noncreature, HasCardType(vec),
SubtypeOrCardType { subtypes, card_types }, HasSubtype(string), Another
```

### ContinuousEffect — for static abilities
```
ModifyPT { power, toughness, scope }
GrantKeyword { keyword, scope }
PreventAttack { scope }
PreventBlock { scope }
CantBeBlocked { scope }
BlockRestriction { allowed_blockers, scope }
PreventCombatDamage { scope }
PreventUntap { scope }
ForceAttack { scope }
ProtectionFromSubtype { subtype, scope }
ProtectionFrom { filter, scope }
ConditionalKeyword { keyword, condition, scope }
ReduceCost { reduction, filter }
MinimumBlockers { count, scope }
PreventDamageRemoveCounter { scope }
DoubleCombatDamage { scope }
```

### EffectScope
```
Global       — affects the card itself
GlobalOther  — affects other permanents (not self)
Attached     — affects the enchanted/equipped permanent
YouControl   — affects all creatures you control
YouDontControl — affects creatures you don't control
```

### Helper functions (in `mtg-engine/src/cards/helpers.rs`)

**Resolution helpers:**
- `resolve_aura(state, aura_id, targets)` — attach aura to target
- `resolve_curse(state, curse_id, targets)` — attach curse to player
- `resolve_damage(state, spell_id, targets, amount)` — deal non-combat damage (emits NonCombatDamageDealt + LifeChanged)
- `resolve_destroy(state, spell_id, targets, registry)` — destroy via try_destroy pipeline

**Choice helpers:**
- `present_target_choice(state, source, controller, targets, effect, description, optional)` — present target choice to player
- `present_optional_target_choice(state, source, controller, targets, effect, description)` — "you may" version

**Target collection:**
- `creature_targets(state)` — all battlefield creatures
- `creature_targets_except(state, exclude)` — all creatures except one
- `any_targets(state)` — creatures + all players
- `any_targets_except(state, exclude)` — creatures except one + all players
- `creatures_controlled_by(state, player)` — creatures you control
- `opponent_player(state, controller)` — single opponent target
- `controller_of(state, object_id)` — get controller of permanent

## Common patterns

### Equipment
```rust
fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], _registry: &CardRegistry) {
    state.move_object(object_id, Zone::Battlefield);
    if let Some(obj) = state.get_object_mut(object_id) {
        obj.is_equipment = true;
    }
}

fn activated_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ActivatedAbilityDef> {
    let obj = match state.get_object(object_id) {
        Some(o) => o,
        None => return vec![],
    };
    if obj.zone == Zone::Battlefield && obj.power.is_none() {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Equip {N}".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(N)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
            once_per_turn: false,
            sorcery_speed_only: true,
        }]
    } else {
        vec![]
    }
}

fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
    if let Some(Target::Object(creature_id)) = targets.first() {
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.attached_to = Some(*creature_id);
        }
    }
}
```

### Werewolf DFC (standard transform)
```rust
fn werewolf_should_transform(state: &GameState, object_id: ObjectId) -> bool {
    let is_transformed = state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false);
    let total_spells_last_turn: u32 = state.spells_cast_last_turn.values().sum();
    if !is_transformed {
        total_spells_last_turn == 0 && !state.is_first_turn
    } else {
        state.spells_cast_last_turn.values().any(|&count| count >= 2)
    }
}

fn back_face_data(&self) -> Option<CardData> {
    Some(CardData { /* back face stats */ })
}

fn should_transform(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> bool {
    Self::werewolf_should_transform(state, object_id)
}

fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
    if state.get_object(object_id).map(|o| o.is_transformed).unwrap_or(false) {
        Some((back_power, back_toughness))
    } else {
        None
    }
}
```

### Non-werewolf DFC (conditional/activated transform)
For cards like Screeching Bat, Cloistered Youth, Delver of Secrets — do NOT use the werewolf transform pattern. Instead, implement the specific transform condition from the oracle text and present any "you may" choices to the player.

### Spell cleanup
Always use `state.move_spell_after_resolve(object_id)` for instants/sorceries. NEVER use `state.move_object(object_id, Zone::Graveyard)`.

### Non-combat damage
Use `helpers::resolve_damage()` or manually emit `GameEvent::NonCombatDamageDealt`. NEVER use `CombatDamageDealt` for non-combat damage. Track `damaged_by` on target creatures.

### Dynamic P/T
For creatures whose P/T depends on game state (e.g., "power and toughness equal to the number of cards in your hand"), implement `dynamic_pt()`:
```rust
fn dynamic_pt(&self, state: &GameState, object_id: ObjectId) -> Option<(i32, i32)> {
    let controller = state.get_object(object_id).map(|o| o.controller)?;
    let count = /* compute dynamic value */;
    Some((count as i32, count as i32))
}
```

### Token creation
Use `state.create_token_with_subtypes()` for tokens that need subtypes:
```rust
let token_id = state.create_token_with_subtypes(
    controller,
    power, toughness,
    vec!["Wolf".into()],
);
if let Some(token) = state.get_object_mut(token_id) {
    token.name = "Wolf".into();
    // Set any additional properties (keywords, etc.)
}
```

## Anti-patterns — DO NOT do these

1. **`move_object(id, Zone::Graveyard)`** for spell cleanup — use `move_spell_after_resolve(id)`
2. **`CombatDamageDealt`** for non-combat damage — use `NonCombatDamageDealt`
3. **`obj.power`** for P/T in effects — use `state.effective_power(id, registry)` when you need the actual effective value
4. **`EffectScope::Global`** when `GlobalOther` is needed — "other creatures you control" needs `GlobalOther`
5. **Missing token subtypes** — tokens need subtypes via `create_token_with_subtypes()`
6. **Missing `triggered_abilities`** for implemented hooks — the trigger system won't dispatch without these declarations
7. **`try_destroy`** when oracle says "sacrifice" — sacrifice bypasses indestructible; use the sacrifice path
8. **Checking subtypes only via registry** — tokens don't have registry entries; check `obj.subtypes` too
9. **Auto-selecting choices** — if the oracle text says the player chooses/targets/sacrifices, present the choice
10. **Hardcoding opponents** — "target player" can be any player; only "target opponent" can be auto-selected
11. **Using menace** for a "can only be blocked by two or more creatures" effect when the card doesn't grant menace
12. **Storing delayed trigger data on `card_state`** — lost when permanent leaves; store at game level (e.g., `state.end_of_combat_exiles`)
