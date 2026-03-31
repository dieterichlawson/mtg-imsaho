# Audit: Mask of Avacyn

## Official Oracle
- **Name:** Mask of Avacyn
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature gets +1/+2 and has hexproof. Equip {3}

## Implementation: `mtg-engine/src/cards/mask_of_avacyn.rs`
- **Name:** Mask of Avacyn -- CORRECT
- **Cost:** {2} -- CORRECT
- **Type:** Artifact -- CORRECT
- **Subtypes:** Equipment -- CORRECT
- **Oracle text:** Equipped creature gets +1/+2 and has hexproof. Equip {3} -- CORRECT
- **Continuous effects:** ModifyPT +1/+2 Attached, GrantKeyword Hexproof Attached -- CORRECT
- **Equip cost:** {3}, sorcery speed, targets creature you control -- CORRECT
- **on_resolve:** Moves to battlefield, sets is_equipment -- CORRECT

## Verdict
**PASS** -- No issues found.
