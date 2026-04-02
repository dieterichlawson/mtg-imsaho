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

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Mask of Avacyn
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle Text:** Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.) / Equip {3}

### Card Data Checks
- [x] Name: "Mask of Avacyn" — correct
- [x] Cost: {2} — correct
- [x] Types: Artifact — correct
- [x] Subtypes: Equipment — correct
- [x] Continuous effects: ModifyPT +1/+2 on Attached scope — correct
- [x] Continuous effects: GrantKeyword Hexproof on Attached scope — correct
- [x] Oracle text matches — correct

### Behavior Checks
- [x] Equip ability costs {3} — correct
- [x] Equip is sorcery speed only — correct
- [x] Equip targets a creature you control (YouControl filter) — correct
- [x] `on_activate_ability` attaches equipment to target creature — correct
- [x] `on_resolve` moves to battlefield and marks as equipment — correct
- [x] Target validation checks for battlefield creature controlled by caster — correct

### Result: PASS
