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


---

## Audit — 2026-04-03 21:31

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: 
Front face: Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.

Back face: Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.

**Type line**: Creature — Vampire // Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Transform ability only available when controlling 5+ vampires: PASS
- Bloodline Keeper counts itself for vampire requirement: PASS  
- Transform ability disappears once transformed: PASS
- Lord of Lineage +2/+2 buff applies to other vampires you control: PASS
- Both faces can create 2/2 vampire tokens with flying: PASS
- Transform preserves tapped/untapped status: PASS
- Multiple transform activations handled correctly: PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Token creation ability: `mtg-engine/tests/tier15_cards.rs:1428`
- Transform ability when controlling 5+ vampires: NOT TESTED
- Transform restriction when controlling <5 vampires: NOT TESTED  
- Lord of Lineage +2/+2 continuous effect: NOT TESTED
- Vampire counting logic (including tokens): NOT TESTED
- Transform ability disappearing after transformation: NOT TESTED
- Multiple activation behavior: NOT TESTED


## Audit — 2026-04-10 (Opus 4.6)

**Oracle text source**: Oracle cache (`python3 scripts/oracle_lookup.py lookup "Bloodline Keeper"`, Scryfall API, cached 2026-04-01)

**Oracle text (front — Bloodline Keeper)**:
```
Flying
{T}: Create a 2/2 black Vampire creature token with flying.
{B}: Transform this creature. Activate only if you control five or more Vampires.
```

**Oracle text (back — Lord of Lineage)**:
```
Flying
Other Vampire creatures you control get +2/+2.
{T}: Create a 2/2 black Vampire creature token with flying.
```

**Type line (both faces)**: `Creature — Vampire`
**Mana cost**: `{2}{B}{B}`
**P/T**: front 3/3, back 5/5

**Status**: ISSUE (anti-patterns; no Oracle text mismatch)

### Code issues

- `mtg-engine/src/cards/isd/bloodline_keeper.rs:112` — `count_vampires` is called with a freshly built `CardRegistry::with_all_cards()` instead of using the `_registry: &CardRegistry` argument already passed into `activated_abilities`. Rebuilding the full registry on every call to `activated_abilities` (which is called frequently from the UI / priority loops) is wasteful and creates a divergent registry instance, violating the "single registry" pattern every other card follows.
  - Code does: `let vampire_count = Self::count_vampires(state, controller, &CardRegistry::with_all_cards());`
  - Anti-pattern; the underscore-prefixed `_registry` parameter is the correct object to pass.

- `mtg-engine/src/cards/isd/bloodline_keeper.rs:155-158` — Transform is implemented by directly mutating `obj.is_transformed = true` and `obj.name = "Lord of Lineage"` rather than calling the shared `helpers::apply_transform(state, object_id, registry)`. The helper also refreshes `obj.keywords` and `obj.subtypes` from the back face. For Bloodline Keeper specifically both faces share `Flying` and subtype `Vampire`, so no observable bug manifests today, but this diverges from the engine's official transform pipeline and is fragile if the card's subtypes/keywords ever diverge or if another subsystem starts reading those object fields.
  - Code does: `if let Some(obj) = state.get_object_mut(object_id) { obj.is_transformed = true; obj.name = "Lord of Lineage".into(); }`
  - Preferred: `crate::cards::helpers::apply_transform(state, object_id, registry);` (consistent with the `helpers` facade even though other ISD DFCs here also inline the flip).

- `mtg-engine/src/cards/isd/bloodline_keeper.rs:101` — Ability description omits "black" from the token. Oracle says "Create a 2/2 **black** Vampire creature token with flying" but the description string shown in the UI/stack logs is `"{T}: Create a 2/2 Vampire token with flying"`. The token is created correctly (colors: `vec![Color::Black]`), this is only the human-readable description. Cosmetic, but misleading on the stack UI.
  - Oracle text says: `{T}: Create a 2/2 black Vampire creature token with flying.`
  - Code does: `description: "{T}: Create a 2/2 Vampire token with flying".into(),`

- `mtg-engine/src/cards/isd/bloodline_keeper.rs:111-125` — "Activate only if you control five or more Vampires" is implemented by hiding ability index 1 from `activated_abilities` entirely when the condition fails. Per rules, the ability still **exists** on the card; only its activation is restricted. This matters for effects that enumerate/printed-abilities. This matches the engine's convention for sorcery-speed-only style restrictions (there is no `ActivationRestriction` mechanism), so it is flagged as an anti-pattern / engine-wide gap rather than a card-specific bug.

### Tricky interactions checked

- Self-exclusion on Lord of Lineage anthem (`Other Vampire creatures you control get +2/+2`): PASS. `EffectScope::GlobalOther` correctly excludes the source (see `state.rs:731`, `creature_id != source_id`).
- Continuous effect sourced from transformed card: PASS. `continuous_pt_mods` uses `behavior.back_face_data().continuous_effects` when `source.is_transformed`.
- Vampire count includes Bloodline Keeper itself: PASS. The filter iterates all battlefield objects the controller controls, including self; 5 Vampires means 4 others + Bloodline Keeper is enough, matching rules.
- Vampire count works on token Vampires: PASS. `count_vampires` checks `o.subtypes` first (set on tokens) before falling back to registry.
- Transformed Bloodline Keeper counted as Vampire (stale-subtype edge case): PASS for this card because front and back both have subtype `Vampire`; the registry-based lookup would agree either way.
- Dynamic P/T after transform: PASS. `dynamic_pt` returns `Some((5, 5))` when `is_transformed`, overriding `obj.power/toughness` in `effective_power/toughness`.
- `{B}: Transform` summoning-sickness: PASS. Cost has no `{T}` / `{Q}`, so rule 302.1 does not restrict it; engine respects `requires_tap: false`.
- `{T}: token` ability is present on back face Lord of Lineage: PASS. `activated_abilities` pushes ability 0 regardless of `is_transformed`, and ability 1 (Transform) is gated behind `!is_transformed`.
- Token receives correct characteristics (black, 2/2, Vampire, Flying): PASS. `create_token_with_subtypes("Vampire", …, 2, 2, vec![Color::Black], vec![CardType::Creature], vec![Keyword::Flying], vec!["Vampire".into()], …)`.
- Ability cost `{B}` matches oracle: PASS.
- Transform is one-way (no back-to-front ability on Lord of Lineage): PASS. Back face has no transform ability.

### Test coverage

- `{T}: create 2/2 black Vampire token with flying` (front face): `mtg-engine/tests/tier15_cards.rs:1462` (`bloodline_keeper_creates_vampire_token`). Verifies a token is created with power/toughness 2/2 and name "Vampire". Does NOT verify color=Black, does NOT verify Flying, does NOT verify subtype "Vampire" on the token.
- `{B}: Transform` activation restriction (< 5 Vampires cannot, ≥ 5 can): NOT TESTED.
- Transformation actually flips `is_transformed` and renames to "Lord of Lineage": NOT TESTED.
- After transform, effective P/T becomes 5/5 via `dynamic_pt`: NOT TESTED.
- Lord of Lineage's anthem (`Other Vampire creatures you control get +2/+2`): NOT TESTED.
- Self-exclusion of anthem (Lord of Lineage stays 5/5, not 7/7): NOT TESTED.
- Back face `{T}: create Vampire token` still works after transform: NOT TESTED.
- Back face does NOT expose the `{B}: Transform` ability: NOT TESTED.
- Vampire count correctly includes token Vampires (created by Bloodline Keeper itself): NOT TESTED.
- Vampire count correctly includes Bloodline Keeper itself (needs only 4 other Vampires): NOT TESTED.
