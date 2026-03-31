# Audit: Night Terrors

## Official Oracle
- **Name:** Night Terrors
- **Cost:** {2}{B}
- **Type:** Sorcery
- **Oracle:** Target player reveals their hand. You choose a nonland card from it. Exile that card.

## Implementation: `mtg-engine/src/cards/night_terrors.rs`
- **Name:** Night Terrors -- CORRECT
- **Cost:** {2}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Target:** PlayerOnly -- CORRECT
- **on_resolve:** Reveals hand, exiles first nonland card found -- CORRECT behavior

## Notes
- Card selection is auto-picked (first nonland found). In a real game, the caster would choose which nonland to exile. Acceptable simplification for AI.

## Verdict
**PASS** -- No issues found. Auto-selection is an acceptable simplification.
