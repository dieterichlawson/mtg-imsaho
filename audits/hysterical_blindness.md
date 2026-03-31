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
