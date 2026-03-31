# Audit: Into the Maw of Hell

## Oracle (Official)
- **Name:** Into the Maw of Hell
- **Cost:** {4}{R}{R}
- **Type:** Sorcery
- **Oracle:** Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
- **P/T:** N/A

## Implementation
- Name: "Into the Maw of Hell" -- CORRECT
- Cost: {4}{R}{R} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- Two targets: land + creature via TwoTargets -- CORRECT
- Destroys land via try_destroy -- CORRECT
- Deals 13 damage to creature -- CORRECT
- Emits NonCombatDamageDealt event -- CORRECT
- Calls move_spell_after_resolve -- CORRECT

## Issues
None.

## Verdict: PASS
