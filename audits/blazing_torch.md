# Audit: Blazing Torch

## Oracle (Scryfall)
- **Name:** Blazing Torch
- **Cost:** {1}
- **Type:** Artifact — Equipment
- **Oracle:** Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/blazing_torch.rs`
- **Name:** Blazing Torch ✅
- **Cost:** {1} ✅
- **Type:** Artifact ✅
- **Subtypes:** Equipment ✅
- **Block restriction:** Vampires and Zombies cannot block ✅
- **Equip cost:** {1}, sorcery speed ✅
- **Damage ability:** {T}, Sacrifice, deal 2 to any target ✅
- **NonCombatDamageDealt events:** emitted ✅
- **Sacrifice:** calls `crate::destruction::sacrifice` ✅

## BUG: Missing `damaged_by` tracking for creature damage
In `on_activate_ability` ability_index 1, when dealing damage to a creature target (line 126-128), the code does:
```rust
obj.damage_marked += 2;
```
But does NOT do:
```rust
obj.damaged_by.push(...);
```
Every other card that deals non-combat damage to creatures (Balefire Dragon, Blasphemous Act, Daybreak Ranger, Olivia Voldaren, Ashmouth Hound, helpers.rs) pushes to `damaged_by`. This omission means damage source tracking is broken for Blazing Torch — affecting interactions like Abattoir Ghoul's life gain on lethal damage.

## BUG: Damage source should be the Torch, not the creature
The oracle text says "Blazing Torch deals 2 damage" — the damage source should be the Torch object, not the equipped creature. The comment on line 122 acknowledges this ("Source is the torch (flavor), but we use creature ID") but uses `object_id` (the creature) as source. Since the torch is sacrificed before damage, using the creature as source is a pragmatic choice, but the `NonCombatDamageDealt` event's source is technically wrong per oracle text. This matters for effects that care about what dealt the damage (e.g., protection from creatures would incorrectly prevent this damage if the source is the creature, but shouldn't since the Torch is an artifact).

## Verdict: FAIL — 2 issues found
1. **Missing `damaged_by.push()`** when dealing damage to creatures (real bug)
2. **Wrong damage source** — uses creature ID instead of torch ID (acknowledged in comment, but technically incorrect)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1}
**Scryfall type line**: Artifact -- Equipment
**Status**: ISSUE

Findings:
1. **Mana cost {1}**: Correct.
2. **Type (Artifact -- Equipment)**: Correct. `card_types: [Artifact]`, `subtypes: ["Equipment"]`.
3. **Oracle text**: Matches Scryfall.
4. **Block restriction (Vampires/Zombies)**: Correctly implemented via `ContinuousEffect::BlockRestriction` with `CreatureFilter::Not(Or([HasSubtype("Vampire"), HasSubtype("Zombie")]))`.
5. **Equip {1}**: Correct. `ability_index: 0`, cost Generic(1), sorcery_speed_only: true.
6. **Damage ability**: Correctly requires tap, targets any target, deals 2 damage, sacrifices torch.
7. **`damaged_by` tracking**: Previous audit claimed this was missing, but re-reading the code at lines 128-129, `obj.damaged_by.push(object_id)` IS present. Previous audit finding #1 appears to be incorrect -- the code does track damaged_by.
8. **Damage source**: The `NonCombatDamageDealt` event and `damaged_by` both use `object_id` (the creature), not the torch. Per oracle, "Blazing Torch deals 2 damage" -- the source should be the torch. The code comment on line 122 acknowledges this. Since the torch is sacrificed before damage is dealt, using the torch's ID may cause issues (it's in the graveyard). This is a pragmatic trade-off but technically incorrect per oracle.
9. **Anti-patterns**: Uses `NonCombatDamageDealt` (correct, not combat damage). No `move_object(id, Zone::Graveyard)` for spells (it's a permanent).
10. **Tests**: Found in `mtg-engine/tests/tier9_cards.rs`.

Issues:
- Wrong damage source (creature ID instead of torch ID) -- acknowledged in code comment, technically incorrect per oracle text.

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/216/blazing-torch)
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1}
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Status**: ISSUE

Findings:
1. **Name**: "Blazing Torch" -- correct.
2. **Mana cost {1}**: Correct (Generic(1)).
3. **Type (Artifact — Equipment)**: Correct. `card_types: [Artifact]`, `subtypes: ["Equipment"]`.
4. **Block restriction (Vampires/Zombies)**: Correctly implemented via `ContinuousEffect::BlockRestriction` with `CreatureFilter::Not(Or([HasSubtype("Vampire"), HasSubtype("Zombie")]))`. Matches oracle.
5. **Equip {1}**: Correct. ability_index 0, cost Generic(1), sorcery_speed_only: true.
6. **Damage ability**: ability_index 1, requires_tap: true, target: AnyTarget, deals 2 damage. Sacrifice handled via `crate::destruction::sacrifice`. All correct.
7. **`damaged_by` tracking**: Present at line 129 (`obj.damaged_by.push(object_id)`). Correct.
8. **NonCombatDamageDealt**: Emitted for both creature and player targets (lines 130, 140). Correct.
9. **Damage source issue**: Both `damaged_by` and `NonCombatDamageDealt` use `object_id` (the equipped creature) as the source. Per oracle text and rulings, "the source of the damage is Blazing Torch, not the equipped creature." The code comment on line 122 acknowledges this. Since the torch is sacrificed before damage is dealt (line 118-119), using the torch's ID could be problematic (it's in the graveyard), but it's still technically the correct source. This affects protection interactions: a creature with protection from creatures should not prevent this damage (it's from an artifact), but with the current source being the creature, it would.
10. **Tests**: Found in `mtg-engine/tests/tier9_cards.rs`. Tests cover card data, damage ability grant, damage to player, damage to creature, and equip. All assertions match oracle text.

Issues:
- Wrong damage source: Uses equipped creature's ID instead of torch's ID as damage source. Per Scryfall rulings: "The source of the damage is Blazing Torch, not the equipped creature."

## Audit — 2026-04-01 14:38

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/216/blazing-torch)
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1}
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Status**: ISSUE

Findings:
1. **Name**: "Blazing Torch" -- correct.
2. **Mana cost {1}**: Correct (`ManaSymbol::Generic(1)`).
3. **Type (Artifact — Equipment)**: Correct. `card_types: [Artifact]`, `subtypes: ["Equipment"]`.
4. **Oracle text in code** (line 27): Matches Scryfall oracle text.
5. **Block restriction**: Oracle says: `Equipped creature can't be blocked by Vampires or Zombies.` Code implements: `ContinuousEffect::BlockRestriction` with `CreatureFilter::Not(Or([HasSubtype("Vampire"), HasSubtype("Zombie")]))` and `scope: EffectScope::Attached`. Correct.
6. **Equip {1}**: ability_index 0, cost Generic(1), sorcery_speed_only: true, target Creature. Correct.
7. **Damage ability**: ability_index 1, requires_tap: true, sacrifice handled manually, target AnyTarget, deals 2 damage. Correct structure.
8. **`damaged_by` tracking**: Present at line 129 (`obj.damaged_by.push(object_id)`). Correct.
9. **NonCombatDamageDealt**: Emitted for both creature (line 130) and player (line 140) targets. Correct.
10. **LifeChanged**: Emitted for player damage (line 145). Correct.
11. **Spell cleanup**: Not needed -- Equipment is a permanent that stays on battlefield. Correct.
12. **Tests**: No dedicated test file found. Previously noted in tier9_cards.rs.

