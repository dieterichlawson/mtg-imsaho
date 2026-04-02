# Audit: Demonmail Hauberk

## Scryfall Reference
- **Name:** Demonmail Hauberk
- **Cost:** {4}
- **Type:** Artifact -- Equipment
- **Oracle:** Equipped creature gets +4/+2. Equip--Sacrifice a creature.
- **P/T:** N/A
- **Keywords:** Equip

## Implementation: `demonmail_hauberk.rs`
- **Name:** Demonmail Hauberk -- CORRECT
- **Cost:** {4} -- CORRECT
- **Type:** Artifact -- CORRECT
- **Subtypes:** ["Equipment"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Continuous effect:** ModifyPT { power: 4, toughness: 2, scope: Attached } -- CORRECT
- **Equip cost:** Free mana, SacrificeCost::SacrificeCreature -- CORRECT
- **Equip sorcery speed:** true -- CORRECT

## Issues
None

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
```
Equipped creature gets +4/+2.
Equip—Sacrifice a creature.
```
Type: Artifact — Equipment | Cost: {4} | Keywords: Equip

### Implementation Checklist (`mtg-engine/src/cards/isd/demonmail_hauberk.rs`)
| Field | Oracle | Implementation | Verdict |
|---|---|---|---|
| Name | Demonmail Hauberk | `"Demonmail Hauberk"` | CORRECT |
| Mana cost | {4} | `Generic(4)` | CORRECT |
| Card types | Artifact | `CardType::Artifact` | CORRECT |
| Subtypes | Equipment | `"Equipment"` | CORRECT |
| P/T | N/A | `None / None` | CORRECT |
| Oracle text string | (see above) | matches verbatim | CORRECT |
| Continuous effect | +4/+2 to equipped creature | `ModifyPT { power: 4, toughness: 2, scope: Attached }` | CORRECT |
| Equip cost | Sacrifice a creature (no mana) | `ManaCost::free()` + `SacrificeCost::SacrificeCreature` | CORRECT |
| Equip target | Creature you control | `CreatureWithFilter(TargetFilter::YouControl)` | CORRECT |
| Sorcery speed | Yes (equip default) | `sorcery_speed_only: true` | CORRECT |
| Equipment flag | enters as equipment | `on_resolve` sets `is_equipment = true` | CORRECT |

### Ruling Coverage
- **2011-09-22**: "You can sacrifice the creature Demonmail Hauberk is equipping in order to equip it to another creature." — Not explicitly tested, but the implementation allows it since `SacrificeCreature` does not exclude the currently-equipped creature.

### Tests (`mtg-engine/tests/tier9_cards.rs`)
- `demonmail_hauberk_card_data` — verifies Artifact type, Equipment subtype, CMC 4. **PASS**
- `demonmail_hauberk_equip_sacrifices_creature` — equips via sacrifice, checks attachment and +4/+2 bonus. **PASS**
- `equipment_enters_unattached` — casts Demonmail Hauberk, verifies it enters battlefield unattached with `is_equipment = true`. **PASS**

### Mismatches Found
None.

### Verdict
**PASS** — Implementation matches oracle text. No issues found.
