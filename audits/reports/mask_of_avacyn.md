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

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/229/mask-of-avacyn)
**Oracle text**: Equipped creature gets +1/+2 and has hexproof. (It can't be the target of spells or abilities your opponents control.) / Equip {3}
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
None found. Implementation is correct and complete.

- Name: "Mask of Avacyn" — matches oracle
- Mana cost: {2} (Generic(2)) — matches oracle
- Card types: [Artifact] — matches oracle
- Subtypes: ["Equipment"] — matches oracle
- Oracle text in code: "Equipped creature gets +1/+2 and has hexproof.\nEquip {3}" — matches (reminder text omission is standard)
- Continuous effects: ModifyPT {power: 1, toughness: 2, scope: Attached} + GrantKeyword {Hexproof, scope: Attached} — correct
- Equip ability: costs {3}, sorcery_speed_only: true, targets CreatureWithFilter(YouControl) — correct
- on_activate_ability: sets attached_to on the equipment to the target creature — correct
- on_resolve: moves to battlefield, sets is_equipment = true — correct
- is_valid_target: checks battlefield, has power (is creature), controller == caster — correct

### Tricky interactions checked (min 3)
1. **Hexproof does not block own equip**: The engine's `can_be_targeted` checks `controller != caster` for hexproof. Since equip uses the equipment controller as caster and targets only creatures you control (same controller), hexproof never blocks equipping your own creature. Verified in engine.rs:758-768.
2. **Equipment detaches when creature dies**: SBA in sba.rs correctly detaches equipment (sets attached_to = None) when the attached creature leaves the battlefield, but keeps the equipment on the battlefield. Test `equipment_detaches_when_creature_dies` confirms this.
3. **Equipment can be moved between creatures**: Re-equipping to a different creature correctly updates attached_to via on_activate_ability. The old creature loses the +1/+2 and hexproof because EffectScope::Attached checks the current attached_to. Test `equipment_can_be_moved_to_different_creature` confirms this pattern (using Cobbled Wings, same mechanics).
4. **Opponent cannot target hexproof creature with spells**: `can_be_targeted` is called in all spell targeting paths and activated ability targeting paths in engine.rs, ensuring an opponent's removal spell cannot target a creature equipped with Mask of Avacyn.

### Test coverage
- `mask_of_avacyn_has_correct_data` — verifies card data (name, types, subtypes, mana cost)
- `mask_of_avacyn_grants_pt_and_hexproof` — verifies equipping grants +1/+2 and hexproof to a 2/2 creature (becomes 3/4 with hexproof)
- General equipment tests also cover relevant mechanics (detach on death, re-equip, cast-and-equip flow)
- No dedicated test for hexproof-blocks-opponent-targeting on an equipped creature (coverage gap, but engine-level hexproof is well-tested via other cards like Invisible Stalker, Geist of Saint Traft)
