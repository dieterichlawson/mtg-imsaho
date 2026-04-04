## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues
- Oracle text field wording mismatch in `/Users/dlaw/mtg/mtg-engine/src/cards/isd/disciple_of_griselbrand.rs:25`
  - Oracle text says: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`
  - Code does: `oracle_text: "{1}, Sacrifice a creature: You gain life equal to that creature's toughness."`

### Tricky interactions checked
- Toughness calculation uses battlefield value not graveyard: pass (correctly uses `last_known_toughness` from CreatureDied event per 2011-09-22 ruling)
- Sacrifice cost bypasses regeneration and indestructible: pass (uses `destruction::sacrifice` function)
- Ability resolves even if source is sacrificed: pass (activated ability with sacrifice as cost)
- Auto-sacrifice behavior when multiple creatures available: pass (engine auto-selects first creature, TODO exists to add player choice)
- Life gain is mandatory when ability activates: pass (no "may" in oracle text)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic life gain functionality: `mtg-engine/tests/tier8_cards.rs:138-166`
- Toughness calculation using battlefield value not graveyard: NOT TESTED
- Sacrifice cost bypassing regeneration/indestructible: NOT TESTED  
- Ability resolving when source is sacrificed: NOT TESTED
- Multiple creature sacrifice choice: NOT TESTED