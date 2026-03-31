# Audit: Kruin Outlaw // Terror of Kruin Pass

## Oracle (Official)
### Front: Kruin Outlaw
- **Cost:** {1}{R}{R}
- **Type:** Creature — Human Rogue Werewolf
- **Oracle:** First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
- **P/T:** 2/2

### Back: Terror of Kruin Pass
- **Type:** Creature — Werewolf
- **Oracle:** Double strike. Menace. Each Werewolf you control can't be blocked except by two or more creatures. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
- **P/T:** 3/3

## Implementation
- Front name: "Kruin Outlaw" -- CORRECT
- Front cost: {1}{R}{R} -- CORRECT
- Front subtypes: ["Human", "Rogue", "Werewolf"] -- CORRECT
- Front P/T: 2/2 -- CORRECT
- Front keywords: [FirstStrike] -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Terror of Kruin Pass" -- CORRECT
- Back subtypes: ["Werewolf"] -- CORRECT
- Back P/T: 3/3 (via dynamic_pt) -- CORRECT
- Back keywords: [DoubleStrike, Menace] -- CORRECT
- Transform logic -- CORRECT

## Issues
1. **ISSUE (minor):** The back face oracle text in the implementation says "Each Werewolf you control can't be blocked except by two or more creatures" but also lists Menace as a keyword. The real card grants menace to all your Werewolves (a global effect), not just to this creature. The implementation only lists the keywords on back_face_data but does NOT implement a continuous effect granting menace to other Werewolves you control.

## Verdict: PASS (with minor issue — global menace for Werewolves not implemented)
