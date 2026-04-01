# Audit: Creeping Renaissance

## Scryfall Reference
- **Name:** Creeping Renaissance
- **Cost:** {3}{G}{G}
- **Type:** Sorcery
- **Oracle:** Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand. Flashback {5}{G}{G}
- **P/T:** N/A
- **Keywords:** Flashback

## Implementation: `creeping_renaissance.rs`
- **Name:** Creeping Renaissance -- CORRECT
- **Cost:** {3}{G}{G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Subtypes:** none -- CORRECT
- **P/T:** N/A -- CORRECT
- **Flashback:** {5}{G}{G} -- CORRECT
- **Behavior:** Returns cards of chosen type from graveyard to hand -- SIMPLIFIED (always chooses "creature")

## Issues
1. **ISSUE: Hardcoded to "creature" permanent type.** The oracle text says "Choose a permanent type" (which could be creature, artifact, enchantment, land, or planeswalker). The implementation always auto-chooses "creature" without giving the player a choice. This is documented as a simplification in the code comment.

## Audit — 2026-04-01 08:07

**Scryfall Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand. Flashback {5}{G}{G}
**Scryfall type line**: Sorcery
**Status**: PASS

Previous issue (hardcoded creature type) has been fixed. The implementation now uses ChooseCardType resolution choice system presenting all 5 permanent types (Creature, Artifact, Enchantment, Land, Planeswalker). Player selects via ChosenIndex, handler returns all matching cards from graveyard to hand, then calls move_spell_after_resolve. Flashback cost is correct. Multi-type cards (e.g., artifact creatures) are correctly handled since the filter uses `contains`. No issues found.
