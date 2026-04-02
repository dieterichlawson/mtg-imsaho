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
