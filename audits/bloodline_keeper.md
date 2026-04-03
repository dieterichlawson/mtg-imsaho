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
