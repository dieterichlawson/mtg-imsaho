## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **X=0 activation**: Allowed — engine permits activating with exactly {R}{G} (X=0), granting +0/+0 and trample. Code correctly uses `can_pay(mana_pool, &non_x_cost)` which only requires the non-X portion, and KWR's `on_activate_ability` reads `last_activated_x_value.unwrap_or(0)`. Tested explicitly by `x_equals_0_gives_trample_only`. Pass.
- **X computation**: Engine computes X as `total_mana.saturating_sub(non_x_amount)` after filtering out the X symbol, then drains the pool — so all remaining mana after paying {R}{G} becomes X. Matches oracle text. Pass.
- **Tap cost enforced for both abilities**: Both `mana_abilities` and `activated_abilities` check `!obj.tapped` before returning their respective ability. The engine also independently checks `ab.requires_tap && obj_tapped` in `legal_actions`. After tapping, neither ability is available again. Pass.
- **+X/+0 (not +X/+X)**: `UntilEndOfTurnEffect { power_mod: x, toughness_mod: 0 }` — toughness modifier is correctly zero. Pass.
- **"until end of turn" cleanup**: Both `until_end_of_turn_effects` and `until_end_of_turn_keywords` are cleared in `Step::Cleanup` (engine.rs lines 3021–3022). Pass.
- **Target validity at resolution**: `on_activate_ability` checks `state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)` before applying effects — correctly handles target dying in response. Pass.
- **Target creature (any, not just yours)**: `generate_ability_targets` for `TargetRequirement::Creature` uses `all_objects_in_zone(Zone::Battlefield)` with no controller filter, allowing targeting opponent's creatures. Matches oracle "Target creature." Pass.
- **Colorless mana production**: `mana_abilities` returns `produced: vec![(ManaType::Colorless, 1)]`. Engine adds this to the controller's mana pool in `ActivateManaAbility` handler. Matches `{T}: Add {C}`. Pass.
- **Land type correctness**: `card_types: vec![CardType::Land]`, `supertypes: vec![]`, `subtypes: vec![]`, `cost: None`, `power: None`, `toughness: None`. All correct for a non-basic land with no land subtypes. Pass.
- **Summoning sickness not checked for tap abilities**: Neither `mana_abilities` nor `activated_abilities` check `summoning_sick`. Correct — summoning sickness only restricts creature {T} abilities, not land tap abilities. Pass.
- **Trample granted via `until_end_of_turn_keywords`**: `has_keyword` checks `until_end_of_turn_keywords` (state.rs lines 1036–1040). Correctly reflects the trample grant during the turn. Pass.
- **Legal action generation requires valid target**: `generate_ability_targets` for `TargetRequirement::Creature` respects hexproof via `can_be_targeted`. If no legal target exists, no `ActivateAbility` action is generated. This matches MTG rules (CR 601.2c: can't activate without legal targets). Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Activate with {R}{G} only (X=0): `kessig_wolf_run.rs:20` (`can_activate_with_rg_only`) — TESTED
- Cannot activate without {G}: `kessig_wolf_run.rs:42` (`cannot_activate_without_rg`) — TESTED
- X=3 gives +3/+0: `kessig_wolf_run.rs:63` (`x_equals_3_gives_plus_3`) — TESTED
- X=0 gives +0/+0 and still grants trample: `kessig_wolf_run.rs:99` (`x_equals_0_gives_trample_only`) — TESTED
- All mana spent after activation: `kessig_wolf_run.rs:93` (assertion in `x_equals_3_gives_plus_3`) — TESTED
- {T}: Add {C} mana ability: NOT TESTED (no test taps the land for colorless mana directly)
- Effect expiring at end of turn (until_end_of_turn_effects/keywords cleared): NOT TESTED
- Target dying in response to activation (ability fizzles): NOT TESTED
- Targeting an opponent's creature: NOT TESTED
- Targeting a hexproof creature (should be illegal): NOT TESTED
- Land being tapped after activation (can't reuse same turn): NOT TESTED
