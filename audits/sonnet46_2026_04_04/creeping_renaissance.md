## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Permanent type options exactly match the 5 permanent types**: The card offers Creature, Artifact, Enchantment, Land, Planeswalker — exactly the set per the ruling "The permanent types are artifact, creature, enchantment, land, and planeswalker." Sorcery and Instant are not offered. PASS
- **"all" cards returned (no limit)**: The engine scans the entire graveyard and returns every matching card; there is no cap. PASS
- **"your graveyard" scoped correctly**: `objects_in_zone(Zone::Graveyard, *controller)` is called. In `state.rs` the graveyard zone filters by `owner`, and the `controller` variable holds the spell controller (who is also the owner in normal play), so the search covers exactly "your graveyard." PASS
- **Choice is mandatory ("Choose a permanent type", no "may")**: The code presents a `ChooseCardType` choice that requires the player to pick one of the 5 options. There is no way to decline the choice. PASS
- **Flashback exile rule**: `cast_with_flashback` is set to `true` when the spell is cast from the graveyard; `move_spell_after_resolve` checks this flag and sends the spell to `Zone::Exile` instead of `Zone::Graveyard`. PASS
- **Spell cleanup timing with awaiting_action**: `on_resolve` sets `awaiting_action` and returns; `stack.rs` detects `awaiting_action.is_some()` and skips the early `move_spell_after_resolve` call. The `ChooseCardType` branch in `engine.rs` calls `move_spell_after_resolve(*spell_id)` after the effect resolves. No double-cleanup or missed cleanup possible. PASS
- **card_types detection in graveyard**: The filter in the engine checks `o.card_types` (non-empty for all game-setup cards, as `card_types` is populated from `card_data` at setup and not cleared on zone changes) before falling back to registry data. Correctly identifies card types for both regular cards and tokens in the graveyard. PASS
- **Flashback exiles even if countered**: The ruling states flashback spells are exiled whether they resolve, are countered, or leave the stack another way. The `fizzle` path in `stack.rs` calls `move_spell_after_resolve` which checks `cast_with_flashback`, so a countered/fizzled flashback spell is also exiled. PASS
- **Mana cost correctness**: {3}{G}{G} → `Generic(3), Colored(Green), Colored(Green)`. Flashback {5}{G}{G} → `Generic(5), Colored(Green), Colored(Green)`. Both match oracle. PASS
- **Sorcery timing restriction**: The spell is a `CardType::Sorcery`, so the engine restricts casting to sorcery speed (main phase, empty stack, active player). The flashback ruling confirms "you can cast a sorcery using flashback only when you could normally cast a sorcery." PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Returns all matching creature cards from graveyard: `mtg-engine/tests/tier15_cards.rs:467` (test `creeping_renaissance_returns_creatures_from_graveyard`)
- Returns only the chosen type, leaving others in graveyard: `mtg-engine/tests/tier15_cards.rs:506` (test `creeping_renaissance_only_returns_chosen_type`)
- Flashback exiles the spell: `mtg-engine/tests/tier15_cards.rs:553` (test `creeping_renaissance_flashback_exiles`)
- Flashback exiles even if countered: NOT TESTED
- Choosing Land, Artifact, or Planeswalker type: NOT TESTED
- Empty graveyard returns 0 cards: NOT TESTED
- All 5 permanent types are offered (not sorcery/instant): NOT TESTED explicitly (inferred from index-based selection in existing tests)
