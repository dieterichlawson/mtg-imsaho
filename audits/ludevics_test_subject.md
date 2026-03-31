# Audit: Ludevic's Test Subject // Ludevic's Abomination

## Oracle (Official)
### Front: Ludevic's Test Subject
- **Cost:** {1}{U}
- **Type:** Creature — Lizard Egg
- **Oracle:** Defender. {1}{U}: Put a hatchling counter on Ludevic's Test Subject. Then if there are five or more hatchling counters on it, remove all of them and transform Ludevic's Test Subject.
- **P/T:** 0/3

### Back: Ludevic's Abomination
- **Type:** Creature — Lizard Horror
- **Oracle:** Trample
- **P/T:** 13/13

## Implementation
- Front name: "Ludevic's Test Subject" -- CORRECT
- Front cost: {1}{U} -- CORRECT
- Front P/T: 0/3 -- CORRECT
- Front keywords: [Defender] -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Ludevic's Abomination" -- CORRECT
- Back subtypes: ["Lizard", "Horror"] -- CORRECT
- Back P/T: 13/13 (via dynamic_pt) -- CORRECT
- Back keywords: [Trample] -- CORRECT
- Activated ability: {1}{U}, adds hatchling counter, transforms at 5 -- CORRECT
- Uses card_state for hatchling counter tracking -- OK (workaround)

## Issues
1. **ISSUE (minor):** Front face subtypes are ["Lizard"] but should be ["Lizard", "Egg"]. The official type line is "Creature — Lizard Egg".

## Verdict: PASS (with minor subtype issue)
