## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Creatures your opponents control get -4/-0 until end of turn.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Snapshot at resolution (Ruling 1)**: The code collects `opponent_creature_ids` as a `Vec<ObjectId>` at the moment `on_resolve` fires, then pushes one `UntilEndOfTurnEffect` per collected ID. Creatures that enter the battlefield or come under an opponent's control *after* resolution are never added to the list — pass.
- **Effect persists through control change (Ruling 2)**: `UntilEndOfTurnEffect` stores the creature's `ObjectId`. Control changes in the engine update `obj.controller` in place without incrementing `zone_change_count` or reassigning the `ObjectId` (only `move_object` changes zones/IDs). Therefore if the caster gains control of a creature that was already tagged, the `target == id` match in `effective_power`/`effective_toughness` still holds — pass.
- **"until end of turn" cleanup**: `state.until_end_of_turn_effects.clear()` is called at `Step::Cleanup` (engine.rs line 3021), correctly expiring the -4/-0 — pass.
- **"your opponents" in multiplayer**: The filter `obj.controller != controller` captures all non-casting-player controllers, not just a single opponent — pass.
- **Creature detection heuristic**: `obj.power.is_some()` is the standard engine-wide convention for detecting creatures and is consistent with the rest of the engine — pass.
- **Non-targeting spell**: The card has no `Target` requirements; `_targets` is unused. Correct, because the oracle text has no "target" — pass.
- **`move_spell_after_resolve`**: Used correctly at end of `on_resolve`; the instant moves to graveyard (or exile if cast with flashback) — pass.
- **Mana cost and types**: `{2}{U}` maps to `Generic(2) + Colored(Blue)`; `card_types: [Instant]`; no supertypes/subtypes — all match oracle — pass.
- **Power/toughness modification values**: `power_mod: -4, toughness_mod: 0` matches "-4/-0" — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Snapshot at resolution (only creatures present at resolution are affected): NOT TESTED
- Effect persists through control change after resolution: NOT TESTED
- "until end of turn" cleanup at end-of-turn cleanup step: NOT TESTED
- Basic resolution (opponents' creatures get -4/-0): NOT TESTED