Issue:
- **Wrong damage source** (file: `mtg-engine/src/cards/blazing_torch.rs`, lines 128-134):
  - Oracle text says: `Blazing Torch deals 2 damage to any target.`
  - Scryfall ruling says: "The source of the damage is Blazing Torch, not the equipped creature."
  - Code does: `obj.damaged_by.push(object_id)` and `source: object_id` in the NonCombatDamageDealt event, where `object_id` is the equipped creature (per comment on line 106: "object_id is the creature").
  - The damage source should be the Blazing Torch equipment object, not the equipped creature. This affects protection interactions (e.g., protection from creatures should not prevent this damage since the source is an artifact).

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
- [2009-10-01] If Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
**Status**: ISSUE

### Code issues

1. **Wrong damage source** (`mtg-engine/src/cards/isd/blazing_torch.rs`, lines 122-134):
   - Oracle text says: `Blazing Torch deals 2 damage to any target.`
   - Ruling says: "The source of the damage is Blazing Torch, not the equipped creature."
   - Code does: `obj.damaged_by.push(object_id)` and `source: object_id` in the NonCombatDamageDealt event, where `object_id` is the equipped creature (per comment on line 122: "Source is the torch (flavor), but we use creature ID"). The torch is sacrificed first (line 118-119) so its ID still exists but refers to a graveyard object. The source should be the torch ID, not the creature ID.

