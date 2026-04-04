## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Until end of turn" cleanup**: Both `until_end_of_turn_effects` (the +2/+2) and `until_end_of_turn_keywords` (lifelink) are cleared in the cleanup step at `engine.rs:3021-3022`. Pass.
- **Target validity on resolution**: `on_resolve` checks `o.zone == Zone::Battlefield` before applying effects, so if the target leaves the battlefield in response, the spell fizzles gracefully. Pass.
- **"Target creature" — any creature, not just own**: `TargetRequirement::Creature` in `engine.rs:866` iterates all creatures on the battlefield (no owner restriction), matching the oracle's unrestricted "target creature". Pass.
- **Lifelink applied to combat damage to creatures**: `deal_damage_to_creature` in `combat.rs:474` calls `state.has_keyword(source, Keyword::Lifelink, registry)`, which checks `until_end_of_turn_keywords` at `state.rs:1036-1040`. Pass.
- **Lifelink applied to combat damage to players**: `deal_damage_to_player` in `combat.rs:539` performs the same check. Pass.
- **`has_keyword` correctly reads temporary grants**: `state.has_keyword` checks `until_end_of_turn_keywords` as step 3 in `state.rs:1035-1040`, so the granted lifelink is visible to all damage paths. Pass.
- **+2/+2 applied to effective_power and effective_toughness**: Both `effective_power` (`state.rs:886-890`) and `effective_toughness` (`state.rs:928-931`) iterate `until_end_of_turn_effects` and add `power_mod`/`toughness_mod` for the matching target. Pass.
- **move_spell_after_resolve called**: Instant moves to graveyard after resolution at `moment_of_heroism.rs:51`. Pass.
- **No "may" — effect is mandatory**: Oracle has no "you may"; code unconditionally pushes both effects when target is valid. Correct. Pass.

### Test coverage
- Lifelink granted after casting: `keywords.rs:504-505` — TESTED
- +2/+2 applied after casting: `keywords.rs:506-507` — TESTED
- Both effects expire at cleanup: `keywords.rs:515-519` — TESTED
- Lifelink causes life gain during combat damage: NOT TESTED (no combat simulation test for Moment of Heroism specifically)
- Spell fizzles if target leaves battlefield in response: NOT TESTED
