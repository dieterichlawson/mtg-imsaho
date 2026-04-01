## Audit — 2026-04-01

**Scryfall Oracle text**: Rolling Temblor deals 2 damage to each creature without flying.\nFlashback {4}{R}{R}
**Scryfall type line**: Sorcery
**Mana cost**: {2}{R}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}{R}, type Sorcery
- Deals 2 damage to each creature without flying (checks `has_keyword(Keyword::Flying)`)
- Flashback cost {4}{R}{R}
- Properly emits NonCombatDamageDealt events
- Tests: `rolling_temblor_damages_non_flyers` in flashback.rs

No issues found.
