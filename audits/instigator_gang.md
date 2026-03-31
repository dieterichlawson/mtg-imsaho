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
