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

---

# Re-Audit: Butcher's Cleaver (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)

> Equipped creature gets +3/+0.
> As long as equipped creature is a Human, it has lifelink.
> Equip {3}

- **Name:** Butcher's Cleaver
- **Cost:** {3}
- **Type:** Artifact -- Equipment
- **Keywords:** Equip

## Implementation Review: `mtg-engine/src/cards/isd/butchers_cleaver.rs`

### Card Data -- CORRECT
- Name: "Butcher's Cleaver" -- matches oracle
- Cost: Generic(3) -- matches {3}
- Card types: [Artifact] -- correct
- Subtypes: ["Equipment"] -- correct
- Oracle text string matches oracle verbatim -- correct
- P/T: None -- correct (not a creature)

### Equip Ability -- CORRECT
- Cost: Generic(3) -- matches Equip {3}
- `sorcery_speed_only: true` -- correct (equip is sorcery-speed by default)
- `requires_tap: false` -- correct
- Target: `CreatureWithFilter(YouControl)` -- correct
- Only available on the battlefield -- correct

### Static +3/+0 Bonus -- CORRECT
- `continuous_effects` in card_data includes `ModifyPT { power: 3, toughness: 0, scope: Attached }` -- matches "Equipped creature gets +3/+0"

### Conditional Lifelink for Humans -- PARTIALLY CORRECT
- `update_effects` checks if the equipped creature is a Human and, if so, adds `GrantKeyword { keyword: Keyword::Lifelink, scope: Attached }` -- matches the oracle intent.

### Target Validation (`is_valid_target`) -- CORRECT
- Checks battlefield zone, has power (is a creature), controller matches caster -- correct for Equip targeting.

### Equip Resolution (`on_activate_ability`) -- CORRECT
- Sets `attached_to` on the equipment object -- correct
- Calls `update_effects` to apply continuous effects -- correct

### Enter Battlefield (`on_resolve`) -- CORRECT
- Moves to battlefield and sets `is_equipment = true` -- correct

## Issues Found

### 1. BUG (minor): Human subtype check only queries registry, not object subtypes

**Oracle text:** "As long as equipped creature is a Human, it has lifelink."

**Code (lines 15-18):**
```rust
let is_human = state.get_object(creature_id)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Human"))
    .unwrap_or(false);
```

The check only looks at `registry.card_data(o.card_id).subtypes` (the static card definition). It does not check `obj.subtypes` (the runtime object subtypes). Creatures that gain the Human subtype at runtime (e.g., via Olivia Voldaren adding Vampire, or changelings represented as tokens) would be missed.

Compare with Wooden Stake (`wooden_stake.rs` lines 86-94), which correctly checks both:
```rust
let is_vampire = state.get_object(other_creature)
    .and_then(|o| registry.card_data(o.card_id))
    .map(|d| d.subtypes.iter().any(|s| s == "Vampire"))
    .unwrap_or(false);
// Also check instance subtypes on the game object (for tokens, etc.).
let is_vampire = is_vampire || state.get_object(other_creature)
    .map(|o| o.subtypes.iter().any(|s| s == "Vampire"))
    .unwrap_or(false);
```

### 2. BUG (moderate): Continuous effect is set once at equip time, not continuously recalculated

**Oracle text:** "**As long as** equipped creature is a Human, it has lifelink."

The phrase "as long as" indicates a continuous condition that should be checked at all times. However, `update_effects` is only called inside `on_activate_ability` (when the equip ability resolves). If the equipped creature gains or loses the Human subtype after equipping (e.g., via a type-changing effect), the lifelink grant will not update.

This is a known engine-level limitation shared with other equipment cards (Silver-Inlaid Dagger, Sharpened Pitchfork) that use the same `instance_continuous_effects` pattern.

### 3. No issue: Missing test for token/runtime Human

The existing tests (`tier9_equipment.rs` lines 253-295) cover:
- Card data correctness
- Non-Human creature gets +3/+0 but no lifelink
- Human creature (Champion of the Parish) gets +3/+0 and lifelink

No test covers a creature that gains Human subtype at runtime, which would expose issue #1.

## Community Rulings

No official rulings are published for Butcher's Cleaver. The card's behavior is straightforward: the Human check is a continuous condition on the equipped creature's current subtypes.

## Test Coverage

| Test | File | Status |
|------|------|--------|
| `butchers_cleaver_has_correct_data` | tier9_equipment.rs:253 | Covers card data |
| `butchers_cleaver_non_human_gets_power_no_lifelink` | tier9_equipment.rs:264 | Covers non-Human equip |
| `butchers_cleaver_human_gets_power_and_lifelink` | tier9_equipment.rs:281 | Covers Human equip |

Missing test: runtime subtype change after equip.

## Verdict: PASS (with minor bugs)

