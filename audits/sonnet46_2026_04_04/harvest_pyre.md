## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- Player cannot choose which specific cards to exile; engine arbitrarily picks cards
  - Oracle text says: `"exile X cards from your graveyard"` — in MTG, the casting player freely selects which X cards to exile from their graveyard as the additional cost
  - Code does: `mtg-engine/src/engine.rs` lines 1613–1616: `new_state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id).map(|o| o.id).take(x as usize).collect()` — iterates a `HashMap<ObjectId, GameObject>` (non-deterministic order) and takes the first `x` entries. The player has no input into which specific cards are exiled; the selection is arbitrary and non-reproducible.

### Tricky interactions checked

- **Player chooses X (number of cards)**: Pass — the engine generates one `CastSpell` action per valid X value (0..=graveyard_count) at `engine.rs` lines 592–610, so the player correctly picks the count.
- **Player chooses WHICH cards to exile**: FAIL — see issue above. Oracle text grants the player free choice of which X cards to exile; the engine takes an arbitrary slice of HashMap iteration order.
- **X=0 deals 0 damage**: Pass — resolution code at `harvest_pyre.rs` line 45 gates the damage block on `if count > 0`, so X=0 is a legal no-op.
- **Target must be a creature**: Pass — `target_requirement()` returns `TargetRequirement::Creature`, correctly matching "target creature".
- **Exile is paid at cast time (not on resolution)**: Pass — exile happens in the cast-handling block at `engine.rs` lines 1604–1629, before the spell moves to the stack, consistent with MTG additional-cost timing.
- **Damage uses non-combat event type**: Pass — `harvest_pyre.rs` line 52 pushes `GameEvent::NonCombatDamageDealt`, correct for a spell.
- **Spell moves to graveyard after resolution**: Pass — `harvest_pyre.rs` line 63 calls `state.move_spell_after_resolve(object_id)`.
- **Only own graveyard is eligible**: Pass — the filter at `engine.rs` line 1614 includes `o.owner == player`.
- **Maximum X bounded by graveyard size**: Pass — action generation at line 596 caps X at the graveyard card count, preventing the player from choosing X > available cards.
- **Target creature no longer on battlefield at resolution**: Pass — `harvest_pyre.rs` line 48 checks `obj.zone == Zone::Battlefield` before marking damage; if the creature has left, no damage is dealt (correct per MTG rules for targeted spells when target is gone).
- **X stored correctly on spell object for resolution**: Pass — cast handler stores `count` at `engine.rs` line 1624 as `ObjectId(count as u64)` in `card_state["exile_count"]`; resolution reads it back at `harvest_pyre.rs` lines 40–43.
- **Mana cost {1}{R}**: Pass — `card_data()` encodes `Generic(1)` + `Colored(Red)`.

### Test coverage

- X=4 exiles 4 cards and deals 4 damage: `tests/tier8_cards.rs:562` — TESTED
- Player chooses partial X (X=2 of 4 available): `tests/tier8_cards.rs:596` — TESTED
- X=0 deals no damage: `tests/tier8_cards.rs:634` — TESTED
- Legal actions include every X value 0..=graveyard_count: `tests/tier8_cards.rs:658` — TESTED
- Only own graveyard is exiled (opponent's graveyard untouched): `tests/tier8_cards.rs:687` — TESTED
- Player chooses WHICH specific cards to exile: NOT TESTED (all test graveyard cards are fungible `CardId(9999)` objects, so the arbitrary selection is invisible)
- Target no longer on battlefield at resolution: NOT TESTED
