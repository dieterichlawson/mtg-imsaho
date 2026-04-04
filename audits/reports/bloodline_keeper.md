## Audit — 2026-04-02 21:23

**Oracle text source**: Scryfall API (cached 2026-04-01)

**Oracle text (front face — Bloodline Keeper)**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```
**Type line (front)**: Creature — Vampire (3/3)

**Oracle text (back face — Lord of Lineage)**:
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```
**Type line (back)**: Creature — Vampire (5/5)

**Status**: PASS

### Code issues

1. **Anti-pattern: `CardRegistry::with_all_cards()` constructed inline** (line 112). The `activated_abilities` method already receives a `registry: &CardRegistry` parameter (named `_registry`), but ignores it and constructs a fresh `CardRegistry::with_all_cards()` to pass to `count_vampires`. This is a performance concern (allocates a new registry on every call) but not a correctness issue. The parameter should be used instead.

2. **Minor: manual transform instead of `apply_transform` helper** (lines 154-158). The `on_activate_ability` for ability index 1 manually sets `obj.is_transformed = true` and `obj.name = "Lord of Lineage"` instead of calling `helpers::apply_transform()`. This works correctly because both faces share the same subtypes (`Vampire`) and keywords (`Flying`), and the engine resolves keywords and subtypes from back-face card data when `is_transformed` is true. However, it is inconsistent with other DFCs that use the helper.

3. **Minor: oracle text string says "Transform Bloodline Keeper" vs current oracle "Transform this creature"** (line 45). This is a WotC templating update and does not affect gameplay since the transform logic is handled by `on_activate_ability`, not parsed from the oracle text string.

None of these issues cause incorrect card behavior.

### Tricky interactions checked

1. **Transform condition counts Bloodline Keeper itself as a Vampire**: PASS. `count_vampires` (lines 16-28) iterates all battlefield objects for the controller and checks "Vampire" subtype. Bloodline Keeper itself has subtype "Vampire", so it is counted. With 4 other Vampires + itself = 5, the threshold is met.

2. **Lord of Lineage +2/+2 applies only to OTHER Vampires you control**: PASS. Uses `EffectScope::GlobalOther` with `CreatureFilter::And([You, HasSubtype("Vampire")])` (line 70). The engine's `effect_applies_to` correctly excludes the source from `GlobalOther` scope via `creature_id != source_id` (state.rs:720).

3. **Transform while tapped is legal**: PASS. The transform ability (ability_index 1) has `requires_tap: false` (line 119), so it can be activated while the creature is tapped. This matches the oracle text which only requires {B}, not {T}.

4. **Dynamic P/T for back face (5/5) is not double-counted**: PASS. `dynamic_pt` returns `Some((5,5))` when transformed (line 82-86). The engine uses this as the base P/T (state.rs:868,912). The `continuous_pt_mods` function only adds `dynamic_pt` contributions from attached auras (`source.attached_to == Some(creature_id)`), so Bloodline Keeper's own `dynamic_pt` is not double-counted as a continuous modifier.

5. **Token creation matches oracle**: PASS. Token is created with name "Vampire", P/T 2/2, colors `[Black]`, types `[Creature]`, keywords `[Flying]`, subtypes `["Vampire"]`. Matches "Create a 2/2 black Vampire creature token with flying."

6. **Vampire tokens count toward transform threshold**: PASS. Tokens have `subtypes: vec!["Vampire".into()]` set on the object. `count_vampires` checks `o.subtypes.iter().any(|s| s == "Vampire")` first (line 21), which matches tokens directly.

7. **Continuous effects only apply when on Lord of Lineage face**: PASS. `continuous_pt_mods` checks `source.is_transformed` and uses `back_face_data().continuous_effects` only when true (state.rs:746-748). Front face has `continuous_effects: vec![]`.

### Test coverage

- Token creation (front face tap ability): `tier15_cards.rs:1429` (`bloodline_keeper_creates_vampire_token`) — checks name, power, toughness
- Token color (black): NOT TESTED
- Token flying keyword: NOT TESTED
- Transform condition (5+ Vampires): NOT TESTED
- Transform into Lord of Lineage: NOT TESTED
- Lord of Lineage +2/+2 buff to other Vampires: NOT TESTED
- Lord of Lineage token creation: NOT TESTED
- Transform while tapped: NOT TESTED

---

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
**Type line**: Creature — Vampire
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text (front face)**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```
**Type line (front)**: Creature — Vampire (3/3)
**Oracle text (back face — Lord of Lineage)**:
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```
**Type line (back)**: Creature — Vampire (5/5)
**Status**: PASS

### Code issues
No issues found.

Minor cosmetic note (not a functional issue): The stored `oracle_text` at `bloodline_keeper.rs:45` says `{B}: Transform Bloodline Keeper` while current Scryfall oracle text says `{B}: Transform this creature`. This is a templating update by WotC and does not affect gameplay behavior since the transform logic is handled by `on_activate_ability`, not parsed from the oracle text string.

Minor style note (not a functional issue): The transform in `on_activate_ability` (line 154-158) manually sets `is_transformed` and `name` instead of using `helpers::apply_transform()`. This works correctly because both faces share the same subtypes (`Vampire`) and keywords (`Flying`), so no data is lost. However, using the helper would be more consistent with other DFCs.

### Tricky interactions checked
- Transform condition counts Bloodline Keeper itself as a Vampire: **pass** — `count_vampires` counts all objects in battlefield with "Vampire" subtype, including itself (line 16-28).
- Lord of Lineage +2/+2 applies only to OTHER Vampires you control: **pass** — uses `EffectScope::GlobalOther` with `CreatureFilter::And([You, HasSubtype("Vampire")])` (line 70), and the engine's `continuous_pt_mods` correctly excludes the source from `GlobalOther` scope (state.rs:720).
- Token creation creates correct token (2/2 black Vampire with flying): **pass** — `create_token_with_subtypes` called with name "Vampire", power 2, toughness 2, colors `[Black]`, types `[Creature]`, keywords `[Flying]`, subtypes `["Vampire"]` (line 139-147).
- Back face P/T (5/5) handled correctly: **pass** — `dynamic_pt` returns `Some((5,5))` when `is_transformed` is true (line 82-86), and the engine uses `dynamic_pt` as base P/T (state.rs:868,912).
- Continuous effects only apply when transformed (Lord of Lineage face): **pass** — `continuous_pt_mods` checks `source.is_transformed` and uses `back_face_data().continuous_effects` only when true (state.rs:746-748).
- Transform ability only available on front face: **pass** — ability index 1 is only added when `!is_transformed` (line 111).
- Both faces have tap-to-create-token ability: **pass** — ability index 0 is always added regardless of `is_transformed` (line 99-108).

### Test coverage
- Token creation (front face tap ability): `tier15_cards.rs:1429` (`bloodline_keeper_creates_vampire_token`)
- Token color (black): NOT TESTED (test checks name, power, toughness but not color)
- Token flying keyword: NOT TESTED
- Transform condition (5+ Vampires): NOT TESTED
- Transform into Lord of Lineage: NOT TESTED
- Lord of Lineage +2/+2 buff to other Vampires: NOT TESTED
- Lord of Lineage token creation: NOT TESTED
