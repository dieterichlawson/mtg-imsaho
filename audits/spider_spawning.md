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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.\nFlashback {6}{B}
**Mana cost**: {4}{G}
**Type line**: Sorcery
**Status**: PASS
### Checks
- **Name**: "Spider Spawning" -- CORRECT
- **Mana cost**: Generic(4) + Green -- CORRECT ({4}{G})
- **Type**: Sorcery -- CORRECT
- **Flashback cost**: Generic(6) + Black -- CORRECT ({6}{B})
- **on_resolve**: Counts creature cards in controller's graveyard (excluding self on stack), creates 1/2 green Spider tokens with Reach and "Spider" subtype -- CORRECT
- **Token**: 1/2, Green, Creature, Reach, Spider subtype -- CORRECT
- **move_spell_after_resolve**: Called to handle graveyard/exile -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
