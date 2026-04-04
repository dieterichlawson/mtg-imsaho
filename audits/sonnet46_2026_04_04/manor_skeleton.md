## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Haste
{1}{B}: Regenerate this creature.
**Type line**: Creature — Skeleton
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Haste allows attacking while summoning sick: `combat.rs:577` checks `!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry)`, and `has_keyword` in `state.rs:1005-1015` reads Haste from Manor Skeleton's card data via the registry — pass.
- Regeneration shield consumed on lethal damage SBA: `sba.rs:101-113` calls `crate::destruction::try_destroy`, which in `destruction.rs:40-43` checks `regeneration_shields > 0` and calls `regenerate()` (taps, clears damage, removes from combat, decrements shield) — pass.
- Regeneration shield cleared at cleanup step: `engine.rs` cleanup step iterates all battlefield objects and sets `obj.regeneration_shields = 0` — pass.
- Activated ability only available on battlefield: `manor_skeleton.rs:33` checks `o.zone == Zone::Battlefield` before returning the ability def — pass.
- Activated ability can be used while summoning sick: the `legal_actions` loop in `engine.rs` does not gate non-tap abilities on `summoning_sick`; only `requires_tap && obj_tapped` is checked. Since `requires_tap: false` for regenerate, a freshly cast (summoning sick) Manor Skeleton can activate regenerate — correct per MTG rules.
- Regeneration removes creature from combat: `destruction.rs:82` calls `remove_from_combat(state, id)`, which removes the creature from `combat.attackers` and all `blocker_assignments` — pass.
- Regeneration shield not consumed by 0-toughness death: `sba.rs:71-74` sends 0-toughness creatures directly to graveyard via `move_object`, bypassing `try_destroy` entirely — pass.
- End-of-turn shield clearance only affects battlefield objects: `engine.rs` cleanup iterates `state.objects.values_mut()` and filters `obj.zone == Zone::Battlefield` before clearing shields — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Correct stats (power, toughness, subtype, Haste keyword): `activated_abilities.rs:22-31` (manor_skeleton_has_correct_stats)
- Activating regenerate ability adds a shield: `activated_abilities.rs:33-51` (manor_skeleton_regenerate_ability)
- Regeneration saves from lethal damage (shield consumed, damage cleared): `activated_abilities.rs:53-76` (manor_skeleton_regeneration_saves_from_lethal)
- Haste overrides summoning sickness (allowing attack): `keywords.rs:135-160` (haste_overrides_summoning_sickness — uses a manually constructed creature, not Manor Skeleton specifically) — NOT TESTED for Manor Skeleton directly
- Regeneration shield cleared at cleanup step: NOT TESTED for Manor Skeleton specifically
- Regeneration removes creature from combat: NOT TESTED
- Activated ability unavailable when not on battlefield: NOT TESTED
