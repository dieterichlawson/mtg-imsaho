## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures your opponents control get -4/-0 until end of turn.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Opponents-only filter**: Code uses `obj.controller != controller` which correctly identifies opponents' creatures only, excluding the caster's own creatures — pass
- **Creature identification**: Uses `obj.power.is_some()` which is the established pattern throughout the engine for identifying creatures — pass
- **Resolution-time snapshot**: Code collects `opponent_creature_ids` at resolution time and applies effects to those specific IDs, so creatures entering battlefield or changing control later are unaffected (per ruling 1) — pass
- **Effect persistence after control change**: `UntilEndOfTurnEffect` targets specific object IDs rather than filtering by current controller, so the -4/-0 debuff persists even if you gain control of affected creatures (per ruling 2) — pass
- **Until end of turn cleanup**: `until_end_of_turn_effects` is cleared during cleanup step (`engine.rs:3021`), so effects properly expire — pass
- **No targeting required**: Hysterical Blindness affects all qualifying creatures when it resolves without targeting, and code correctly implements this with no targeting logic — pass
- **Spell cleanup**: Correctly calls `move_spell_after_resolve()` to move the instant to graveyard after resolution — pass

### Test coverage
- **Basic functionality**: `innistrad_cards.rs:180-197` (`hysterical_blindness_debuffs_opponents`) / TESTED
- **Opponents-only effect**: Test verifies own creatures unaffected while opponent's creatures get -4/-0 / TESTED  
- **Power/toughness modification**: Test verifies 5/5 becomes 1/5 (power -4, toughness unchanged) / TESTED
- **Resolution-time snapshot ruling**: NOT TESTED
- **Control-change persistence ruling**: NOT TESTED
- **Until end of turn expiry**: NOT TESTED

Sources:
- [Hysterical Blindness · Innistrad (ISD) #59 - Scryfall](https://scryfall.com/card/isd/59/hysterical-blindness)
- [Hysterical Blindness rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Hysterical-Blindness/rulings)