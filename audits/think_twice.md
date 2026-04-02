# Audit: Think Twice

## Oracle (Scryfall)
- **Name:** Think Twice
- **Cost:** {1}{U}
- **Type:** Instant
- **Oracle:** Draw a card. Flashback {2}{U}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/think_twice.rs`
- **Name:** Think Twice ✅
- **Cost:** {1}{U} ✅
- **Type:** Instant ✅
- **on_resolve:** draws 1 card for controller ✅
- **Flashback:** {2}{U} ✅
- **Spell cleanup:** move_spell_after_resolve ✅

## Verdict: PASS -- no issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Draw a card.\nFlashback {2}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Mana Cost**: {1}{U}
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Think Twice", cost {1}{U}, type Instant, flashback cost {2}{U}. on_resolve draws one card for controller and moves spell after resolve. All correct.
