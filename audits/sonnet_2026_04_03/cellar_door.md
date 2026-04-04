## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
**Type line**: Artifact
**Status**: ISSUE

### Code issues
- Creature type checking logic doesn't handle tokens correctly (mtg-engine/src/cards/isd/cellar_door.rs:73-79)
  - Oracle text says: `If it's a creature card, you create a 2/2 black Zombie creature token.`
  - Code does: Only checks `registry.card_data(o.card_id).map(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))` with fallback to `o.power.is_some()`. Missing check of `o.card_types` which is where token card types are stored. This would cause the ability to fail detecting creature tokens.

- Test puts creature at wrong library position (mtg-engine/tests/tier15_cards.rs:621)
  - Oracle text says: `Target player puts the bottom card of their library into their graveyard`
  - Code does: Test uses `library_order.insert(0, card)` which puts card at top of library, but ability mills from bottom using `library_order.len() - 1`

### Tricky interactions checked
- Empty library handling: pass (returns early if library_order.is_empty())
- Token creation parameters: pass (matches other 2/2 black Zombie tokens like Moan of the Unhallowed)
- Target player choice: pass (TargetRequirement::PlayerOnly correctly allows any player including self)
- Bottom-of-library milling: pass (uses len-1 index correctly, unlike top-milling cards that use index 0)
- Creature card detection for regular cards: pass (registry.card_data check works for non-tokens)
- Fallback creature detection: fail (o.power.is_some() is not rules-accurate proxy for creature type)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Creature card milled creates Zombie token: tier15_cards.rs:607 / INCORRECTLY TESTED (creature at wrong position)
- Non-creature card milled doesn't create token: NOT TESTED
- Empty library handling: NOT TESTED
- Target any player (including self): NOT TESTED
- Token creature card detection: NOT TESTED (would fail due to code bug)