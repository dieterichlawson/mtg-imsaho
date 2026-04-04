## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures you control gain protection from non-Human creatures until end of turn.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Protection's "T" (targeting) aspect not enforced by engine — non-Human creature activated abilities can still target protected creatures
  - Oracle text says: `"gain protection from non-Human creatures until end of turn"` (protection means DEBT: Damage, Enchanting, Blocking, Targeting prevented from non-Human creature sources)
  - Code does: `can_be_targeted` in `mtg-engine/src/engine.rs` line 758 only checks hexproof: `if state.has_keyword(target_id, Keyword::Hexproof, registry)`. It does not consult `until_end_of_turn_protection` at all. `valid_targets_for_req` (line 1053) calls `can_be_targeted` for all creature targets, so a non-Human creature's activated ability (e.g., Olivia Voldaren's `{T}: deal 1 damage to target creature`) can still legally target a creature that has protection from non-Human creatures.

- Protection's "D" (damage) aspect not enforced for non-combat damage from non-Human creature sources
  - Oracle text says: `"gain protection from non-Human creatures until end of turn"` (protection prevents all damage from non-Human creature sources, not just combat damage)
  - Code does: `apply_pending_effect` in `mtg-engine/src/engine.rs` lines 2154–2191 for `PendingEffect::DealDamage` does not check `until_end_of_turn_protection`. It only checks `PreventDamageRemoveCounter`: `let has_prevent = state.has_continuous_effect(*id, &|e| { match e { crate::types::ContinuousEffect::PreventDamageRemoveCounter { scope } => Some(scope), _ => None, } }, registry);`. Non-combat damage from a non-Human creature source would not be prevented.

### Tricky interactions checked

- Creature snapshot at resolution (ruling: "Only creatures you control when Spare from Evil resolves will be affected"): pass — code collects `creature_ids` at resolve time and applies protection to each; creatures entering later do not receive protection
- Human creature correctly excluded from protection filter: pass — `CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into())))` correctly returns false for Humans, so `matches_filter` returns false and protection does not apply to Human blockers/attackers
- Token Humans correctly excluded (subtypes on object, not registry): pass — `matches_filter` in `state.rs` line 666–672 checks both `registry.card_data(creature.card_id)` AND `creature.subtypes.iter()`, so token Humans (whose subtype is stored on the object) are correctly treated as Humans and thus not non-Human creatures
- Transformed werewolves (e.g., Gatstaf Shepherd back face "Werewolf" — no Human subtype): pass — `matches_filter` for transformed creatures uses `back_face_data()` subtypes; a transformed werewolf has no Human subtype on its back face, so it counts as a non-Human creature and protection applies
- Blocking prevention (B aspect of protection): pass — `can_block_attacker` in `mtg-engine/src/combat.rs` line 696–701 calls `has_protection_from_creature`, which checks `until_end_of_turn_protection` at lines 407–416
- Combat damage prevention (D aspect — combat path): pass — `deal_damage_to_creature` in `mtg-engine/src/combat.rs` line 440 calls `has_protection_from_creature` before applying damage
- Non-combat damage from non-Human creature sources prevented: fail — `apply_pending_effect` in `engine.rs` lines 2154–2191 does not check `until_end_of_turn_protection` (see Code Issues above)
- Targeting by non-Human creature sources prevented: fail — `can_be_targeted` in `engine.rs` line 758 only checks hexproof (see Code Issues above)
- Cleanup at end of turn: pass — `state.until_end_of_turn_protection.clear()` at `engine.rs` line 3024 (Step::Cleanup branch)
- Spell uses `move_spell_after_resolve`: pass — `state.move_spell_after_resolve(object_id)` called at `spare_from_evil.rs` line 62, correctly handles flashback exile vs. normal graveyard

### Test coverage

- Creature snapshot at resolution (only battlefield creatures at resolution time): NOT TESTED
- Human creature excluded from protection: `mtg-engine/tests/tier12_cards.rs:272` — tested via `human_opp` can still block
- Token Human excluded: NOT TESTED
- Non-Human creature (Zombie) can't block protected creature: `mtg-engine/tests/tier12_cards.rs:268`
- Transformed werewolf treated as non-Human: NOT TESTED
- Combat damage from non-Human prevented: NOT TESTED
- Non-combat damage from non-Human creature source not prevented: NOT TESTED
- Targeting by non-Human creature ability not prevented: NOT TESTED
- Cleanup at end of turn (protection expires): NOT TESTED
