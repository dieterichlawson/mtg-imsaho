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
