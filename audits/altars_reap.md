# Audit: Altar's Reap

## Reference (Scryfall/API)
- **Name:** Altar's Reap
- **Mana Cost:** {1}{B}
- **Type:** Instant
- **Oracle:** As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
- **P/T:** N/A

## Implementation: `altars_reap.rs`
- **Name:** Altar's Reap -- CORRECT
- **Mana Cost:** {1}{B} -- CORRECT
- **Type:** Instant -- CORRECT
- **Additional cost:** SacrificeCreature -- CORRECT
- **Effect:** Draw 2 cards -- CORRECT

## Issues
1. **ISSUE (minor/known):** The sacrifice happens on resolution rather than as part of casting. Code has a comment acknowledging this simplification: "the engine doesn't yet support multi-step casting with additional costs." The spell also selects the creature to sacrifice automatically rather than letting the player choose. Additionally, if no creature is available at resolution time, it still fizzles (which is correct behavior since you shouldn't have been able to cast it without the cost, but this is a consequence of the simplification).

## Verdict: PASS (with known simplification) -- Sacrifice timing is noted as simplified

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
**Scryfall type line**: Instant
**Status**: PASS

Findings:
- Mana cost {1}{B}: correct.
- Type Instant: correct.
- P/T N/A: correct.
- additional_cost: SacrificeCreature: correct.
- on_resolve draws 2 cards via `crate::engine::draw_cards(state, controller, 2)`: correct.
- Anti-pattern check: uses `move_spell_after_resolve(object_id)` (line 42): correct, not the bad `move_object(id, Zone::Graveyard)` pattern.
- No CombatDamageDealt misuse (card deals no damage).
- No triggered_abilities declared, none needed: correct.
- Tests found in tier8_cards.rs.
- Carried forward: sacrifice timing simplification (acknowledged in code comments).

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/86/altars-reap)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
**Type line**: Instant
**Status**: PASS

Findings:
- Mana cost {1}{B}: correct.
- Type Instant: correct.
- P/T N/A: correct.
- additional_cost: SacrificeCreature: correct.
- on_resolve draws 2 cards via crate::engine::draw_cards(state, controller, 2): correct.
- Uses move_spell_after_resolve(object_id) (line 42): correct, no anti-pattern.
- No CombatDamageDealt misuse (card deals no damage).
- No triggered_abilities declared, none needed: correct.
- Tests: 1 test in tier8_cards.rs (altars_reap_sacrifices_and_draws_two). Minimal coverage but tests core functionality (sacrifice + draw 2).
