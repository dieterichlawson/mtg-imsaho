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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand. Flashback {5}{G}{G}
**Scryfall type line**: Sorcery
**Status**: PASS

Findings:
- Mana cost {3}{G}{G}: correct.
- Type Sorcery: correct.
- P/T N/A: correct.
- Flashback cost {5}{G}{G}: correct.
- on_resolve presents ChooseCardType choice with 5 permanent types (Creature, Artifact, Enchantment, Land, Planeswalker): correct.
- Resolution handler in engine.rs (line 1608-1641) processes the choice: maps type string to CardType, filters graveyard, moves matching cards to hand, calls move_spell_after_resolve: correct.
- Anti-pattern check: move_spell_after_resolve is called in the engine handler (line 1640), not move_object to graveyard: correct.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- Tests found in tier15_cards.rs.
- Previous issue (hardcoded creature type) confirmed still fixed.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/174/creeping-renaissance)
**Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand. Flashback {5}{G}{G}
**Type line**: Sorcery
**Status**: PASS

Findings:
- Mana cost {3}{G}{G}: correct.
- Type Sorcery: correct.
- P/T N/A: correct.
- Flashback cost {5}{G}{G}: correct.
- on_resolve presents ChooseCardType choice with all 5 permanent types (Creature, Artifact, Enchantment, Land, Planeswalker): correct per oracle.
- Resolution handler in engine.rs (line 1608-1641) processes the choice correctly: maps type string to CardType, filters graveyard cards matching chosen type, moves them to hand, calls move_spell_after_resolve: correct.
- Anti-pattern check: move_spell_after_resolve called in engine handler (line 1640): correct.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- Tests: 3 tests in tier15_cards.rs (creeping_renaissance_returns_creatures_from_graveyard, creeping_renaissance_only_returns_chosen_type, creeping_renaissance_flashback_exiles). Good coverage including flashback exile behavior.
