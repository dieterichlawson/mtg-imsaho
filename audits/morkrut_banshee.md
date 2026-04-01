## Audit — 2026-04-01

**Scryfall Oracle text**: Morbid — When Morkrut Banshee enters the battlefield, if a creature died this turn, target creature gets -4/-4 until end of turn.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

- Name: Correct ("Morkrut Banshee")
- Cost: {3}{B}{B} - Correct
- Type: Creature — Spirit - Correct
- P/T: 4/4 - Correct
- Morbid condition: Checks `creature_died_this_turn` - Correct
- Targeting: Can target ANY creature (including itself, per the rules). Confirmed in test.
- Effect: -4/-4 until end of turn via PendingEffect::DebuffUntilEOT - Correct
- Mandatory targeting (not "you may") - Correct
- Tests: card_fixes.rs has `morkrut_banshee_can_target_self` test.

No issues found.
