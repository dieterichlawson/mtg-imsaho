# Audit: Cobbled Wings

## Scryfall Reference
- **Name:** Cobbled Wings
- **Cost:** {2}
- **Type:** Artifact -- Equipment
- **Oracle:** Equipped creature has flying. Equip {1}
- **P/T:** N/A
- **Keywords:** Equip

## Implementation: `cobbled_wings.rs`
- **Name:** Cobbled Wings -- CORRECT
- **Cost:** {2} -- CORRECT
- **Type:** Artifact -- CORRECT
- **Subtypes:** ["Equipment"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Continuous effect:** GrantKeyword Flying to Attached -- CORRECT
- **Equip cost:** {1} -- CORRECT
- **Equip sorcery speed:** true -- CORRECT
- **Target validation:** own creatures only -- CORRECT

## Issues
None
