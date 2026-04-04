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

---

# Re-Audit: Cobbled Wings (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)
> Name: Cobbled Wings
> Mana Cost: {2}
> Type Line: Artifact — Equipment
> Oracle Text: Equipped creature has flying.
> Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
> Keywords: Equip

## Implementation File
`mtg-engine/src/cards/isd/cobbled_wings.rs`

## Card Data Checks
- **Name**: "Cobbled Wings" — correct.
- **Mana cost**: `Generic(2)` — correct, matches {2}.
- **Card types**: `[Artifact]` — correct.
- **Subtypes**: `["Equipment"]` — correct.
- **Power/Toughness**: None — correct (not a creature).
- **Oracle text string**: `"Equipped creature has flying.\nEquip {1}"` — correct.

## Equip Ability Checks
- **Equip cost**: `Generic(1)` — correct, matches Equip {1}.
- **Sorcery speed only**: `true` — correct per equip rules.
- **Target requirement**: `CreatureWithFilter(TargetFilter::YouControl)` — correct.
- **Requires tap**: `false` — correct, equip does not require tapping the equipment.
- **Only available on battlefield**: Gated by `zone == Zone::Battlefield` check — correct.

## Continuous Effect Checks
- `GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached }` — correct, grants flying to equipped creature only.

## Resolve Behavior
- Moves to battlefield and sets `is_equipment = true` — correct.

## Target Validation
- Checks: on battlefield, has power (creature proxy), controller matches caster — correct.

## Test Coverage (`mtg-engine/tests/tier9_equipment.rs`)
- `cobbled_wings_has_correct_data` — card data assertions.
- `cobbled_wings_enters_as_equipment` — enters battlefield with is_equipment flag.
- `cobbled_wings_grants_flying` — flying keyword granted after equipping.
- `cobbled_wings_equip_only_your_creatures` — cannot equip opponent's creatures.
- Shared tests cover detach on creature death and re-equip to different creature.

## LLM Player (`mtg-player/src/llm.rs`)
No references found. No special handling needed for this simple equipment card.

## Issues Found
None. The implementation faithfully matches the oracle text.

## Verdict
PASS — No discrepancies found.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:41
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Equipped creature has flying.
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
None. All card data fields match oracle text exactly:
- Name: "Cobbled Wings" -- correct
- Mana cost: Generic(2) -- matches {2}
- Card types: [Artifact] -- correct
- Subtypes: ["Equipment"] -- correct
- Oracle text string: "Equipped creature has flying.\nEquip {1}" -- correct
- Continuous effect: GrantKeyword Flying with EffectScope::Attached -- correctly grants flying to equipped creature only
- Equip cost: Generic(1) -- matches Equip {1}
- Sorcery speed only: true -- correct per equip rules
- Target: CreatureWithFilter(YouControl) -- correct, equip only targets your creatures
- on_resolve sets is_equipment = true and moves to battlefield -- correct
- on_activate_ability sets attached_to on the equipment -- correct

### Tricky interactions checked (min 3)
1. **Equipment detaches when creature dies**: Verified by `equipment_detaches_when_creature_dies` test -- equipment stays on battlefield with attached_to = None after creature goes to graveyard via SBA.
2. **Re-equip to different creature**: Verified by `equipment_can_be_moved_to_different_creature` test -- flying transfers from first creature to second when re-equipped.
3. **Cannot equip opponent's creatures**: Verified by `cobbled_wings_equip_only_your_creatures` test -- no equip actions generated when only opponent has creatures. Target validation also checks controller == caster.
4. **Full cast-then-equip lifecycle**: Verified by `equipment_cast_and_equip_full_flow` test -- cast enters as unattached equipment, then equip grants flying.
5. **EffectScope::Attached resolution**: Engine checks source's `attached_to` field matches the creature being evaluated, so flying only applies to the equipped creature, not all creatures.

### Test coverage
All 22 tests in `mtg-engine/tests/tier9_equipment.rs` pass. Cobbled Wings has 4 dedicated tests plus 3 shared equipment-mechanics tests that use Cobbled Wings:
- `cobbled_wings_has_correct_data` -- card data assertions
- `cobbled_wings_enters_as_equipment` -- enters battlefield with is_equipment flag
- `cobbled_wings_grants_flying` -- flying keyword granted after equipping
- `cobbled_wings_equip_only_your_creatures` -- cannot equip opponent's creatures
- `equipment_detaches_when_creature_dies` -- uses Cobbled Wings
- `equipment_can_be_moved_to_different_creature` -- uses Cobbled Wings
- `equipment_cast_and_equip_full_flow` -- uses Cobbled Wings
