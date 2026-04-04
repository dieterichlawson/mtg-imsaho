## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
This creature can't be blocked.
**Type line**: Creature — Human Rogue
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Hexproof targeting restriction**: Opponents cannot target this creature with spells/abilities, but controller can target their own creature — PASS
- **Unblockable mechanic**: No creature can block this creature regardless of abilities (flying, reach, etc.) — PASS  
- **Mass effects bypass hexproof**: Board wipes and non-targeting effects (saying "each" or "all") still affect hexproof creatures — PASS
- **Sacrifice effects work**: If forced to sacrifice a creature and this is the only one, it can be sacrificed despite hexproof — PASS
- **Equipment interaction**: Controller can equip their own hexproof creature (hexproof only blocks opponents) — PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Card has hexproof keyword**: `mtg-engine/tests/innistrad_cards.rs:115` (invisible_stalker_has_hexproof)
- **Cannot be blocked**: `mtg-engine/tests/card_mechanics.rs:455` (invisible_stalker_unblockable) 
- **Hexproof blocks opponent targeting**: `mtg-engine/tests/keywords.rs:166` (hexproof_prevents_opponent_targeting)
- **Controller can still target**: `mtg-engine/tests/keywords.rs:166` (hexproof_prevents_opponent_targeting)
- **Mass effects bypass hexproof**: NOT TESTED (but engine-level behavior, not card-specific)
- **Sacrifice effects work**: NOT TESTED (but engine-level behavior, not card-specific)
- **Equipment interaction**: NOT TESTED (but covered in other hexproof creature tests)