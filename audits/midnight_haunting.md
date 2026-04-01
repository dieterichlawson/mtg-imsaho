# Audit: Midnight Haunting

## Official Oracle
- **Name:** Midnight Haunting
- **Cost:** {2}{W}
- **Type:** Instant
- **Oracle:** Create two 1/1 white Spirit creature tokens with flying.

## Implementation: `mtg-engine/src/cards/midnight_haunting.rs`
- **Name:** Midnight Haunting -- CORRECT
- **Cost:** {2}{W} -- CORRECT
- **Type:** Instant -- CORRECT
- **on_resolve:** Creates two 1/1 white Spirit tokens with flying -- CORRECT

## Issues
1. **Token subtypes missing:** Uses `create_token("Spirit", ...)` which passes empty subtypes vec. The Spirit tokens will not have the "Spirit" creature subtype. Should use `create_token_with_subtypes` with `vec!["Spirit".into()]`.

## Verdict
**FAIL** -- 1 issue: Spirit tokens lack "Spirit" creature subtype.

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: Create two 1/1 white Spirit creature tokens with flying.
**Scryfall type line**: Instant
**Status**: PASS

Findings:
1. **Mana cost {2}{W}**: Correct.
2. **Type (Instant)**: Correct.
3. **Oracle text**: Matches Scryfall.
4. **Token creation**: Uses `create_token_with_subtypes("Spirit", ..., vec![Keyword::Flying], vec!["Spirit".into()])`. Previous audit said Spirit subtype was missing, but the current code uses `create_token_with_subtypes` with the subtype. This is now correct.
5. **Spell cleanup**: Uses `state.move_spell_after_resolve(object_id)` (line 34) -- correct, not the anti-pattern `move_object(id, Zone::Graveyard)`.
6. **No anti-patterns detected**.
7. **Tests**: Found in `mtg-engine/tests/tier3_cards.rs`.

No new issues found. Previous token subtype issue appears resolved.
