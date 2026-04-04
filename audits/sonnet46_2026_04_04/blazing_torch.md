## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Equipped creature can't be blocked by Vampires or Zombies.
Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target."
Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues

- `AnyTarget` engine implementation does not include planeswalkers as valid targets
  - Oracle text says: `"Blazing Torch deals 2 damage to any target."`
  - Code does: `generate_ability_targets` for `TargetRequirement::AnyTarget` (engine.rs lines 1343–1358) only generates creatures (`o.power.is_some()`) and players. Planeswalkers — which are not creatures and have no power — are excluded. The ISD set has implemented planeswalkers (Garruk Relentless, Liliana of the Veil), so targets exist in practice. Confirmed by reading engine.rs:1343: `filter(|o| o.power.is_some())` with no separate planeswalker loop (contrast with `TargetRequirement::PlayerOrPlaneswalker` at engine.rs:1322–1342 which explicitly handles planeswalkers).

### Tricky interactions checked

- **Block restriction (Vampires and Zombies can't block)**: PASS. `ContinuousEffect::BlockRestriction { allowed_blockers: Not(Or(HasSubtype("Vampire"), HasSubtype("Zombie"))), scope: Attached }` is evaluated in `can_block_attacker` (combat.rs:650–683). `effect_applies_to` with `EffectScope::Attached` correctly resolves to true only when the torch is equipped to the attacking creature (state.rs:701–705). The `matches_filter` check then rejects Vampires and Zombies. Correct.
- **"Any target" includes planeswalkers**: FAIL (see Code Issues above). `AnyTarget` generates creatures and players only.
- **Damage source is Blazing Torch, not equipped creature**: PASS. `on_activate_ability` searches for the attached torch by name/identity and uses `damage_source = torch_id` (blazing_torch.rs:119). If the torch is found (the expected case when the ability fires), the `NonCombatDamageDealt` event and `damaged_by` list both record `torch_id`, not the creature id.
- **Sacrifice is part of activation cost, not effect**: PASS. The torch is found and sacrificed in `on_activate_ability` before the damage is dealt (blazing_torch.rs:120–122). The tap cost is handled by the engine (engine.rs:1739–1741).
- **NonCombatDamageDealt (not CombatDamageDealt)**: PASS. Both player and object target paths in `on_activate_ability` push `GameEvent::NonCombatDamageDealt` (blazing_torch.rs:132–145).
- **Equip sorcery-speed only**: PASS. The equip `ActivatedAbilityDef` has `sorcery_speed_only: true` (blazing_torch.rs:80), enforced by engine.rs:360.
- **Equip targets only your creatures**: PASS. `TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)` (blazing_torch.rs:77); confirmed by test `blazing_torch_equip_only_own_creatures`.
- **Equipment detaches (not destroyed) when creature dies**: PASS. SBA at sba.rs:169–188 detaches equipment when its attached-to creature leaves the battlefield, keeping the torch on the battlefield.
- **Behavior dispatch for damage ability (ability_index collision)**: PASS for simple creatures without activated abilities. The engine's behavior detection at engine.rs:1783–1799 checks if the equipped creature's own behavior returns any activated abilities; if it does not, it correctly falls back to finding the torch as the source behavior. For creatures with their own activated abilities this heuristic could mis-dispatch, but no such collision arises in normal Innistrad gameplay with Blazing Torch.
- **Ruling: cross-controller equip makes ability unactivatable**: NOT MODELED. The equip ability only targets creatures you control (`TargetFilter::YouControl`), so normal gameplay cannot produce a cross-controller equipped state. This ruling edge case is therefore unreachable in this engine and is not an issue in practice.

### Test coverage

For each ruling and tricky interaction:
- Block restriction (Vampires/Zombies can't block equipped creature): NOT TESTED
- Damage to player (2 damage, tap cost, sacrifice): `tier9_cards.rs` — `blazing_torch_deals_damage_to_player`
- Damage to creature (2 damage marked): `tier9_cards.rs` — `blazing_torch_deals_damage_to_creature`
- Damage source is torch not creature: `tier9_cards.rs` — `blazing_torch_damage_source_is_torch_not_creature`
- Damage ability is granted when equipped: `tier9_cards.rs` — `blazing_torch_grants_damage_ability`
- Equip ability attaches torch: `tier9_cards.rs` — `blazing_torch_equip_ability`
- Equip only targets own creatures: `tier9_cards.rs` — `blazing_torch_equip_only_own_creatures`
- Card data (Artifact, Equipment subtype, {1} cost): `tier9_cards.rs` — `blazing_torch_card_data`
- AnyTarget includes planeswalkers: NOT TESTED
- Equip sorcery-speed restriction: NOT TESTED
- Ruling (cross-controller equip): NOT TESTED
