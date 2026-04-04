## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
**Type line**: Creature — Angel
**Status**: ISSUE

### Code issues
- Engine prevents triggered ability resolution when source permanent leaves battlefield (mtg-engine/src/triggers.rs:955)
  - Oracle text says: `At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.`
  - Code does: Checks if Angel is still on battlefield before resolving trigger; if Angel is gone, trigger doesn't resolve at all
  - MTG Rule 112.7a: Once triggered, abilities exist independently on the stack and should resolve even if source leaves battlefield (since this ability doesn't reference the source)

### Tricky interactions checked
- "target Spirit card": PASS - correctly finds Spirit cards in graveyard and presents as targets
- "from your graveyard": PASS - correctly searches controller's graveyard specifically 
- "at the beginning of your upkeep": PASS - uses TriggerKind::Upkeep and checks active_player == controller
- "mandatory targeting": PASS - uses optional=false, not a "you may" ability
- "no targets available": PASS - helper returns early if targets.is_empty(), matching ruling that ability is removed with no effect
- "source leaves battlefield before resolution": FAIL - ability incorrectly fails to resolve when Angel leaves battlefield
- "subtype checking for tokens": PASS - correctly checks both registry.card_data().subtypes and o.subtypes

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (single Spirit in graveyard): `mtg-engine/tests/tier7_cards.rs:225-241`
- Multiple Spirits in graveyard (choice presentation): NOT TESTED
- No Spirits in graveyard (ruling case): NOT TESTED
- Angel leaves battlefield before trigger resolution: NOT TESTED
- Targeting Spirit tokens vs cards: NOT TESTED
- Angel controlled by different player than active player: NOT TESTED