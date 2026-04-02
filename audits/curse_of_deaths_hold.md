# Audit: Curse of Death's Hold

## Oracle Text (Scryfall)
```
Enchant player
Creatures enchanted player controls get -1/-1.
```
- **Mana Cost:** {3}{B}{B}
- **Type Line:** Enchantment — Aura Curse

## Implementation Summary
File: `mtg-engine/src/cards/isd/curse_of_deaths_hold.rs`

## Checklist

### Card Data
- [x] **Name:** `"Curse of Death's Hold"` — correct
- [x] **Mana cost:** `Generic(3), Black, Black` — matches `{3}{B}{B}`
- [x] **Card types:** `Enchantment` — correct
- [x] **Subtypes:** `["Aura", "Curse"]` — correct
- [x] **Oracle text:** matches oracle verbatim
- [x] **Power/toughness:** `None` — correct (not a creature)

### Enchant Player / Curse Mechanics
- [x] **Target requirement:** `TargetRequirement::PlayerOnly` — correct for "Enchant player"
- [x] **on_resolve:** calls `crate::cards::helpers::resolve_curse(state, object_id, targets)` which attaches to target player via `attached_to_player` and moves to battlefield — correct

### Continuous Effect: -1/-1 to Enchanted Player's Creatures
- [x] **Effect:** `ContinuousEffect::ModifyPT { power: -1, toughness: -1, scope: EffectScope::Global(CreatureFilter::AttachedPlayer) }` — correct
- [x] **Scope:** `EffectScope::Global(CreatureFilter::AttachedPlayer)` uses the `attached_to_player` field on the source object to determine which player's creatures are affected. Verified in `state.rs` (line ~635): when `AttachedPlayer` filter is detected within `EffectScope::Global`, the engine looks up `source.attached_to_player` and applies the effect to creatures controlled by that player. This correctly implements "creatures enchanted player controls".
- [x] **Does NOT affect controller's own creatures** — verified: the `AttachedPlayer` filter checks `creature.controller == attached_player`, not `creature.controller != source_controller`, so it only targets the cursed player's creatures regardless of who controls the curse.

### Tests
- [x] **Test exists:** `curse_of_deaths_hold_debuffs_opponent_creatures` in `mtg-engine/tests/tier7_cards.rs` (line 197)
  - Creates curse attached to P1, verifies P1's 3/3 creature becomes 2/2, and P0's 3/3 creature stays 3/3.

### LLM Player
- [x] No special-case handling in `mtg-player/src/llm.rs` — none needed (continuous effect is handled by the engine).

## Issues
None

## Verdict
**PASS** — No issues found. The implementation correctly matches the oracle text. The continuous -1/-1 effect properly targets only creatures controlled by the enchanted player via the `AttachedPlayer` filter, the curse attachment mechanics are correct, and test coverage validates the behavior.
