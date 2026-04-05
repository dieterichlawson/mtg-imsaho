## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, you may transform this creature.
**Type line**: Creature — Human
**Status**: ISSUE

### Code issues
- Oracle text wording discrepancy in mtg-engine/src/cards/isd/cloistered_youth.rs:25
  - Oracle text says: `At the beginning of your upkeep, you may transform this creature.`
  - Code does: `oracle_text: "At the beginning of your upkeep, you may transform Cloistered Youth.".into()`

### Tricky interactions checked
- "may" optionality for transform: pass - code correctly presents yes/no choice to player
- Transform timing (controller's upkeep only): pass - code checks active_player == controller  
- Back face mandatory life loss: pass - code causes life loss without player choice
- Back face trigger timing (end step): pass - code triggers during controller's end step only
- Transform state tracking: pass - code correctly checks is_transformed flag
- Dynamic P/T update: pass - dynamic_pt returns (3,3) when transformed
- Trigger description lookup for DFCs: pass - trigger_description correctly handles is_transformed state
- Source leaving battlefield during trigger resolution: pass - both triggers check zone == Zone::Battlefield

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Front face transform choice presentation: `mtg-engine/tests/tier15_cards.rs:1033`
- Player can decline transform: `mtg-engine/tests/tier15_cards.rs:1060`
- Back face life drain at end step: `mtg-engine/tests/tier15_cards.rs:1085`
- Front face has upkeep trigger only: `mtg-engine/tests/tier15_cards.rs:1102`
- Back face has end step trigger only: `mtg-engine/tests/tier15_cards.rs:1114`
- Oracle text wording discrepancy: NOT TESTED