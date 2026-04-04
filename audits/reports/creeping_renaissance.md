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

## Audit — 2026-04-02

**Oracle text** (Scryfall, cached 2026-04-01): Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand. Flashback {5}{G}{G}
**Type line**: Sorcery
**Status**: PASS

Findings:
- Mana cost {3}{G}{G}: correct.
- Type Sorcery: correct.
- Flashback cost {5}{G}{G}: correct (`flashback_cost` field in card_data).
- on_resolve presents `ChooseCardType` choice with all 5 permanent types (Creature, Artifact, Enchantment, Land, Planeswalker): correct per ruling "[2011-09-22] The permanent types are artifact, creature, enchantment, land, and planeswalker."
- Player must actively choose a type (not auto-selected): correct. The choice is presented via `AwaitingAction::ResolutionChoice`.
- Resolution handler in engine.rs (line 1854-1886) processes the choice: maps type string to CardType, filters all graveyard cards of chosen type, moves ALL matching to hand: correct. Oracle says "Return all cards of the chosen type" -- no targeting involved.
- `move_spell_after_resolve` called at engine.rs:1886: correct. This method checks `cast_with_flashback` flag and exiles if true, sends to graveyard otherwise.
- Tests: 3 tests in tier15_cards.rs covering basic return, type-filtering, and flashback exile.

Minor note (not a rules bug):
- In `mtg-player/src/llm.rs:491`, `ChosenIndex(i)` is displayed as `"Option {i}"` rather than the actual type name (e.g., "Creature"). The LLM player sees opaque labels like "Option 0" instead of meaningful type names. This is a UX concern for the AI player, not a game-rules issue.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:45

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/174/creeping-renaissance)
**Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

Card data is fully correct:
- Name: "Creeping Renaissance" -- matches oracle
- Mana cost: {3}{G}{G} -- matches oracle
- Type: Sorcery -- matches oracle
- Oracle text: matches verbatim
- Flashback cost: {5}{G}{G} via `flashback_cost` field -- matches oracle. Keywords left empty, consistent with all other flashback cards in the codebase.

Behavior (engine.rs lines 2051-2084):
- On resolve, presents choice of all 5 permanent types (Creature, Artifact, Enchantment, Land, Planeswalker) -- correct per ruling [2011-09-22].
- Filters controller's graveyard by chosen type using `card_types.contains(&card_type)` -- correctly handles multi-type cards (e.g., artifact creature returned when choosing either Artifact or Creature).
- Moves all matching cards to hand -- no targeting, matches "Return all cards of the chosen type".
- Calls `move_spell_after_resolve` which exiles if `cast_with_flashback` is true, otherwise sends to graveyard -- correct.

### Tricky interactions checked (min 3)
1. **Artifact creatures**: If you choose "Creature", an artifact creature card is returned because `card_types.contains(&CardType::Creature)` is true for `[CardType::Artifact, CardType::Creature]`. Likewise choosing "Artifact" returns it. Correct per MTG ruling.
2. **Flashback exile**: When cast via flashback, the spell is exiled after resolution (not returned to graveyard). Verified via `move_spell_after_resolve` which checks `cast_with_flashback` flag. Test `creeping_renaissance_flashback_exiles` confirms this.
3. **Empty graveyard / no matching cards**: If no cards of the chosen type exist in graveyard, `to_return` is empty, zero cards are moved, and the spell resolves normally. No crash or error -- the count is logged as 0. Graceful handling.
4. **Cannot return itself**: Creeping Renaissance is a Sorcery, not a permanent type. Even if it somehow ended up in the graveyard mid-resolution, it would not match any permanent type choice. Correct.

### Test coverage
3 tests in `mtg-engine/tests/tier15_cards.rs`, all passing:
- `creeping_renaissance_returns_creatures_from_graveyard`: 3 creatures in graveyard, choose Creature, all 3 returned to hand.
- `creeping_renaissance_only_returns_chosen_type`: Creatures and enchantments in graveyard, choose Enchantment, only enchantments returned; creatures remain.
- `creeping_renaissance_flashback_exiles`: Cast from graveyard via flashback, creature returned to hand, spell exiled afterward.
