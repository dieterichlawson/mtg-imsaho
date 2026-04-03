## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
**Type line**: Creature — Dragon
**Status**: PASS

### Code issues
No issues found.

Note: The oracle_text field in code uses "Balefire Dragon" where Scryfall now uses "this creature" (WotC template update). This is cosmetic only and does not affect behavior.

### Tricky interactions checked
- Triggered ability damage is non-combat: PASS (code emits `NonCombatDamageDealt` event at line 57, matching ruling from 2018-12-07)
- Only damages defending player's creatures, not controller's own: PASS (filters `o.controller == damaged_player` at line 46; test confirms own creatures untouched)
- Damage amount matches combat damage dealt (not power): PASS (uses `amount` parameter passed from trigger system, not hardcoded power)
- Self must be on battlefield to trigger: PASS (zone check at line 40)

### Test coverage
- Basic sweep of opponent creatures + own creatures unharmed: `tier6_cards.rs:329` (balefire_dragon_sweeps_opponent_creatures)
- Trigger does not fire when Balefire Dragon is not on battlefield: NOT TESTED (code handles it at line 40, but no dedicated test)
- Variable damage amount (e.g. power-modifying effects changing combat damage): NOT TESTED

---

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
**Type line**: Creature — Dragon
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.