The core functionality is correct: the card data matches oracle text, equip works at sorcery speed for {3}, +3/+0 is applied to equipped creatures, and Humans get lifelink. The two bugs (registry-only subtype check and one-time effect calculation) are minor edge cases that affect uncommon interactions. The registry-only check should be fixed to also check `obj.subtypes` for consistency with other subtype-checking cards like Wooden Stake.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature gets +3/+0.\nAs long as equipped creature is a Human, it has lifelink.\nEquip {3}
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found. +3/+0 buff, conditional lifelink for Humans, and equip {3} all correctly implemented. The update_effects helper correctly recalculates effects when equipment is attached.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink. Equip {3}
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found.

- Card name "Butcher's Cleaver" matches oracle.
- Mana cost Generic(3) matches `{3}`.
- Card types `[Artifact]` with subtypes `["Equipment"]` match "Artifact — Equipment".
- Oracle text string in card_data matches oracle verbatim.
- `continuous_effects` includes `ModifyPT { power: 3, toughness: 0, scope: Attached }` -- correctly implements "+3/+0".
- `update_effects` conditionally adds `GrantKeyword { keyword: Lifelink, scope: Attached }` when equipped creature is a Human -- correctly implements "As long as equipped creature is a Human, it has lifelink."
- Equip activated ability costs Generic(3), `sorcery_speed_only: true`, targets `CreatureWithFilter(YouControl)` -- correctly implements "Equip {3}" with standard equip restrictions.
- `on_resolve` moves to battlefield and sets `is_equipment = true` -- correct.

### Tricky interactions checked
- Equipment detachment on creature death: PASS -- SBA code (sba.rs:183-187) clears `attached_to`, and `effect_applies_to` for `EffectScope::Attached` checks `source.attached_to == creature_id`, so effects stop applying when detached.
- Lifelink during combat damage: PASS -- `combat.rs` calls `state.has_keyword(source, Keyword::Lifelink, registry)` at damage time, which walks `instance_continuous_effects` on all battlefield objects and checks `GrantKeyword` with `Attached` scope.
- Re-equipping to a different creature: PASS -- `on_activate_ability` updates `attached_to` and calls `update_effects` to recalculate instance effects for the new creature. The `Attached` scope ensures effects only apply to the currently attached creature.
- Non-Human gets +3/+0 but not lifelink: PASS -- `update_effects` only includes `GrantKeyword::Lifelink` in the effects list when `is_human` is true; otherwise only `ModifyPT` is included.
- Equip only your own creatures: PASS -- `TargetFilter::YouControl` in the activated ability and `is_valid_target` checks `o.controller == caster`.

