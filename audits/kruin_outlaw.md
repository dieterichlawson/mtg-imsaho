# Audit: Kruin Outlaw // Terror of Kruin Pass

## Oracle (Official)
### Front: Kruin Outlaw
- **Cost:** {1}{R}{R}
- **Type:** Creature — Human Rogue Werewolf
- **Oracle:** First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
- **P/T:** 2/2

### Back: Terror of Kruin Pass
- **Type:** Creature — Werewolf
- **Oracle:** Double strike. Each Werewolf you control can't be blocked except by two or more creatures. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
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
- Back keywords: [DoubleStrike] -- CORRECT (no menace keyword)
- Transform logic -- CORRECT
- Global blocking restriction -- CORRECT (MinimumBlockers continuous effect)

## Issues
1. **FIXED:** The back face previously listed Menace as a keyword but did not implement the global effect. Now correctly uses `ContinuousEffect::MinimumBlockers { count: 2 }` with scope `Global(And(You, HasSubtype("Werewolf")))`, affecting all Werewolves you control. The Menace keyword was removed from the back face's keyword list since the Oracle text does not grant menace — it has a separate static ability.

## Verdict: ALL ISSUES FIXED

## Audit — 2026-04-01 06:20

**Scryfall Oracle text (front)**: First strike / At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Scryfall Oracle text (back)**: Double strike / Each Werewolf you control can't be blocked except by two or more creatures. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Scryfall type line (front)**: Creature — Human Rogue Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: PASS

No issues found. The global "can't be blocked except by two or more creatures" blocking restriction is correctly implemented as a MinimumBlockers continuous effect that applies to all Werewolves controlled by the same player. Tests verify self, other werewolves, non-werewolves, and opponent werewolves are all handled correctly. The MinimumBlockers enforcement is integrated into the combat blocker validation system.