### Tricky interactions checked
- Block restriction for Vampires/Zombies: PASS (uses `CreatureFilter::Not(Or(...))` with `EffectScope::Attached`)
- Equip sorcery speed: PASS (sorcery_speed_only: true)
- Torch sacrifice on use: PASS (calls `crate::destruction::sacrifice`)
- NonCombatDamageDealt event: PASS (emitted for both creature and player targets)
- LifeChanged event for player damage: PASS
- damaged_by tracking: PASS (present at line 128) but uses wrong source (creature instead of torch)

### Test coverage
- Card data (mana cost, types, subtypes): `tier9_cards.rs:384` (blazing_torch_card_data)
- Grants damage ability to equipped creature: `tier9_cards.rs:394` (blazing_torch_grants_damage_ability)
- Deals 2 damage to player: `tier9_cards.rs:412` (blazing_torch_deals_damage_to_player)
- Deals 2 damage to creature: `tier9_cards.rs:444` (blazing_torch_deals_damage_to_creature)
- Equip ability: `tier9_cards.rs:470` (blazing_torch_equip_ability)
- Torch sacrificed after use: `tier9_cards.rs:438` (verified in deals_damage_to_player test)
- Cross-controller equip interaction (ruling): NOT TESTED
- Damage source is torch not creature (ruling): NOT TESTED

## Audit — 2026-04-01 13:35

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
- [2009-10-01] If Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
**Status**: PASS

### Code issues
No issues found.

The previous audit flagged a wrong damage source (creature ID instead of torch ID). This has been fixed. The current code (lines 107-119) finds the torch object ID before sacrificing it, stores it as `damage_source`, and uses that for both `damaged_by.push()` (line 130) and `NonCombatDamageDealt` event (lines 133, 143). This matches the ruling: "The source of the damage is Blazing Torch, not the equipped creature."

### Tricky interactions checked
- Block restriction for Vampires/Zombies: PASS (uses `CreatureFilter::Not(Or(...))` with `EffectScope::Attached`)
- Equip sorcery speed: PASS (sorcery_speed_only: true)
- Torch sacrifice on use: PASS (calls `crate::destruction::sacrifice`)
- Damage source is torch, not creature: PASS (fixed since last audit; `damage_source = torch_id.unwrap_or(object_id)`)
- NonCombatDamageDealt event: PASS (emitted for both creature and player targets)
- LifeChanged event for player damage: PASS

### Test coverage
- Card data (mana cost, types, subtypes): `tier9_cards.rs:384` (blazing_torch_card_data)
- Grants damage ability to equipped creature: `tier9_cards.rs:394` (blazing_torch_grants_damage_ability)
- Deals 2 damage to player: `tier9_cards.rs:412` (blazing_torch_deals_damage_to_player)
- Deals 2 damage to creature: `tier9_cards.rs:444` (blazing_torch_deals_damage_to_creature)
- Damage source is torch not creature (ruling): `tier9_cards.rs:470` (blazing_torch_damage_source_is_torch_not_creature)
- Equip ability: `tier9_cards.rs:501` (blazing_torch_equip_ability)
- Torch sacrificed after use: `tier9_cards.rs:438` (verified in deals_damage_to_player test)
- Cross-controller equip interaction (ruling): NOT TESTED

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
- [2009-10-01] If Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
**Status**: ISSUE

### Code issues

