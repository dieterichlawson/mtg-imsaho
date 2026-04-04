## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Exile a creature card from your graveyard and pay its mana cost: Create a token that's a copy of that card. Activate only as a sorcery.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
- X-cost creatures in graveyard are not handled per ruling. If a creature card with {X} in its mana cost is in the graveyard, Back from the Brink generates an ability that includes `ManaSymbol::X` in the cost. The engine's X-cost handler (engine.rs:1719-1731) treats this as variable X, draining the player's entire mana pool, but the ruling states X should be considered zero.
  - Oracle ruling says: `If the exiled creature card has {X} in its mana cost, X is considered to be zero.`
  - Code does: `activated_abilities()` at back_from_the_brink.rs:65-67 passes through `ManaSymbol::X` from the registry cost: `registry.card_data(creature.card_id).and_then(|d| d.cost.clone())`, causing the engine to treat X as variable rather than 0.

### Tricky interactions checked
- X-cost handling for creatures like Mikaeus, the Lunarch: FAIL (X treated as variable, should be 0)  
- Sorcery-speed only restriction: PASS (`sorcery_speed_only: true` at line 84)
- Token copy preserves creature characteristics: PASS (`create_token_copy` copies name, P/T, types, subtypes, keywords, and card_id)
- Double-faced card uses front face cost: PASS (`registry.card_data()` returns front face data)
- Double-faced card token copies front face: PASS (`create_token_copy` uses registry data for characteristics)
- Multiple creatures in graveyard generate separate abilities: PASS (tested in `tier15_cards.rs:845`)
- Creature must still be in graveyard at resolution: PASS (zone check at lines 99-103)
- Only creature cards in controller's graveyard are eligible: PASS (lines 51-61 filter correctly)
- Mana cost accurately reflects creature's cost: PASS (for non-X creatures - tested in `tier15_cards.rs:900`)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic token creation from graveyard creature: `tier15_cards.rs:813` (back_from_the_brink_creates_token_copy)
- One ability per graveyard creature: `tier15_cards.rs:845` (back_from_the_brink_ability_per_creature_in_graveyard)  
- No abilities without creatures in graveyard: `tier15_cards.rs:883` (back_from_the_brink_no_abilities_without_creatures_in_graveyard)
- Ability uses creature's mana cost: `tier15_cards.rs:900` (back_from_the_brink_uses_creature_mana_cost)
- X-cost creature in graveyard (X=0 ruling): NOT TESTED
- Double-faced creature uses front face cost: NOT TESTED
- Double-faced creature token can't transform: NOT TESTED
- Creature removed from graveyard before resolution: NOT TESTED (but code handles it via zone check)