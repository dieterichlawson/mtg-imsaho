# Audit: Mausoleum Guard

## Official Oracle
- **Name:** Mausoleum Guard
- **Cost:** {3}{W}
- **Type:** Creature — Human Scout
- **Oracle:** When Mausoleum Guard dies, create two 1/1 white Spirit creature tokens with flying.
- **P/T:** 2/2

## Implementation: `mtg-engine/src/cards/mausoleum_guard.rs`
- **Name:** Mausoleum Guard -- CORRECT
- **Cost:** {3}{W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Human, Scout -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Triggered ability:** SelfDies -- CORRECT
- **on_dies:** Creates two 1/1 white Spirit tokens with flying -- CORRECT

## Issues
1. **Token subtypes missing:** Uses `create_token("Spirit", ...)` which passes empty subtypes vec. The Spirit tokens will not have the "Spirit" creature subtype. Should use `create_token_with_subtypes` with `vec!["Spirit".into()]`.

## Verdict
**FAIL** -- 1 issue: Spirit tokens lack "Spirit" creature subtype.

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: When Mausoleum Guard dies, create two 1/1 white Spirit creature tokens with flying.
**Scryfall type line**: Creature -- Human Scout
**Status**: PASS

Findings:
1. **Mana cost {3}{W}**: Correct.
2. **Type (Creature -- Human Scout)**: Correct. Subtypes `["Human", "Scout"]`.
3. **P/T 2/2**: Correct.
4. **Oracle text**: Matches Scryfall (uses "this creature" vs card name, both acceptable).
5. **Dies trigger**: Correctly declared with `triggered_abilities: [TriggerKind::SelfDies]` and implemented in `on_dies`.
6. **Token creation**: Uses `create_token_with_subtypes("Spirit", ..., vec![Keyword::Flying], vec!["Spirit".into()])`. Previous audit said Spirit subtype was missing, but the current code uses `create_token_with_subtypes` with the subtype. This is now correct.
7. **Controller tracking**: Uses `o.controller` (not `o.owner`) for token creation -- correct for stolen creatures.
8. **No anti-patterns detected**: Uses `on_dies` hook (not `on_resolve` for a creature dying). Triggered ability properly declared.
9. **Tests**: Found in `mtg-engine/tests/tier3_cards.rs`.

No new issues found. Previous token subtype issue appears resolved.
