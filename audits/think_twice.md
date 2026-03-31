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
