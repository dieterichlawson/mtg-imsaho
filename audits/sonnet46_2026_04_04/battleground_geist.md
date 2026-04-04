## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Other Spirit creatures you control get +1/+0.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Other" exclusion of self: The `EffectScope::GlobalOther` scope in `effect_applies_to` (state.rs:719-721) checks `creature_id != source_id` before applying the filter, correctly excluding Battleground Geist itself from its own buff. pass
- Spirit subtype check covers tokens: `matches_filter` for `HasSubtype` (state.rs:654-673) first checks `registry.card_data(creature.card_id)` and then falls through to `creature.subtypes.iter().any(...)` for all non-transformed creatures. Spirit tokens created by Doomed Traveler and Midnight Haunting use `create_token_with_subtypes` with `vec!["Spirit".into()]` stored in `obj.subtypes`, so they are correctly recognized. pass
- "you control" filter: `CreatureFilter::You` checks `creature.controller == source_controller` (state.rs:651), correctly requiring the Spirit to be controlled by Battleground Geist's controller. pass
- "+1/+0" values: `power: 1, toughness: 0` in the `ModifyPT` effect matches the oracle exactly. pass
- Continuous re-evaluation (not snapshot): `continuous_pt_mods` is called every time `effective_power`/`effective_toughness` is invoked, and it scans only battlefield objects (skipping sources where `source.zone != Zone::Battlefield`). If Battleground Geist leaves the battlefield, the effect immediately stops applying on the next P/T query. pass
- Opponent's Spirits not buffed: `CreatureFilter::You` restricts to the controller's own creatures, so opponent-controlled Spirits receive no bonus. pass
- Flying keyword declared: `keywords: vec![Keyword::Flying]` is present in card data. pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Battleground Geist gives other Spirits +1/+0 (not itself): `tier5_cards.rs:23` (`battleground_geist_buffs_other_spirits`)
- Opponent's Spirits are not buffed: `tier5_cards.rs:58` (`spirit_lord_doesnt_buff_opponent`)
- Non-Spirit creatures are not buffed: `tier5_cards.rs:23` (asserts non-spirit gets 0 power bonus)
- Spirit tokens (e.g., from Doomed Traveler/Midnight Haunting) are buffed: NOT TESTED
- Effect stops applying when Battleground Geist leaves the battlefield: NOT TESTED
- Flying keyword on Battleground Geist itself: NOT TESTED directly (only the lord effect is tested)