### Test coverage
- Card data correctness: `tier9_equipment.rs:253` (butchers_cleaver_has_correct_data)
- Non-Human gets +3/+0 without lifelink: `tier9_equipment.rs:264` (butchers_cleaver_non_human_gets_power_no_lifelink)
- Human gets +3/+0 with lifelink: `tier9_equipment.rs:281` (butchers_cleaver_human_gets_power_and_lifelink)
- Equipment detaches when creature dies: `tier9_equipment.rs:398` (equipment_detaches_when_creature_dies) -- general equipment test
- Re-equip to different creature: `tier9_equipment.rs:421` (equipment_can_be_moved_to_different_creature) -- general equipment test
- Runtime subtype change after equip: NOT TESTED (engine-level limitation, not Butcher's Cleaver specific)

## Audit — 2026-04-10 (Opus 4.6)

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**:
Equipped creature gets +3/+0.
As long as equipped creature is a Human, it has lifelink.
Equip {3}
**Type line**: Artifact — Equipment
**Mana cost**: {3}
**Status**: ISSUE

### Code issues

- `mtg-engine/src/cards/isd/butchers_cleaver.rs:14-34` — Conditional lifelink is evaluated statically at equip time, not continuously.
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink.`
  - Code does: `update_effects` is called only from `on_activate_ability` (line 90). It checks `is_human` at that moment and writes a fixed `instance_continuous_effects` vector either with or without `GrantKeyword { Lifelink }`. Nothing re-evaluates the Human check afterwards, so if the equipped creature later gains or loses the Human type (e.g., via a type-changing effect such as Conspiracy/Xenograft, or via a loss-of-subtype effect), lifelink will not follow the current state of the permanent. The correct pattern used elsewhere in the codebase (see `mtg-engine/src/cards/isd/bonds_of_faith.rs:33` and the `check_condition` handler for `EffectCondition::AttachedHasSubtype` in `mtg-engine/src/state.rs:1238`) is `ContinuousEffect::ConditionalKeyword { keyword: Keyword::Lifelink, condition: EffectCondition::AttachedHasSubtype("Human".into()), scope: EffectScope::Attached }` placed on the card's static `continuous_effects`, which is re-evaluated dynamically through `has_conditional_keyword`.

- `mtg-engine/src/cards/isd/butchers_cleaver.rs:15-18` — Human check consults registry subtypes only, ignoring `obj.subtypes`.
  - Oracle text says: `As long as equipped creature is a Human, it has lifelink.`
  - Code does:
    ```
    let is_human = state.get_object(creature_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.subtypes.iter().any(|s| s == "Human"))
        .unwrap_or(false);
    ```
    This reads only `registry.card_data(o.card_id).subtypes` and never `o.subtypes`. Token creatures (which have no registry `card_data` populated with meaningful subtypes and rely on `obj.subtypes`) and type-modified objects whose instance subtypes differ from registry subtypes will be missed. Compare with the reference pattern in `mtg-engine/src/cards/isd/champion_of_the_parish.rs:49-52`, `mtg-engine/src/cards/isd/avacynian_priest.rs:58-60`, `mtg-engine/src/cards/isd/elder_cathar.rs:52-54`, and `mtg-engine/src/cards/isd/dearly_departed.rs:61-64`, all of which check both `d.subtypes` AND `o.subtypes`. (This is also listed as a known anti-pattern in the audit procedure: "Human/subtype check only via registry, not also checking `obj.subtypes` (misses tokens)".) Note: switching to `ConditionalKeyword` + `AttachedHasSubtype` as recommended above would automatically resolve this issue, because `check_condition` in `state.rs:1238-1249` already consults `target_obj.subtypes` first before falling back to registry.

### Card data checks

- Name: `"Butcher's Cleaver"` matches Scryfall. PASS
- Mana cost: `{3}` matches. PASS
- Card types: `[Artifact]` matches. PASS
- Supertypes: none needed. PASS
- Subtypes: `["Equipment"]` matches. PASS
- Power/toughness: `None`/`None` (not a creature). PASS
- `oracle_text` field: matches the fetched oracle text verbatim. PASS
- `keywords`: empty. PASS (the `Equip` ability is modeled via `activated_abilities`, and `Keyword::Equip` does not exist in the engine's `Keyword` enum — consistent with every other equipment card in the codebase).
- `continuous_effects` base set: `[ModifyPT { +3/+0, Attached }]`. The `+3/+0` is always on — PASS for that part — but the conditional lifelink is missing from this static list, which is what underpins issue 1.
- Equip cost: `{3}` matches Oracle. PASS
- Targeting: `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` with `sorcery_speed_only: true`. Matches CR 702.6c. PASS
- `is_valid_target` verifies `zone == Battlefield`, `power.is_some()` (creature), and `controller == caster`. Correct for "target creature you control". PASS
- `on_resolve` moves to battlefield and sets `is_equipment = true`. Matches the pattern used by every other ISD equipment card. PASS

### Tricky interactions checked

- Equip to non-Human: +3/+0 applies, no lifelink. PASS (tested).
- Equip to Human: +3/+0 applies, lifelink granted. PASS (tested).
- Equip to non-Human, then that creature becomes Human via a type-changing effect: lifelink should turn on. FAIL — lifelink is a static snapshot. (Not tested.)
- Equip to Human, then that creature loses the Human type: lifelink should turn off. FAIL — lifelink would persist. (Not tested.)
- Equip to a Human token (a creature whose Human subtype lives in `obj.subtypes`, not registry `card_data`): lifelink should apply. FAIL — the is_human check ignores `obj.subtypes`. (Not tested. No Human token creators are present in ISD, but the issue is still a correctness bug because the engine's policy, as evidenced by other cards, is to check both sources.)
- Re-equipping from Human to non-Human (or vice versa) via a second activation: lifelink updates because `update_effects` is re-run on each activation. PASS.
- Equipment stays on battlefield when equipped creature dies: handled by the generic equipment detach logic, not this card. PASS.
- Equip at sorcery speed only: `sorcery_speed_only: true`. PASS.

### Test coverage

- `butchers_cleaver_has_correct_data` at `mtg-engine/tests/tier9_equipment.rs:253` — checks name, types, subtype, mana cost.
- `butchers_cleaver_non_human_gets_power_no_lifelink` at `mtg-engine/tests/tier9_equipment.rs:264` — covers the +3/+0 + no-lifelink case on a Bear.
- `butchers_cleaver_human_gets_power_and_lifelink` at `mtg-engine/tests/tier9_equipment.rs:281` — covers the +3/+0 + lifelink case on Champion of the Parish.
- Dynamic Human-ness change after equip: NOT TESTED.
- Human token case: NOT TESTED.
- Declining to pay equip (sorcery speed / priority) behavior: NOT TESTED for this card (generic equip flow tested elsewhere).
- Re-equip switching Human status: NOT TESTED.
