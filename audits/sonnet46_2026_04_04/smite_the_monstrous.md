## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target creature with power 4 or greater.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Target legality at resolution does not re-check the power condition (`mtg-engine/src/stack.rs:8-41`)
  - Oracle text says: `Destroy target creature with power 4 or greater.`
  - Code does: `is_target_legal` in `stack.rs` checks only zone for `CreatureWithFilter(_)` targets — it falls through to the wildcard branch `_ => obj.zone == Zone::Battlefield || obj.zone == Zone::Stack` without re-evaluating the power filter. If a targeted creature's effective power drops below 4 in response (e.g., via a debuff), the spell should fizzle per CR 608.2b (target is no longer legal), but instead `any_legal` returns `true` and `on_resolve` is called, destroying the now-ineligible creature.

### Tricky interactions checked

- **Targeting a creature with effective power exactly 4**: `is_valid_target` uses `effective_power` (accounts for buffs, debuffs, counters, until-end-of-turn effects), so this is correctly included. PASS
- **Targeting a creature whose power drops below 4 in response**: `is_target_legal` in `stack.rs` only checks zone (`obj.zone == Zone::Battlefield`) for `CreatureWithFilter` — does not re-check the power condition. Spell should fizzle per CR 608.2b but instead resolves and destroys the creature. FAIL
- **Targeting an indestructible creature with power >= 4**: The creature is targetable (correct — indestructible doesn't grant hexproof). On resolution, `try_destroy` is called which returns `DestroyResult::Indestructible` without moving the creature. PASS
- **Regeneration shield on target creature**: `try_destroy` checks `regeneration_shields > 0` and calls `regenerate` instead of destroying, consuming a shield. Correct per MTG rules. PASS
- **Target leaves battlefield in response**: `resolve_destroy` checks `obj.zone == Zone::Battlefield` before calling `try_destroy`; `is_target_legal` returns `false` (zone != Battlefield), causing fizzle via `any_legal == false`. PASS
- **Spell cleanup (instant goes to graveyard)**: `resolve_destroy` calls `move_spell_after_resolve`, which sends it to graveyard (or exile if cast with flashback). No flashback cost on this card; goes to graveyard correctly. PASS
- **Hexproof creature**: `can_be_targeted` in `engine.rs` filters hexproof creatures from the legal-actions list, so the spell cannot be cast targeting a hexproof creature at all. PASS
- **Power computed with +1/+1 counters**: `effective_power` adds `PlusOnePlusOne` counter count; a 3/3 with two +1/+1 counters is treated as a 5/5. PASS

### Test coverage

- Basic case (5/5 targeted and destroyed, 2/2 not targetable): `mtg-engine/tests/tier2_spells.rs:155` — TESTED
- Power drops below 4 in response (should fizzle): NOT TESTED
- Target with effective power exactly 4 (e.g., 4/4 base): NOT TESTED
- Indestructible creature with power >= 4 (targetable but survives): NOT TESTED
- Regeneration shield on target (survives via regeneration): NOT TESTED
- Target with power boosted to 4+ by +1/+1 counters: NOT TESTED
- Target leaves battlefield in response (fizzle): NOT TESTED
