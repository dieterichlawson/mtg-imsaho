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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player\nCreatures enchanted player controls get -1/-1.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found. Card data matches: name, cost {3}{B}{B}, type Enchantment, subtypes Aura Curse, oracle text. Continuous effect ModifyPT -1/-1 with scope Global(CreatureFilter::AttachedPlayer) correctly targets creatures the enchanted player controls. Resolves via resolve_curse helper. Target requirement is PlayerOnly as expected.

## Audit — 2026-04-02 20:45
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/94/curse-of-deaths-hold)
**Oracle text**: "Enchant player\nCreatures enchanted player controls get -1/-1."
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
None.

### Tricky interactions checked (min 3)
1. **1/1 creatures dying to SBA under the curse**: `sba.rs` uses `effective_toughness` (which includes continuous -1/-1 from the curse) and puts creatures with toughness <= 0 into the graveyard per Rule 704.5f. Correct.
2. **Multiple Curse of Death's Hold stacking**: `continuous_pt_mods` in `state.rs` iterates all battlefield objects and sums applicable ModifyPT effects. Two curses on the same player would give -2/-2. Correct.
3. **Curse can target self (caster)**: No `is_valid_target` override restricts targeting to opponents. The default `PlayerOnly` allows targeting any player, which is correct per oracle text ("Enchant player", not "Enchant opponent").
4. **Controller vs. enchanted player distinction**: `AttachedPlayer` filter checks `creature.controller == attached_player`, not `creature.controller != source.controller`. If the curse changes controllers (e.g., via Donate), it still debuffs the originally enchanted player's creatures, not the new controller's. Correct.

### Test coverage
- `curse_of_deaths_hold_debuffs_opponent_creatures` in `mtg-engine/tests/tier7_cards.rs`: Creates curse attached to P1, verifies P1's 3/3 becomes 2/2, P0's 3/3 stays 3/3. Passes.
