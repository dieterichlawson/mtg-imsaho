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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature gets +4/+2. / Equip—Sacrifice a creature.
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:50
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Equipped creature gets +4/+2.\nEquip—Sacrifice a creature.
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
- No code issues found. All card data fields match oracle text exactly.
- Mana cost {4} implemented as `Generic(4)` -- correct.
- Continuous effect `ModifyPT { power: 4, toughness: 2, scope: Attached }` -- correct.
- Equip cost is `ManaCost::free()` with `SacrificeCost::SacrificeCreature` -- correct (no mana, sacrifice a creature).
- `sorcery_speed_only: true` -- correct (equip is sorcery speed by default).
- `on_resolve` sets `is_equipment = true` and moves to battlefield -- correct.
- `on_activate_ability` attaches equipment to target creature -- correct.
- Known engine limitation: sacrifice choice for activated abilities is auto-selected (first creature found), not player-chosen. This is a pre-existing engine-level TODO, not a card-specific bug.

### Tricky interactions checked (min 3)
1. **Sacrificing the equipped creature to re-equip (ruling 2011-09-22)**: The implementation uses `SacrificeCost::SacrificeCreature` which does not exclude the currently-equipped creature from sacrifice candidates. This correctly allows sacrificing the creature the Hauberk is currently attached to in order to move it to another creature.
2. **Equip at sorcery speed only**: `sorcery_speed_only: true` ensures the equip ability can only be activated during the controller's main phase when the stack is empty, consistent with the equip keyword rules (CR 702.6).
3. **Equipment enters unattached**: `on_resolve` moves the artifact to the battlefield and sets `is_equipment = true` but does not set `attached_to`. The test `equipment_enters_unattached` explicitly confirms this. When the equipped creature dies, SBA handling in `sba.rs` (line ~151) keeps equipment on the battlefield unattached rather than sending it to graveyard (unlike auras).
4. **Equip target validation**: `is_valid_target` checks that the target is on the battlefield, is a creature (has power), and is controlled by the caster -- matching the equip keyword requirement of "creature you control."

### Test coverage
- `demonmail_hauberk_card_data` -- verifies Artifact type, Equipment subtype, CMC 4. PASS
- `demonmail_hauberk_equip_sacrifices_creature` -- equips creature_b by sacrificing creature_a, verifies attachment and +4/+2 bonus (effective 6/4 on a 2/2). PASS
- `equipment_enters_unattached` -- casts Demonmail Hauberk, verifies it enters battlefield unattached with is_equipment flag. PASS
