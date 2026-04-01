# Audit: Moonmist

## Official Oracle
- **Name:** Moonmist
- **Cost:** {1}{G}
- **Type:** Instant
- **Oracle:** Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.

## Implementation: `mtg-engine/src/cards/moonmist.rs`
- **Name:** Moonmist -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Instant -- CORRECT
- **on_resolve:** Transforms Human DFCs, updates characteristics from back face -- CORRECT

## Issues
1. **Combat damage prevention not implemented:** The oracle says "Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves." This is noted in comments as not implemented. This is a significant part of the card's effect.

## Verdict
**FAIL** -- 1 issue: Combat damage prevention for non-Wolf/non-Werewolf creatures is not implemented.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves. (Only double-faced cards can be transformed.)
**Scryfall type line**: Instant
**Status**: PASS

Previous combat damage prevention issue has been fixed. The implementation now sets `state.prevent_non_wolf_werewolf_combat_damage = true` on resolve, which flags the engine to prevent combat damage from non-Wolf/non-Werewolf creatures this turn.

Verified correct:
- Mana cost: {1}{G} -- matches
- Type: Instant -- matches
- Transform logic: transforms all Humans that have a back face (DFCs), updates name/P/T/keywords/subtypes from back face -- correct per reminder text "(Only double-faced cards can be transformed.)"
- Combat damage prevention: sets engine flag for non-Wolf/non-Werewolf prevention -- correct
- `on_resolve` calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/moonmist.rs` and `mtg-engine/tests/innistrad_simple_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Type line**: Instant
**Status**: ISSUE

Card data correct: name, mana cost ({1}{G}), type (Instant).

Transform logic: transforms all Humans with a back face (DFCs only), updates name/P/T/keywords/subtypes from back face. Correct per rulings ("Moonmist causes any double-faced Human to transform, not just Werewolves") and reminder text.

Combat damage prevention: sets state.prevent_non_wolf_werewolf_combat_damage = true. Correct.

on_resolve calls move_spell_after_resolve(object_id). Correct.

Minor issue:
1. The code filters on `!o.is_transformed` which means it only transforms front-face Humans to their back face. The oracle says "Transform all Humans" which should also transform any currently-transformed creature whose back face has the Human subtype back to its front face. In practice this is unlikely to matter in Innistrad (Humans are typically front-face), but it is technically incomplete.

Tests in moonmist.rs cover prevention flag, damage prevention to player/creature, and wolf exception. No test for the transform functionality itself, but the damage prevention tests are thorough.

## Audit — 2026-04-01 14:37

**Oracle text source**: Scryfall via WebSearch (https://scryfall.com/card/isd/195/moonmist)
**Oracle text**: Transform all Humans. Prevent all combat damage that would be dealt this turn by creatures other than Werewolves and Wolves.
**Type line**: Instant
**Status**: ISSUE

Card data verified correct: name, mana cost ({1}{G}), card_types (Instant), oracle_text matches.

on_resolve correctly:
- Transforms Human DFCs by checking for Human subtype and back face existence
- Updates characteristics (name, P/T, keywords, subtypes) from back face data
- Sets `state.prevent_non_wolf_werewolf_combat_damage = true`
- Calls `move_spell_after_resolve(object_id)` (correct for instant)

Issue:

1. **Transform only applies to non-transformed Humans** (`moonmist.rs` line 34).
   - Oracle text says: `Transform all Humans.`
   - Code does: `.filter(|o| o.zone == Zone::Battlefield && !o.is_transformed)` -- the `!o.is_transformed` filter means only front-face (non-transformed) Humans are transformed. If a DFC's back face has the Human subtype and is currently transformed (showing the back face), it would not be transformed back to the front face. The oracle says "Transform all Humans" which means any creature currently with the Human subtype should be transformed, regardless of which face is showing.

No other issues found. Tests in moonmist.rs (4 tests) cover prevention flag, damage prevention to player, wolf exception, and damage prevention to creature. No test for the transform functionality itself.
