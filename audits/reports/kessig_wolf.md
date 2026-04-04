# Audit: Kessig Wolf

## Oracle (Official)
- **Name:** Kessig Wolf
- **Cost:** {2}{R}
- **Type:** Creature — Wolf
- **Oracle:** {1}{R}: Kessig Wolf gains first strike until end of turn.
- **P/T:** 3/1

## Implementation
- Name: "Kessig Wolf" -- CORRECT
- Cost: {2}{R} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Wolf"] -- CORRECT
- P/T: 3/1 -- CORRECT
- Oracle text matches -- CORRECT
- Activated ability: {1}{R} cost, no tap required, grants first strike until end of turn -- CORRECT
- Uses UntilEndOfTurnKeyword for first strike -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Kessig Wolf
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Wolf
- **Cost:** {2}{R}
- **P/T:** 3/1
- **Oracle:** {1}{R}: This creature gains first strike until end of turn.

### Card Data
- **Name:** Kessig Wolf -- PASS
- **Cost:** {2}{R} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Wolf -- PASS
- **P/T:** 3/1 -- PASS

### Oracle Text Match
- Code uses "Kessig Wolf gains first strike" vs oracle "This creature gains first strike". Cosmetic only.
- PASS (minor wording variance)

### Behavior Audit
- **Activated ability cost:** {1}{R}, no tap required. -- PASS
- **Activated ability effect:** Grants FirstStrike via UntilEndOfTurnKeyword. -- PASS
- **Restrictions:** No once_per_turn, no sorcery_speed_only (can activate multiple times at instant speed). -- PASS
- **Zone check:** Only available on battlefield. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:08

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/151/kessig-wolf)
**Oracle text**: {1}{R}: This creature gains first strike until end of turn.
**Type line**: Creature — Wolf

**Status**: PASS

### Code issues

None. Implementation correctly matches oracle behavior.

Minor cosmetic note: The `oracle_text` field in `card_data()` uses "Kessig Wolf gains first strike" while Scryfall's modern Oracle text uses "This creature gains first strike". This is a templating update by WotC; behavior is identical.

### Tricky interactions checked (min 3)

1. **Multiple activations per turn**: `once_per_turn: false` is correct. The oracle text has no such restriction. Activating multiple times is legal but redundant since first strike is a binary keyword.
2. **First strike expires at cleanup**: The engine clears `until_end_of_turn_keywords` at cleanup step (engine.rs:3022), so the first strike grant correctly wears off at end of turn.
3. **Activation during combat**: `sorcery_speed_only: false` and `requires_tap: false` allow activating after blockers are declared but before damage, which is correct for this instant-speed activated ability with no tap cost.
4. **Zone restriction**: Ability is only offered when the object is on the battlefield (line 33), preventing activation from graveyard/hand/exile.

### Test coverage

- `kessig_wolf_has_correct_stats` (activated_abilities.rs:83): Verifies P/T 3/1 and Wolf subtype.
- `kessig_wolf_gains_first_strike` (activated_abilities.rs:93): Verifies ability activation grants first strike via the engine's legal action system.

Both tests pass.
