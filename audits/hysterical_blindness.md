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
