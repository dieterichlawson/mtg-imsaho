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
