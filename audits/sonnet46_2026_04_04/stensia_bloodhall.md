## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Colorless source ruling: `card_data()` sets `cost: None`, and `setup_game` derives colors from mana cost symbols — `None` cost yields empty `colors` vec, confirming the object is colorless. Damage events carry `source: object_id`, whose `colors` will be `[]`. PASS
- `{T}` as cost (not part of effect) for the damage ability: `activated_abilities()` returns ability only when `!obj.tapped` and sets `requires_tap: true`; the engine at line 1739 pays the tap cost before calling `on_activate_ability`. PASS
- PlayerOrPlaneswalker target enumeration: engine's `generate_ability_targets` at line 1322 enumerates all alive players and all Battlefield planeswalkers (checking both `obj.card_types` and `registry.card_data`). PASS
- Planeswalker loyalty-counter removal: `on_activate_ability` uses `obj.counters.entry(CounterType::Loyalty).or_insert(0)` and `saturating_sub(2)` on `u32`, so loyalty cannot go below 0; `sba.rs` line 220 checks `== 0` and puts it in graveyard. PASS
- Multiple activations per turn: `once_per_turn: false`, so the ability can be activated multiple times in a turn (correct for a land ability). PASS
- Sorcery speed: `sorcery_speed_only: false`, ability available any time player has priority (correct for land activated abilities). PASS
- Player life reduction: `on_activate_ability` for `Target::Player` reduces `state.get_player_mut(*player_id).life` by 2 and emits `NonCombatDamageDealt` + `LifeChanged`. PASS
- NonCombatDamageDealt (not CombatDamageDealt): the ability's damage is non-combat; code emits `GameEvent::NonCombatDamageDealt` in both branches. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Colorless source (ruling 2011-09-22): NOT TESTED
- `{T}: Add {C}` mana ability: NOT TESTED
- `{3}{B}{R}, {T}` damage ability fires and deals 2 to a player: NOT TESTED
- `{3}{B}{R}, {T}` damage ability fires and reduces planeswalker loyalty: NOT TESTED
- Ability unavailable when land is tapped: NOT TESTED
- Multiple activations in a turn: NOT TESTED
- Target player hexproof protection prevents targeting: NOT TESTED
- Planeswalker reaches 0 loyalty and is put in graveyard by SBA: NOT TESTED
