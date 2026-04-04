## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +2/+2 and has flying.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"Enchant creature" targeting enforced correctly**: `TargetRequirement::Creature` is returned by `target_requirement()`, which causes the engine to restrict cast targets to battlefield creatures. The valid-targets path in `engine.rs:1050` filters `o.power.is_some()` and checks `can_be_targeted`. Pass.
- **+2/+2 continuous effect applied to enchanted creature only**: `ContinuousEffect::ModifyPT { power: 2, toughness: 2, scope: EffectScope::Attached }` is read by `state.rs::continuous_pt_mods`. The scope resolves via `effect_applies_to` at `state.rs:700–704`, which reads `source.attached_to` dynamically on every call, correctly restricting the bonus to the single attached creature. Pass.
- **Flying granted continuously**: `ContinuousEffect::GrantKeyword { keyword: Keyword::Flying, scope: EffectScope::Attached }` is checked by `state.rs::has_keyword` via `has_continuous_effect`. Evaluated dynamically on every call; no snapshot. Pass.
- **"As long as" (not present, but checking anyway)**: The card has no conditional qualifier. The effects apply unconditionally while the aura is on the battlefield and attached. The engine's zone filter (`source.zone != Zone::Battlefield` → skip) and `EffectScope::Attached` both re-evaluate on every call. Pass.
- **Effects stop when aura leaves battlefield**: `continuous_pt_mods` and `has_continuous_effect` skip any source not in `Zone::Battlefield`. Once the aura is moved to graveyard, it no longer contributes. Pass.
- **Aura falls off when enchanted creature dies**: SBA (`sba.rs:149–193`) implements CR 704.5m. When the attached creature moves to `Zone::Graveyard`, the filter `t.zone != Zone::Battlefield` becomes true and the aura is moved to graveyard in the same SBA pass. Token creatures that are later removed from the objects HashMap are also caught via `unwrap_or(true)`. Pass.
- **Target must still be on battlefield when spell resolves**: `resolve_aura` checks `state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before attaching. If the target has left, `move_spell_after_resolve` sends the aura to the graveyard. Pass.
- **No triggered abilities**: Card declares `triggered_abilities: vec![]`, matching the oracle text which has no "whenever" or "at the beginning of" clauses. Pass.
- **oracle_text field**: The code stores `"Enchanted creature gets +2/+2 and has flying."`. The full Scryfall oracle text is `"Enchant creature\nEnchanted creature gets +2/+2 and has flying."` — the `"Enchant creature"` line is absent. This field is used only for display/UI (`view.rs:223`) and never for game mechanics; the targeting restriction is encoded in `TargetRequirement::Creature`. Several other auras in the codebase (Holy Strength, Pacifism, Gruesome Deformity) have the same omission. No functional impact. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- +2/+2 granted to enchanted creature: `innistrad_cards.rs:272–287` (`spectral_flight_buffs_and_grants_flying`), `bug_fixes.rs:476–493` (`spectral_flight_gives_plus_two_and_flying`), `keywords.rs:471–489` (`aura_grants_keyword`)
- Flying granted to enchanted creature: `innistrad_cards.rs:285`, `bug_fixes.rs:491`, `keywords.rs:483`
- Aura falls off when enchanted creature dies: `enchantments.rs:35–52` (`aura_falls_off_when_creature_dies`) — tested via Holy Strength (same code path); NOT tested for Spectral Flight specifically
- Effects stop when aura leaves battlefield: NOT TESTED directly for Spectral Flight
- Aura target no longer valid on resolution (fizzle to graveyard): NOT TESTED for Spectral Flight
- Continuous re-evaluation (not snapshot at ETB): NOT TESTED explicitly
