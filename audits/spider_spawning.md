# Audit: Spider Spawning

## Oracle (Scryfall)
- **Name:** Spider Spawning
- **Cost:** {4}{G}
- **Type:** Sorcery
- **Oracle:** Create a 1/2 green Spider creature token with reach for each creature card in your graveyard. Flashback {6}{B}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/spider_spawning.rs`
- **Name:** Spider Spawning ✅
- **Cost:** {4}{G} ✅
- **Type:** Sorcery ✅
- **Flashback:** {6}{B} ✅
- **on_resolve:** counts creature cards in graveyard (owner == controller, power.is_some(), excludes self on stack) ✅
- **Token:** 1/2 green Spider with Reach, subtypes ["Spider"] ✅
- **Spell cleanup:** move_spell_after_resolve ✅

## Verdict: PASS -- no issues found