1. **Equip ability allows targeting opponents' creatures** (`mtg-engine/src/cards/isd/blazing_torch.rs`, lines 50-57):
   - Oracle text says: `Equip {1} ({1}: Attach to target creature you control.)`
   - Code does: `is_valid_target` checks only `o.zone == Zone::Battlefield` for object targets, without checking `o.controller == caster`. Compare with `runechanters_pike.rs` line 46 which correctly checks `o.controller == caster`. Because `is_valid_target` is shared between the equip ability and the damage ability, and the damage ability legitimately targets any creature/player, the equip case is too permissive.

### Tricky interactions checked
- Block restriction for Vampires/Zombies: PASS (`CreatureFilter::Not(Or(...))` with `EffectScope::Attached`)
- Equip sorcery speed: PASS (sorcery_speed_only: true)
- Equip target "creature you control": FAIL (see issue above)
- Torch sacrifice on use: PASS (calls `crate::destruction::sacrifice`)
- Damage source is torch, not creature: PASS (lines 107-119 find torch ID before sacrificing, use as `damage_source`)
- NonCombatDamageDealt event: PASS (emitted for both creature and player targets)
- LifeChanged event for player damage: PASS (line 147)
- damaged_by tracking: PASS (line 129 uses torch ID as source)

### Test coverage
- Card data (mana cost, types, subtypes): `tier9_cards.rs:384` (blazing_torch_card_data)
- Grants damage ability to equipped creature: `tier9_cards.rs:394` (blazing_torch_grants_damage_ability)
- Deals 2 damage to player: `tier9_cards.rs:412` (blazing_torch_deals_damage_to_player)
- Deals 2 damage to creature: `tier9_cards.rs:444` (blazing_torch_deals_damage_to_creature)
- Damage source is torch not creature (ruling): `tier9_cards.rs:470` (blazing_torch_damage_source_is_torch_not_creature)
- Equip ability: `tier9_cards.rs:501` (blazing_torch_equip_ability)
- Torch sacrificed after use: `tier9_cards.rs:438` (verified in deals_damage_to_player test)
- Equip only targets creature you control: NOT TESTED (bug: not enforced)
- Cross-controller equip interaction (ruling): NOT TESTED

## Audit — 2026-04-01 20:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
- [2009-10-01] If Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
**Status**: PASS

### Code issues
No issues found.

The previous audit flagged equip targeting opponents' creatures via the shared `is_valid_target` method. However, the equip ability at line 77 uses `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)`, which is handled by the engine to filter valid targets before `is_valid_target` is consulted. The test `blazing_torch_equip_only_own_creatures` (tier9_cards.rs:528) verifies this works correctly.

Damage source is correctly attributed to the torch, not the equipped creature: lines 107-119 find the torch ID before sacrificing it, store it as `damage_source`, and use it for `damaged_by.push()` (line 129) and `NonCombatDamageDealt` events (lines 132, 143). This matches the ruling.

### Tricky interactions checked
- Block restriction for Vampires/Zombies: PASS (`CreatureFilter::Not(Or(...))` with `EffectScope::Attached`)
- Equip sorcery speed: PASS (sorcery_speed_only: true)
- Equip targets only creature you control: PASS (`CreatureWithFilter(TargetFilter::YouControl)` at line 77)
- Torch sacrifice on use: PASS (calls `crate::destruction::sacrifice` at line 121)
- Damage source is torch, not creature: PASS (lines 107-119 find torch ID before sacrifice, use as `damage_source`)
- NonCombatDamageDealt event: PASS (emitted for both creature and player targets, lines 132, 143)
- LifeChanged event for player damage: PASS (line 147)
- damaged_by tracking: PASS (line 129 uses torch ID as source)

### Test coverage
- Card data (mana cost, types, subtypes): `tier9_cards.rs:384` (blazing_torch_card_data)
- Grants damage ability to equipped creature: `tier9_cards.rs:394` (blazing_torch_grants_damage_ability)
- Deals 2 damage to player: `tier9_cards.rs:412` (blazing_torch_deals_damage_to_player)
- Deals 2 damage to creature: `tier9_cards.rs:444` (blazing_torch_deals_damage_to_creature)
- Damage source is torch not creature (ruling): `tier9_cards.rs:470` (blazing_torch_damage_source_is_torch_not_creature)
- Equip ability: `tier9_cards.rs:501` (blazing_torch_equip_ability)
- Equip only targets own creatures: `tier9_cards.rs:528` (blazing_torch_equip_only_own_creatures)
- Torch sacrificed after use: `tier9_cards.rs:438` (verified in deals_damage_to_player test)
- Cross-controller equip interaction (ruling): NOT TESTED
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-01 14:49

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies. Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target." Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] If a Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
**Status**: PASS

