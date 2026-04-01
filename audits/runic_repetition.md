# Audit: Runic Repetition

## Official Oracle
- **Name:** Runic Repetition
- **Cost:** {2}{U}
- **Type:** Sorcery
- **Oracle Text:** Return target exiled card with flashback you own to your hand.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{U} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Return target exiled card you own with flashback to your hand." — minor wording order difference from official "Return target exiled card with flashback you own to your hand." — functionally identical — OK
- **P/T:** N/A — OK
- **on_resolve:** Auto-selects a card in exile owned by controller with flashback, moves to hand — functionally correct but no targeting

## Issues
1. **ISSUE: No targeting — auto-selects**: The card says "target exiled card with flashback you own" but the implementation auto-selects the first matching card rather than letting the player choose which exiled flashback card to return. The player should have a choice when multiple candidates exist.

## Verdict: FAIL
- **Targeting is bypassed** — player should choose which exiled flashback card to return

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Return target exiled card with flashback you own to your hand.
**Scryfall type line**: Sorcery
**Status**: PASS

Previous targeting issue has been fixed. The implementation now uses `TargetRequirement::ExileCard` and implements `is_valid_target` to check that the target is in exile, owned by the caster, and has flashback. The player can now choose which exiled flashback card to return.

Verified correct:
- Mana cost: {2}{U} -- matches
- Type: Sorcery -- matches
- Oracle text: minor word order difference ("card you own with flashback" vs "card with flashback you own") but functionally identical
- `on_resolve`: moves target from exile to hand, then calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/innistrad_simple_cards.rs`
