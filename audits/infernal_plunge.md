# Audit: Infernal Plunge

## Oracle (Official)
- **Name:** Infernal Plunge
- **Cost:** {R}
- **Type:** Sorcery
- **Oracle:** As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}.
- **P/T:** N/A

## Implementation
- Name: "Infernal Plunge" -- CORRECT
- Cost: {R} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- additional_cost: SacrificeCreature -- CORRECT
- Adds {R}{R}{R} to mana pool -- CORRECT
- SIMPLIFICATION noted: sacrifice happens on resolution rather than during casting -- ACKNOWLEDGED

## Issues
1. **ISSUE (minor/simplification):** The sacrifice is performed at resolution instead of as a casting cost. Comment acknowledges this. In real MTG, the creature would be sacrificed as part of casting before the spell goes on the stack.

## Verdict: PASS (with noted simplification)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}.
**Scryfall type line**: Sorcery
**Status**: PASS

Findings:
- Mana cost {R}: correct.
- Type Sorcery: correct.
- P/T N/A: correct.
- additional_cost: SacrificeCreature: correct.
- on_resolve adds 3 red mana to controller's pool: correct.
- Anti-pattern check: uses `move_spell_after_resolve(object_id)` (line 41): correct, not the bad `move_object(id, Zone::Graveyard)` pattern.
- No CombatDamageDealt misuse (card deals no damage).
- No triggered_abilities declared, none needed: correct.
- Tests found in infernal_plunge.rs and tier8_cards.rs.
- Carried forward: sacrifice timing simplification (happens at cast time via engine, not in on_resolve). Comment in code acknowledges this correctly.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/148/infernal-plunge)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Add {R}{R}{R}.
**Type line**: Sorcery
**Status**: PASS

Findings:
- Mana cost {R}: correct.
- Type Sorcery: correct.
- P/T N/A: correct.
- additional_cost: SacrificeCreature: correct.
- on_resolve adds 3 red mana via `state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3)`: correct.
- Uses move_spell_after_resolve(object_id) (line 41): correct, no anti-pattern.
- No CombatDamageDealt misuse (card deals no damage).
- No triggered_abilities declared, none needed: correct.
- Tests: 4 tests in infernal_plunge.rs (cannot_cast_without_creature, can_cast_with_creature, sacrifice_at_cast_time, adds_three_red_mana, one_action_per_sacrifice_target) plus test in tier8_cards.rs. Good coverage.

## Audit (2026-04-02)

### Oracle Text (Scryfall)
- **Name:** Infernal Plunge
- **Cost:** {R}
- **Type:** Sorcery
- **Oracle Text:** As an additional cost to cast this spell, sacrifice a creature. / Add {R}{R}{R}.

### Implementation: `mtg-engine/src/cards/isd/infernal_plunge.rs`

#### Card Data
- **name:** "Infernal Plunge" — correct.
- **cost:** `{R}` — correct.
- **card_types:** `[Sorcery]` — correct.
- **supertypes/subtypes:** empty — correct.
- **oracle_text:** `"As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}."` — matches oracle.
- **additional_cost:** `Some(AdditionalCost::SacrificeCreature)` — correct. Engine enforces sacrifice at cast time via `SacrificeCreature` variant in `engine.rs`.
- **keywords, power, toughness, flashback_cost, continuous_effects, triggered_abilities:** all empty/None — correct for a sorcery with no extras.

#### on_resolve
- Gets controller from the object — correct.
- Adds 3 red mana: `state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3)` — correct, matches "Add {R}{R}{R}".
- Calls `state.move_spell_after_resolve(object_id)` — correct. Sends to graveyard (or exile if flashback).

#### Additional Cost Mechanism
- `AdditionalCost::SacrificeCreature` in `engine.rs` requires at least one creature on battlefield to generate cast actions, presents each creature as a sacrifice option, and sacrifices the chosen creature when the spell is cast (before resolution). This correctly implements "As an additional cost to cast this spell, sacrifice a creature."

### Tests
- `tier8_cards::infernal_plunge_sacrifices_and_adds_rrr` — verifies creature goes to graveyard and 3 red mana is added. Passes.

### Verdict
**PASS** — No issues found. The card data, additional cost, mana production, and spell cleanup all match the oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name, mana cost {R}, Sorcery. Additional cost SacrificeCreature correctly set (creature sacrifice happens at cast time). On resolve, adds 3 Red mana to controller's mana pool via mana_pool.add(ManaType::Red, 3). move_spell_after_resolve called. No anti-patterns.

## Audit — 2026-04-03 07:04

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/148/infernal-plunge)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}.
**Type line**: Sorcery
**Status**: PASS

### Code issues
None found.

- Name "Infernal Plunge": matches oracle.
- Mana cost {R}: matches oracle.
- Type Sorcery: matches oracle.
- Oracle text in code: `"As an additional cost to cast this spell, sacrifice a creature.\nAdd {R}{R}{R}."` -- matches oracle exactly.
- `additional_cost: Some(AdditionalCost::SacrificeCreature)`: correctly models the sacrifice-a-creature additional cost.
- `on_resolve` adds 3 red mana via `mana_pool.add(ManaType::Red, 3)`: correct.
- `move_spell_after_resolve(object_id)`: correct (no anti-pattern of raw `move_object` to graveyard).
- No supertypes, subtypes, keywords, power, toughness: correct for a sorcery.
- Engine correctly enforces sacrifice at cast time (before spell goes on stack), not at resolution. Verified in engine.rs lines 1541-1566.
- Engine correctly generates one CastSpell action per eligible creature sacrifice target.
- Engine correctly prevents casting when no creatures are controlled.

### Tricky interactions checked (min 3)
1. **Sacrifice timing with counterspell**: If Infernal Plunge is countered, the creature is already sacrificed (at cast time) and the {R}{R}{R} is never produced. The implementation is correct: sacrifice is in the cast action, mana addition is in `on_resolve`.
2. **No creatures = can't cast**: Engine's `legal_actions` checks for at least one creature on the battlefield before generating CastSpell actions. Test `cannot_cast_without_creature` confirms.
3. **Multiple sacrifice candidates**: Each eligible creature generates a distinct CastSpell action with `sacrifice: Some(creature_id)`, giving the player a real choice. Test `one_action_per_sacrifice_target` confirms with 2 creatures.
4. **Flashback interaction**: `move_spell_after_resolve` checks `cast_with_flashback` and exiles instead of sending to graveyard when appropriate. Correct.

### Test coverage
5 tests in `mtg-engine/tests/infernal_plunge.rs`, all passing:
- `cannot_cast_without_creature` -- gating on creature presence
- `can_cast_with_creature` -- cast allowed with creature
- `sacrifice_at_cast_time` -- sacrifice happens during cast, not resolution
- `adds_three_red_mana` -- verifies {R}{R}{R} added on resolution
- `one_action_per_sacrifice_target` -- each creature is a separate sacrifice option
