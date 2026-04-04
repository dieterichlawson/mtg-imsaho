## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"each creature you control" vs "target creature"**: Code uses `state.objects_in_zone(Zone::Battlefield, controller)` (no targeting, all creatures) with no `target_requirement`, which is correct for "each". Pass.
- **Controller scope (not "each player")**: `objects_in_zone(Zone::Battlefield, controller)` filters strictly to the activating player's creatures; opponent's creatures are excluded. Pass.
- **Creature detection via `o.power.is_some()`**: The land itself has `power: None` and is thus excluded from the counter loop. Consistent with engine-wide convention for identifying creatures. Pass.
- **Mana ability produces {C}**: `ManaType::Colorless` is the engine's representation of colorless mana ({C}). `produced: vec![(ManaType::Colorless, 1)]` is correct. Pass.
- **Mana cost `{2}{G}{W}`**: Implemented as `Generic(2), Colored(Green), Colored(White)`. Matches oracle exactly. Pass.
- **Tap cost for both abilities**: Both the mana ability and the activated ability gate on `!obj.tapped`, and `requires_tap: true` causes the engine to set `tapped = true` when the activated ability fires. Pass.
- **`once_per_turn: false`**: Oracle imposes no once-per-turn restriction; code correctly omits it. Pass.
- **`sorcery_speed_only: false`**: Oracle has no "activate only as a sorcery" clause; code correctly allows activation at any time. Pass.
- **Behavior lookup after tap cost is paid**: After the engine sets `tapped = true`, re-calling `activated_abilities()` on the card returns `vec![]`. The engine's `behavior_card_id` logic falls through to the `unwrap_or(card_id)` fallback, so `on_activate_ability` is still called on the correct card. No bug. Pass.
- **`ability_index` ignored in `on_activate_ability`**: The parameter `_ability_index` is unused. Since Gavony Township has exactly one activated ability (index 1) — the mana ability at index 0 is handled via the separate `ActivateManaAbility` path and never reaches `on_activate_ability` — ignoring the index is safe. Pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card is type Land with no casting cost: `mtg-engine/tests/tier10_cards.rs:223` (`gavony_township_card_data`)
- +1/+1 counter placed on each creature the controller controls: `mtg-engine/tests/tier10_cards.rs:233` (`gavony_township_counters_all_creatures`)
- Opponent's creatures do not receive counters: `mtg-engine/tests/tier10_cards.rs:233` (`gavony_township_counters_all_creatures`, checks `enemy` creature)
- Ability available when untapped with correct mana: `mtg-engine/tests/tier10_cards.rs:233`
- {T}: Add {C} mana ability: NOT TESTED
- Activation at instant speed (non-sorcery): NOT TESTED
- Multiple activations in the same turn: NOT TESTED
- Activation with zero creatures on battlefield: NOT TESTED
