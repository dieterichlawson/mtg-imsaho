# Audit: Hysterical Blindness

## Oracle (Official)
- **Name:** Hysterical Blindness
- **Cost:** {2}{U}
- **Type:** Instant
- **Oracle:** Creatures your opponents control get -4/-0 until end of turn.
- **P/T:** N/A

## Implementation
- Name: "Hysterical Blindness" -- CORRECT
- Cost: {2}{U} -- CORRECT
- Type: Instant -- CORRECT
- Oracle text matches -- CORRECT
- Applies -4/+0 until end of turn to opponent creatures on battlefield -- CORRECT
- Correctly filters opponent creatures by `controller != controller` and `power.is_some()` -- CORRECT
- Calls `move_spell_after_resolve` -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Hysterical Blindness
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Instant
- **Cost:** {2}{U}
- **Oracle:** Creatures your opponents control get -4/-0 until end of turn.

### Card Data
- **Name:** Hysterical Blindness -- PASS
- **Cost:** {2}{U} -- PASS
- **Types:** Instant -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Exact match. -- PASS

### Behavior Audit
- **on_resolve:** Collects all battlefield creatures where controller != caster and power.is_some(). Applies -4/+0 UntilEndOfTurnEffect. -- PASS
- **Scope:** Correctly targets only opponents' creatures at resolution time, consistent with rulings. -- PASS
- **Cleanup:** Calls move_spell_after_resolve. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:04

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/59/hysterical-blindness, cached 2026-04-01)
**Oracle text**: Creatures your opponents control get -4/-0 until end of turn.
**Type line**: Instant
**Mana cost**: {2}{U}
**Status**: PASS

### Code issues

None found.

- Card name, mana cost, type line, oracle text all match Scryfall exactly.
- Power modifier is -4, toughness modifier is 0 — matches `-4/-0` from oracle.
- No targeting (correct: oracle does not use "target").
- Filters battlefield creatures controlled by opponents (`obj.controller != controller`).
- Uses `UntilEndOfTurnEffect` with target object ID, which correctly:
  - Only affects creatures on the battlefield at resolution time (per ruling 2011-09-22).
  - Continues applying even if control of the creature changes (per ruling 2011-09-22).
- Cleanup step clears `until_end_of_turn_effects` — effect correctly expires.
- Spell moves to graveyard after resolution (or exile if flashback).

### Tricky interactions checked (min 3)

1. **Creatures entering after resolution are unaffected**: The implementation snapshots creature IDs at resolution time into a `Vec<ObjectId>`, then applies effects only to those IDs. Creatures entering the battlefield later will not have an `UntilEndOfTurnEffect` entry, so they are unaffected. Matches ruling.
2. **Control change after resolution**: `UntilEndOfTurnEffect` tracks by `ObjectId`, not by controller. If the caster later gains control of an affected creature, the -4/-0 still applies because `effective_power` checks `effect.target == id` regardless of controller. Matches ruling.
3. **Creature with 3 or less power goes to 0 or negative power**: A creature whose power is reduced to 0 or below by -4/-0 can still block and exists on the battlefield — negative power is legal in MTG (it just deals 0 combat damage). The implementation uses `i32` for power, so negative values are represented correctly. No SBA kills a creature for having 0 or negative power.
4. **Interaction with toughness-boosting effects**: Since toughness_mod is 0, this does not interact with toughness at all. A creature at 1 toughness stays alive. Verified in `effective_toughness` which only sums `toughness_mod` from the effect.

### Test coverage

- `hysterical_blindness_debuffs_opponents` in `innistrad_cards.rs`: Verifies opponent's 5/5 becomes 1/5, own 2/2 is unaffected. Passes.
- No AI card knowledge file exists (not an issue for the audit but noted).
