## Audit — 2026-04-01

**Scryfall Oracle text**: Reach\nMorbid — When Somberwald Spider enters the battlefield, if a creature died this turn, put two +1/+1 counters on Somberwald Spider.
**Scryfall type line**: Creature — Spider
**Mana cost**: {4}{G}
**P/T**: 2/4
**Status**: PASS

Implementation correctly models:
- Name, mana cost {4}{G}, type Creature, subtype Spider, P/T 2/4
- Reach keyword
- Morbid ETB: if `creature_died_this_turn`, adds 2 +1/+1 counters
- Tests: `somberwald_spider_morbid_counters`, `somberwald_spider_no_morbid_no_counters` in card_mechanics.rs, and `somberwald_spider_has_reach` in innistrad_cards.rs

No issues found.
