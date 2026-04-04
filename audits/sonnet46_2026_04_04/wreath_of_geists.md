## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Continuously updated X (ruling: "constantly updated as creature cards are put into or removed from your graveyard")**: PASS. `dynamic_pt` re-reads `state.objects.values()` on every call; `continuous_pt_mods` invokes it each time `effective_power`/`effective_toughness` is computed, so X reflects live graveyard state.

- **"Your graveyard" = aura controller's graveyard**: PASS. `dynamic_pt` is called with the Wreath's own `ObjectId` (`source.id` from `continuous_pt_mods`), so `state.get_object(object_id)?.controller` retrieves the aura's controller, not the enchanted creature's controller. Correct per MTG rules.

- **Owner vs controller for graveyard counting**: PASS. The filter uses `o.owner == controller`, which is correct: a player's graveyard contains cards they own. This is consistent with `objects_in_zone(Graveyard, player)` which also uses `owner` (state.rs:603).

- **`power.is_some()` as proxy for "creature card"**: PASS. Non-creature cards (enchantments, lands, sorceries, non-creature artifacts) are created with `power: None`; creature cards use `power: Some(n)`. `move_object` does not reset `power` when a card leaves the battlefield, so creature cards correctly retain `power.is_some() == true` in the graveyard. Consistent with Boneyard Wurm and other graveyard-counting cards.

- **Aura attaches correctly at resolution**: PASS. `resolve_aura` (helpers.rs:18) checks `target.zone == Zone::Battlefield` before moving the aura there and setting `attached_to`. If the target has left the battlefield, the aura goes to graveyard via `move_spell_after_resolve`.

- **Aura falls off when enchanted creature leaves the battlefield (SBA 704.5m)**: PASS. `check_state_based_actions_with_registry` (sba.rs:152-193) identifies any aura with `attached_to.is_some()` whose target is no longer on the battlefield and moves it to the graveyard.

- **No double-counting of X**: PASS. `continuous_pt_mods` applies the Wreath's dynamic_pt exactly once (when `source.attached_to == Some(creature_id)`). The enchanted creature's own `behavior.dynamic_pt(self, creature_id)` is called in `effective_power` only for the creature's own card behavior (not the Wreath's). The two calls use different object IDs and different code paths.

- **Multiple Wreaths on one creature**: PASS. `continuous_pt_mods` iterates all battlefield sources and would accumulate each Wreath's +X/+X separately. Each Wreath uses its own controller's graveyard to determine X.

- **Enchanted creature also has its own dynamic P/T (e.g., Boneyard Wurm)**: PASS. Boneyard Wurm's own `dynamic_pt` sets the base P/T; then `continuous_pt_mods` adds the Wreath's +X/+X on top, yielding 2X total. Correct.

- **Wreath itself not counted as creature card in graveyard**: PASS. Wreath of Geists has `power: None` (enchantment), so `o.power.is_some()` is false for it; it is never counted.

- **Aura cannot apply modifier when not on battlefield**: PASS. `continuous_pt_mods` skips any source with `source.zone != Zone::Battlefield` at the top of its loop.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic +X/+X buff (3 creatures in graveyard → 2/2 becomes 5/5): `mtg-engine/tests/tier5_cards.rs` — `wreath_of_geists_dynamic_buff` (line 102)
- Dynamic update when creature card added to graveyard: `mtg-engine/tests/tier5_cards.rs` — `wreath_of_geists_updates_dynamically` (line 124)
- Dynamic update when creature card removed from graveyard: NOT TESTED
- Aura falls off when enchanted creature dies: NOT TESTED (general aura SBA test coverage may exist elsewhere)
- Wreath controlled by different player than enchanted creature (stolen creature scenario): NOT TESTED
- Multiple Wreaths on one creature: NOT TESTED
- Wreath on Boneyard Wurm (double dynamic_pt): NOT TESTED
