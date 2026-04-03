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

## Audit — 2026-04-02

**Oracle text source**: Scryfall API via `scripts/oracle_lookup.py` (cached 2026-04-01)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature. Draw two cards.
**Type line**: Instant
**Mana cost**: {1}{B}
**Status**: PASS

### Card Data
- Name "Altar's Reap": correct.
- Mana cost `Generic(1), Colored(Color::Black)` = {1}{B}: correct.
- Type `CardType::Instant`: correct.
- Oracle text string matches Scryfall verbatim: correct.
- `additional_cost: Some(AdditionalCost::SacrificeCreature)`: correct.

### Behavior Audit
- **Sacrifice timing**: Contrary to the note in the first audit entry (which claimed sacrifice happens on resolution), the current engine code in `engine.rs` (lines ~1356-1377) performs the sacrifice at cast time as an additional cost, before the spell goes on the stack. This is correct per MTG rules. The engine checks for eligible creatures in the cast-legality logic (line ~512-519) and refuses the cast if no creature is available.
- **Draw effect**: `on_resolve` calls `crate::engine::draw_cards(state, controller, 2)` -- draws exactly 2 cards. Correct.
- **move_spell_after_resolve**: Present at line 42. Correctly moves the spell to graveyard (or exile if flashback). No anti-pattern.
- **Controller fallback**: `unwrap_or(crate::ids::PlayerId(0))` on line 37 is a safe fallback; the object should always exist at resolution time.

### Test Coverage (`mtg-engine/tests/tier8_cards.rs`, line 169)
- `altars_reap_sacrifices_and_draws_two`: Sets up a creature and 3 library cards, casts and resolves spell, verifies creature is in graveyard and 2 cards drawn to hand. Adequate for core functionality.

### LLM Player
- No special handling in `mtg-player/src/llm.rs`. None needed -- engine handles the additional cost automatically.

### Correction to Prior Audits
- The first audit entry states "The sacrifice happens on resolution rather than as part of casting." This appears to be outdated. The current engine code performs the sacrifice at cast time via the `AdditionalCost::SacrificeCreature` path in `engine.rs`.

### Verdict
PASS -- No mismatches between oracle text and implementation.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.\nDraw two cards.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01, URL: https://scryfall.com/card/isd/86/altars-reap
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Type line**: Instant
**Mana cost**: {1}{B}
**Status**: PASS

### Code issues
No issues found.

### Checklist
- Mana cost: Oracle says `{1}{B}`. Code has `Generic(1), Colored(Color::Black)`. MATCH.
- Card types: Oracle says "Instant". Code has `vec![CardType::Instant]`. MATCH.
- Supertypes: None in type line. Code has `vec![]`. MATCH.
- Subtypes: None in type line. Code has `vec![]`. MATCH.
- Power/toughness: N/A (Instant). Code has `None, None`. MATCH.
- Keywords: None. Code has `vec![]`. MATCH.
- Oracle text field: Code matches Scryfall verbatim. MATCH.
- Additional cost: Oracle says "sacrifice a creature". Code has `Some(AdditionalCost::SacrificeCreature)`. MATCH.
- Triggered abilities: None needed. Code has `vec![]`. MATCH.
- `on_resolve`: Calls `draw_cards(state, controller, 2)` to draw two cards. Sacrifice is handled at cast time by the engine's additional cost system. CORRECT.
- Spell cleanup: Uses `move_spell_after_resolve(object_id)` (line 42). CORRECT (no anti-pattern).
- No targeting (spell has no targets): CORRECT.
- No `CombatDamageDealt` misuse: CORRECT (card deals no damage).

### Tricky interactions checked
- Sacrifice happens at cast time (additional cost), not on resolution: PASS (engine handles via `AdditionalCost::SacrificeCreature` path in engine.rs lines ~530-537, ~1541-1546)
- Cannot cast without a creature to sacrifice: PASS (engine checks for eligible creatures at lines ~530-536 and skips generating cast action if none available)
- Spell has no targets so cannot fizzle due to invalid targets: PASS
- Sacrifice bypasses indestructible: PASS (engine uses `destruction::sacrifice`, not `try_destroy`)

### Test coverage
- Main effect (sacrifice creature + draw 2): `mtg-engine/tests/tier8_cards.rs:169` (`altars_reap_sacrifices_and_draws_two`)
- Cannot cast without creatures: NOT TESTED (engine prevents it at action generation level)
- Ruling: must sacrifice exactly one creature: Implicitly tested (engine's `AdditionalCost::SacrificeCreature` enforces exactly one)
- Ruling: sacrifice at cast time cannot be responded to: Implicitly correct (engine pays costs before spell goes on stack)

## Audit — 2026-04-02 20:28

**Oracle text source**: Scryfall API via `scripts/oracle_lookup.py` (cached 2026-04-01), URL: https://scryfall.com/card/isd/86/altars-reap
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Draw two cards.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Sacrifice at cast time (not resolution): PASS -- Engine's `AdditionalCost::SacrificeCreature` path in engine.rs (lines ~1541-1546) performs sacrifice before spell goes on stack. The `on_resolve` correctly only draws cards.
- Cannot cast without a creature to sacrifice: PASS -- Engine checks for eligible creatures at lines ~530-536 and skips generating cast actions if none available (`if creatures.is_empty() { continue; }`).
- Spell has no targets, cannot fizzle due to invalid targets: PASS -- No `Target` usage in code; `_targets` parameter is unused.
- Sacrifice bypasses indestructible/regeneration: PASS -- Engine uses `destruction::sacrifice()` (not `try_destroy`), which correctly cannot be prevented.
- Spell cleanup uses correct pattern: PASS -- Uses `move_spell_after_resolve(object_id)` (line 42), which handles flashback exile correctly.

### Test coverage
- Main effect (sacrifice creature + draw 2): `mtg-engine/tests/tier8_cards.rs:169` (`altars_reap_sacrifices_and_draws_two`)
- Cannot cast without creatures on battlefield: NOT TESTED
- Ruling: must sacrifice exactly one creature: NOT TESTED (enforced by engine's `AdditionalCost::SacrificeCreature` allowing exactly one selection)
