## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

- Player cannot choose which creature to sacrifice when multiple are available (`mtg-engine/src/engine.rs` lines 1750–1759)
  - Oracle text says: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`
  - Code does: `SacrificeCost::SacrificeCreature => { // For now, auto-sacrifice the first eligible creature. // TODO: Present choice to player when there are multiple options. let creature = new_state.objects_in_zone(Zone::Battlefield, player).iter().find(|o| o.power.is_some()).map(|o| o.id); if let Some(cid) = creature { crate::destruction::sacrifice(&mut new_state, cid, registry); } }` — `objects_in_zone` iterates a `HashMap` in non-deterministic order and picks the first match. When the activating player controls more than one creature, they are never presented with a choice; the engine silently auto-selects. "Sacrifice a creature" as an activation cost requires the player to choose which creature to sacrifice.

- `oracle_text` field wording does not match Scryfall oracle text (`mtg-engine/src/cards/isd/disciple_of_griselbrand.rs` line 25)
  - Oracle text says: `{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.`
  - Code does: `oracle_text: "{1}, Sacrifice a creature: You gain life equal to that creature's toughness.".into()` — "that creature's" instead of "the sacrificed creature's".

### Tricky interactions checked

- **Last-known toughness on the battlefield (ruling 2011-09-22)**: pass. `destruction::sacrifice` calls `destroy` which captures `effective_toughness` (including continuous P/T mods, counters, until-end-of-turn effects) BEFORE calling `move_object`, satisfying the "as it last existed on the battlefield" requirement.
- **Sacrificing the Disciple itself**: pass. `SacrificeCost::SacrificeCreature` includes the Disciple among valid targets (correct per oracle text — "a creature" is unrestricted). After self-sacrifice, `state.get_object(object_id)` in `on_activate_ability` still returns the object (it stays in `state.objects` with `zone = Graveyard`), so the controller lookup and life gain proceed correctly.
- **Toughness ≤ 0 edge case**: pass. The `CreatureDied` event stores `last_known_toughness` as an `i32`; `on_activate_ability` does `.max(0)` before the `if toughness > 0` guard. Gaining 0 life is a no-op, consistent with MTG rules.
- **Mandatory life gain (no "you may")**: pass. Oracle text has no "you may" — the life gain is mandatory upon ability resolution. The code unconditionally gains life (subject to the `> 0` guard above); no optional choice is presented, which is correct.
- **Correct `CreatureDied` event matched**: pass (with caveats). `on_activate_ability` scans `state.events.iter().rev()` for the most-recent `CreatureDied` event. Because the engine pays the sacrifice cost immediately before invoking `on_activate_ability` and nothing else pushes a `CreatureDied` event in between (no SBA pass is interleaved), this always retrieves the toughness of the just-sacrificed creature. The matching is implicit rather than explicit (no `object` field check), so it is fragile if the call order ever changes, but is correct as written today.
- **Sorcery-speed restriction**: pass. The ability has `sorcery_speed_only: false`, which is correct — the oracle text has no timing restriction.
- **Mana cost**: pass. Cost is `{1}` generic (no colored requirement), implemented as `ManaCost::new(vec![ManaSymbol::Generic(1)])`.
- **Card types / subtypes / P/T**: pass. `CardType::Creature`, subtypes `["Human", "Cleric"]`, power 1, toughness 1 all match the type line and P/T.

### Test coverage

- Basic life gain from sacrifice: `mtg-engine/tests/tier8_cards.rs:138` (`disciple_of_griselbrand_gains_life`) — covered, but test only asserts `gained > 0`; it does not verify the specific amount gained.
- Player choice of sacrifice target when multiple creatures available: NOT TESTED (the test comment explicitly acknowledges auto-sacrifice happens).
- Last-known toughness equals battlefield toughness, not graveyard toughness (ruling 2011-09-22): NOT TESTED — no test uses a creature whose toughness is modified by a continuous effect before sacrifice.
- Sacrificing the Disciple itself: NOT TESTED.
- Life gain when sacrificed creature has toughness 0 (or negative after effects): NOT TESTED.
- Correct creature's toughness used when multiple `CreatureDied` events are in the log: NOT TESTED.
