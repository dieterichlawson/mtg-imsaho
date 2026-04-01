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
## Audit — 2026-04-01

**Scryfall Oracle text**: Rolling Temblor deals 2 damage to each creature without flying. Flashback {4}{R}{R}
**Scryfall type line**: Sorcery
**Status**: ISSUE

- **Missing damaged_by tracking**: `mtg-engine/src/cards/rolling_temblor.rs:37-39` marks damage on creatures but does not push to `obj.damaged_by`. This means death triggers that check `damaged_by` (e.g., Abattoir Ghoul) won't know Rolling Temblor was the source.
- **Oracle text incomplete**: oracle_text field does not include "Flashback {4}{R}{R}" text (minor, since flashback_cost is correctly set).
