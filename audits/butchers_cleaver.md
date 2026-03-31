# Audit: Butcher's Cleaver

## Oracle (Scryfall/API)
- **Name:** Butcher's Cleaver
- **Cost:** {3}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
- **P/T:** N/A

## Implementation: `butchers_cleaver.rs`
- **Name:** Butcher's Cleaver -- CORRECT
- **Cost:** {3} -- CORRECT
- **Type:** Artifact — Equipment -- CORRECT (subtypes: ["Equipment"])
- **Static P/T bonus:** +3/+0 via ModifyPT with Attached scope -- CORRECT
- **Conditional lifelink:** Grants Lifelink keyword if creature is Human via `update_effects` -- CORRECT
- **Equip cost:** {3}, sorcery speed -- CORRECT
- **Target validation:** Only your own creatures -- CORRECT

## Issues
1. **ISSUE (minor):** The Human check in `update_effects` only checks registry subtypes, not object subtypes. Token creatures that have Human subtype only on the object (not in registry) would not get lifelink. Other cards (e.g., Avacynian Priest) check both sources.
2. **ISSUE (minor):** Like Bonds of Faith, the Human check is done once when equipping. If the creature gains/loses Human subtype later, the lifelink status won't update. The oracle says "as long as" which implies continuous checking.

## Verdict: PASS (with minor limitations) -- Human check is slightly incomplete and not continuously updated
