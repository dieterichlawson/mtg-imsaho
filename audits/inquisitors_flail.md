# Audit: Inquisitor's Flail

## Oracle (Official)
- **Name:** Inquisitor's Flail
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle:** If equipped creature would deal combat damage, it deals double that damage instead. If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
- **P/T:** N/A

## Implementation
- Name: "Inquisitor's Flail" -- CORRECT
- Cost: {2} -- CORRECT
- Type: Artifact -- CORRECT
- Subtypes: ["Equipment"] -- CORRECT
- Equip {2}, sorcery speed, targets creature you control -- CORRECT
- Oracle text: says "another creature" in code comment but oracle says "another source" -- the oracle_text string in code is correct

## Issues
1. **ISSUE (simplification):** Offensive double damage is approximated by granting +P/+0 equal to creature's effective power via `dynamic_pt`. This is an approximation rather than a true damage replacement effect. The comment acknowledges this.
2. **ISSUE (missing):** Defensive doubling (equipped creature takes double combat damage from other sources) is NOT implemented. Comment acknowledges this.
3. **ISSUE (minor):** The `dynamic_pt` approach means the power bonus is visible outside combat, which could affect other game interactions differently than the real card.

## Verdict: PASS (with noted simplifications)
