## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

- Auto-selection of which two creatures to tap — `mtg-engine/src/cards/isd/skirsdag_high_priest.rs` lines 68–73
  - Oracle text says: `{T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying.`
  - Code does: `let to_tap: Vec<ObjectId> = state.objects_in_zone(Zone::Battlefield, controller).iter().filter(|o| o.id != object_id && o.power.is_some() && !o.tapped).take(2).map(|o| o.id).collect();`
  - "Tap two untapped creatures you control" is a cost the player pays, meaning the player must choose which two untapped creatures to tap. When the controller has more than two untapped creatures (besides the High Priest), `.take(2)` silently picks the first two found in an arbitrary HashMap iteration order, denying the player any choice. The engine's action generation for this ability (engine.rs lines 399–405) emits a single `ActivateAbility` action with no target list, so there is no mechanism for the player to specify which two creatures they wish to tap.

### Tricky interactions checked

- **Morbid condition gating (`creature_died_this_turn`)**: `activated_abilities()` returns an empty vec when `state.creature_died_this_turn` is false, so the ability simply does not appear as a legal action when morbid is inactive. The flag is set by `destruction::destroy()` (destruction.rs:100) and by `sba.rs` (lines 96, 144) whenever a creature moves from battlefield to graveyard; it is cleared at turn change in `advance_step()` (engine.rs:2888). All paths that kill creatures set the flag. **Pass.**
- **High Priest's own {T} cost / summoning sickness**: `activated_abilities()` checks `obj.summoning_sick` before listing the ability (line 38), correctly preventing activation while the priest is summoning sick. The engine's `requires_tap: true` field causes the engine to tap the priest when the ability resolves (engine.rs:1739–1741). **Pass.**
- **Summoning-sick creatures as tap-cost fodder**: The ruling states the two tapped creatures need not have been under the controller's control since the start of their most recent turn. The `on_activate_ability()` filter `!o.tapped` (without `!o.summoning_sick`) correctly allows summoning-sick creatures to be tapped as the cost. **Pass.**
- **Token correctness (5/5 black Demon with flying)**: `create_token_with_subtypes("Demon", controller, 5, 5, vec![Color::Black], vec![CardType::Creature], vec![Keyword::Flying], vec!["Demon".into()])` matches the oracle exactly. **Pass.**
- **High Priest excluded from the two tapped creatures**: Both the availability check and `on_activate_ability()` use `o.id != object_id` to exclude the High Priest from the set of creatures available to tap as cost. **Pass.**
- **No once-per-turn or sorcery-speed restriction**: The card sets `once_per_turn: false` and `sorcery_speed_only: false`. The oracle imposes no such restrictions, so the ability can be used multiple times per turn and at instant speed. **Pass.**
- **Parallel Lives interaction**: Token creation goes through `create_token_with_subtypes()`, which checks for Parallel Lives and doubles accordingly. **Pass.**
- **Player choice for which two creatures to tap**: Auto-selection removes choice when the controller has more than two untapped creatures. **Fail** (see Code issues above).

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic activation with morbid + exactly 2 other creatures: `mtg-engine/tests/tier10_cards.rs:157` (`skirsdag_high_priest_creates_demon_with_morbid`) — TESTED, but uses exactly 2 helpers so auto-selection bug is not observable.
- Cannot activate without morbid: `mtg-engine/tests/tier10_cards.rs:184` (`skirsdag_high_priest_no_morbid`) — TESTED.
- Needs 2 other untapped creatures: `mtg-engine/tests/tier10_cards.rs:202` (`skirsdag_high_priest_needs_two_creatures`) — TESTED.
- Auto-selection vs. player choice of which two creatures to tap: NOT TESTED.
- Summoning-sick creatures can be used as tap-cost fodder (ruling 2020-08-07): NOT TESTED.
- Demon token has correct color (black), subtype (Demon), P/T (5/5), and keyword (Flying): `mtg-engine/tests/tier10_cards.rs:174–179` — TESTED (power, toughness, flying; black color not explicitly asserted).
- Ability unavailable while High Priest is summoning sick: NOT TESTED.
- Ability unavailable while High Priest is tapped: NOT TESTED.
- `creature_died_this_turn` flag cleared at turn change (morbid expires): NOT TESTED.
- Multiple activations per turn: NOT TESTED.