### Code issues
No issues found.

Card data matches oracle text exactly. Mana cost {1} (Generic(1)), type Artifact with subtype Equipment, oracle text field at line 27 matches Scryfall. Block restriction implemented via `ContinuousEffect::BlockRestriction` with `CreatureFilter::Not(Or([HasSubtype("Vampire"), HasSubtype("Zombie")]))` and `EffectScope::Attached`. Equip {1} at ability_index 0 with `sorcery_speed_only: true` and `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)`. Damage ability at ability_index 1 with `requires_tap: true`, `TargetRequirement::AnyTarget`, sacrifices torch via `crate::destruction::sacrifice`, and correctly attributes damage source to the torch object (lines 107-119 find torch ID before sacrificing, store as `damage_source`).

### Tricky interactions checked
- Block restriction for Vampires/Zombies: PASS (`CreatureFilter::Not(Or(...))` with `EffectScope::Attached`)
- Equip sorcery speed only: PASS (`sorcery_speed_only: true`)
- Equip targets only creature you control: PASS (`CreatureWithFilter(TargetFilter::YouControl)` at line 77)
- Torch sacrifice on use: PASS (calls `crate::destruction::sacrifice` at line 121)
- Damage source is torch, not creature (ruling): PASS (lines 107-119 find torch ID before sacrifice, use as `damage_source` for `damaged_by.push()` at line 130 and `NonCombatDamageDealt` events at lines 132, 143)
- NonCombatDamageDealt event emitted: PASS (for both creature and player targets)
- LifeChanged event for player damage: PASS (lines 146-149)
- damaged_by tracking on creature targets: PASS (line 130)

### Test coverage
- Card data (mana cost, types, subtypes): `tier9_cards.rs:384` (blazing_torch_card_data)
- Grants damage ability to equipped creature: `tier9_cards.rs:394` (blazing_torch_grants_damage_ability)
- Deals 2 damage to player: `tier9_cards.rs:412` (blazing_torch_deals_damage_to_player)
- Deals 2 damage to creature: `tier9_cards.rs:444` (blazing_torch_deals_damage_to_creature)
- Damage source is torch not creature (ruling): `tier9_cards.rs:470` (blazing_torch_damage_source_is_torch_not_creature)
- Equip ability: `tier9_cards.rs:501` (blazing_torch_equip_ability)
- Equip only targets own creatures: `tier9_cards.rs:528` (blazing_torch_equip_only_own_creatures)
- Torch sacrificed after use: `tier9_cards.rs:438` (verified in deals_damage_to_player test)
- Cross-controller equip interaction (ruling): NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/216/blazing-torch?utm_source=api
**Oracle text**:
```
Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
```
**Type line**: Artifact — Equipment
**Mana cost**: {1}
**Rulings**:
- [2009-10-01] The source of the damage is Blazing Torch, not the equipped creature.
- [2009-10-01] If a Blazing Torch controlled by one player somehow winds up equipping a creature a different player controls, the damage ability can't be activated by either player.
**Status**: PASS (with minor concern)

### Card Data — PASS
- **Mana cost**: `Generic(1)` — matches oracle.
- **Types**: `Artifact`, subtype `"Equipment"` — matches oracle.
- **`is_equipment`**: Set to `true` in `on_resolve` — correct.
- **Oracle text field** (line 27): Matches Scryfall verbatim.

### Block Restriction — PASS
Oracle: `Equipped creature can't be blocked by Vampires or Zombies.`
Code (lines 31-37):
```rust
ContinuousEffect::BlockRestriction {
    allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::Or(vec![
        CreatureFilter::HasSubtype("Vampire".into()),
        CreatureFilter::HasSubtype("Zombie".into()),
    ]))),
    scope: EffectScope::Attached,
},
```
Correctly prevents Vampires and Zombies from blocking the equipped creature.

