# Audit: Instigator Gang // Wildblood Pack

## Oracle (Official)
### Front: Instigator Gang
- **Cost:** {3}{R}
- **Type:** Creature — Human Werewolf
- **Oracle:** Attacking creatures you control get +1/+0. At the beginning of each upkeep, if no spells were cast last turn, transform Instigator Gang.
- **P/T:** 2/3

### Back: Wildblood Pack
- **Type:** Creature — Werewolf
- **Oracle:** Trample. Attacking creatures you control get +3/+0. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.
- **P/T:** 5/5

## Implementation
- Front name: "Instigator Gang" -- CORRECT
- Front cost: {3}{R} -- CORRECT
- Front subtypes: ["Human", "Werewolf"] -- CORRECT
- Front P/T: 2/3 -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Wildblood Pack" -- CORRECT
- Back subtypes: ["Werewolf"] -- CORRECT
- Back P/T: 5/5 (via dynamic_pt) -- CORRECT
- Back keywords: [Trample] -- CORRECT
- Back oracle text matches -- CORRECT
- Transform logic: no spells -> transform to back; any player 2+ spells -> transform to front -- CORRECT
- Attacking creatures buff: +1/+0 (front) or +3/+0 (back) via on_any_creature_attacks -- CORRECT
- Uses UntilEndOfTurnEffect for power bonus -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit — 2026-04-01 15:12

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform Instigator Gang.
**Oracle text (back)**: Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Wildblood Pack.
**Type line (front)**: Creature — Human Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

### Code issues

1. **Instigator Gang / Wildblood Pack does not buff itself when attacking**
   - Oracle text says: `Attacking creatures you control get +1/+0.` (front) / `Attacking creatures you control get +3/+0.` (back) — no "other" qualifier, so the buff applies to ALL attacking creatures you control, including Instigator Gang/Wildblood Pack itself.
   - Code does: The `AnyCreatureAttacks` watcher in `mtg-engine/src/triggers.rs:708` filters with `o.id != *attacker_id`, which means when Instigator Gang itself attacks, it is excluded from the watchers list and `on_any_creature_attacks` is never called with itself as both watcher and attacker. As a result, Instigator Gang never gives itself the +1/+0 (or +3/+0) bonus when it attacks.
   - Note: This is an engine-level limitation in how `AnyCreatureAttacks` watchers are dispatched. The fix would need to either include self in the watcher list or add a separate `on_attacks` hook for the self-buff case.

### Tricky interactions checked
- Werewolf transform timing (upkeep, no spells / 2+ spells): PASS
- First turn protection (no transform on first turn): PASS
- Buff only applies to creatures the controller owns: PASS
- Trample on back face only: PASS
- Back face missing Upkeep in triggered_abilities but trigger still fires because front face has it: PASS (works due to `trigger_description` checking front face first)

### Test coverage
- Transform test: `werewolf_cards.rs:instigator_gang_transforms_and_gains_trample` — tests transform and trample
- Buff on attack (other creatures): NOT TESTED
- Buff on attack (self): NOT TESTED (this is the bug — would fail)
- Transform back when 2+ spells cast: NOT DIRECTLY TESTED (covered by generic werewolf tests)
- First turn no-transform: NOT DIRECTLY TESTED (covered by generic werewolf tests)
