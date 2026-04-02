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

## Audit: Into the Maw of Hell
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Sorcery
- **Cost:** {4}{R}{R}
- **Oracle:** Destroy target land. Into the Maw of Hell deals 13 damage to target creature.

### Card Data
- **Name:** Into the Maw of Hell -- PASS
- **Cost:** {4}{R}{R} -- PASS
- **Types:** Sorcery -- PASS
- **P/T:** None -- PASS

### Oracle Text Match
- Exact match. -- PASS

### Behavior Audit
- **Targeting:** TwoTargets requiring a Land permanent and a Creature. -- PASS
- **is_valid_target:** Checks battlefield zone, allows lands or creatures. -- PASS
- **on_resolve (land):** Destroys target land via try_destroy. -- PASS
- **on_resolve (creature):** Deals 13 damage (damage_marked += 13), emits NonCombatDamageDealt event. -- PASS
- **Independent targets:** Each target checked independently; if one is illegal, the other still resolves. Consistent with rulings. -- PASS
- **Cleanup:** Calls move_spell_after_resolve. -- PASS

### Result: PASS