### Equip Ability — PASS
- Cost: `Generic(1)` — matches oracle `Equip {1}`.
- `sorcery_speed_only: true` — correct per reminder text.
- Target: `CreatureWithFilter(TargetFilter::YouControl)` — correct per "target creature you control".
- `on_activate_ability` ability_index 0 attaches torch to target creature — correct.

### Damage Ability — PASS
- Granted to the equipped creature (ability_index 1) when the object has `power` (i.e., is a creature) — correct design for granted abilities.
- `requires_tap: true` — matches `{T}` in cost.
- `TargetRequirement::AnyTarget` — matches "any target".
- Deals exactly 2 damage — correct.
- `NonCombatDamageDealt` event emitted for both creature and player targets — correct.
- `LifeChanged` event emitted for player damage — correct.

### Damage Source Attribution — PASS
Oracle: `Blazing Torch deals 2 damage`
Ruling: "The source of the damage is Blazing Torch, not the equipped creature."
Code (lines 107-119):
```rust
let torch_id = state.objects.values()
    .find(|o| { /* finds the Blazing Torch equipment attached to this creature */ })
    .map(|o| o.id);
let damage_source = torch_id.unwrap_or(object_id);
if let Some(torch) = torch_id {
    crate::destruction::sacrifice(state, torch, registry);
}
```
The code finds the torch's object ID before sacrificing it, then uses that ID as `damage_source` for both `damaged_by.push()` (line 130) and `NonCombatDamageDealt` events (lines 132, 143). This correctly attributes damage to the torch, not the creature. Test `blazing_torch_damage_source_is_torch_not_creature` (tier9_cards.rs:470) verifies this.

### Sacrifice Handling — MINOR CONCERN
```rust
sacrifice_cost: SacrificeCost::None, // Torch sacrifice handled manually in on_activate_ability.
```
The sacrifice is performed manually in `on_activate_ability` rather than declared via `SacrificeCost::SacrificeThis`. In normal play this works correctly because the sacrifice happens immediately upon activation. However:
- If `torch_id` is `None` (torch somehow missing), `damage_source` falls back to `object_id` (the creature), which would be incorrect damage source attribution.
- The cross-controller ruling (can't sacrifice a permanent you don't control) is not automatically enforced by the framework.
These are edge-case robustness concerns, not standard-play bugs.

### Anti-patterns check — PASS
- Uses `NonCombatDamageDealt` (correct — not combat damage).
- No `move_object(id, Zone::Graveyard)` for spells (correct — Equipment is a permanent).
- Sacrifice uses `crate::destruction::sacrifice` (correct helper).

### Test coverage
- `blazing_torch_card_data` (tier9_cards.rs:384) — card type/subtype check
- `blazing_torch_grants_damage_ability` (tier9_cards.rs:394) — creature gets ability when equipped
- `blazing_torch_deals_damage_to_player` (tier9_cards.rs:412) — 2 damage, taps creature, sacrifices torch
- `blazing_torch_deals_damage_to_creature` (tier9_cards.rs:444) — 2 damage to creature
- `blazing_torch_damage_source_is_torch_not_creature` (tier9_cards.rs:470) — verifies torch is source
- `blazing_torch_equip_ability` (tier9_cards.rs:501) — equip attaches torch
- `blazing_torch_equip_only_own_creatures` (tier9_cards.rs:528) — can't equip opponent's creatures

### Missing test coverage
- Block restriction (Vampires/Zombies can't block equipped creature) — NOT TESTED
- Cross-controller equip interaction (ruling) — NOT TESTED
- Edge case: torch missing at activation time (torch_id = None fallback) — NOT TESTED

### Summary
**PASS** — The implementation correctly handles all standard-play scenarios for Blazing Torch. All previous audit issues (wrong damage source, equip targeting) have been resolved. The only remaining concern is edge-case robustness around the manual sacrifice handling when the torch is unexpectedly absent.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies.\nEquipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: PASS

### Code issues
No issues found. Damage source correctly attributed to Blazing Torch (not the equipped creature) per rulings. Block restriction, granted activated ability, and equip ability all correctly implemented.
