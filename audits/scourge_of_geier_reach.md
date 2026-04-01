## Audit — 2026-04-01

**Scryfall Oracle text**: Scourge of Geier Reach gets +1/+1 for each creature your opponents control.
**Scryfall type line**: Creature — Elemental
**Mana cost**: {3}{R}{R}
**P/T**: 3/3
**Status**: ISSUE

**Issue: `dynamic_pt` returns absolute values instead of modifier values.**

The `dynamic_pt` method returns `(3 + opponent_creatures, 3 + opponent_creatures)`, which includes the base 3/3 in the returned value. If the engine uses `dynamic_pt` as a replacement for base P/T (overriding power/toughness), this is correct. However, if the engine adds `dynamic_pt` on top of the base P/T (which is already set to 3/3), then the creature would effectively be 6/6 base + bonus, which is wrong. The comment "Base 3/3 + N/N" suggests the implementer intended this to be the total, but the semantics depend on engine convention.

Looking at how other cards use `dynamic_pt` (e.g., Reckless Waif returns `(3, 2)` as the transformed P/T, which replaces the base 1/1), this pattern appears to be "return the total P/T as an override." If the engine treats `dynamic_pt` as an override of base P/T, then this is correct. But for a static ability that says "+1/+1 for each," this should ideally only return the bonus, not the total, to avoid bugs if the engine convention changes.

**Verdict**: Likely functions correctly given the engine's convention for `dynamic_pt` as a total override, but the pattern is fragile and inconsistent with the Oracle text semantics (which describe a modifier, not an absolute).

- Tests: `scourge_of_geier_reach_scales_with_opponent_creatures` and `scourge_of_geier_reach_ignores_own_creatures` in tier12_cards.rs
