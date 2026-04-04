## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Mikaeus enters with X +1/+1 counters on it.
{T}: Put a +1/+1 counter on Mikaeus.
{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
**Type line**: Legendary Creature — Human Cleric
**Status**: ISSUE

### Code issues

- Summoning sickness not enforced for {T} activated abilities
  - Oracle text says: `{T}: Put a +1/+1 counter on Mikaeus.` and `{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.`
  - Code does: `mtg-engine/src/engine.rs` line 356: `if ab.requires_tap && obj_tapped { continue; }` — this only skips if the permanent is already tapped; it does not check `summoning_sick`. Neither does `activated_abilities` in `mtg-engine/src/cards/isd/mikaeus_the_lunarch.rs` lines 53–55 (only checks `zone == Battlefield`). Per CR 302.6, a creature's activated ability with the tap symbol in its cost cannot be activated unless the creature has been under its controller's control since the start of their most recent turn. Mikaeus can activate both {T} abilities on the turn it enters, which is incorrect.

### Tricky interactions checked

- ETB with X +1/+1 counters (replacement effect timing): The counters are placed in `on_resolve` after `move_object` emits the `EnteredBattlefield` event into `state.events`, but triggers are not processed until after `on_resolve` returns. So ETB-watching triggers that fire from `EnteredBattlefield` will see Mikaeus with its counters already placed. This is correct behavior.
- X=0 entry: Code guards with `if x > 0`, so entering with X=0 skips counter placement. Mikaeus enters with 0 counters, consistent with oracle text. Pass.
- "each other creature you control" filter in ability 1: Code at line 112–116 filters `o.id != object_id` (excludes Mikaeus) and `o.power.is_some()` (creatures only) from `objects_in_zone(Battlefield, controller)`. Correctly targets only the controller's other creatures. Pass.
- Ability 1 offered only when counter is present: `activated_abilities` at line 73–85 gates ability_index 1 on `has_counter = state.get_counter_count(object_id, CounterType::PlusOnePlusOne) > 0`. If no counter, ability 1 is not offered. Pass.
- Counter removal as part of effect vs. cost: Per MTG rules the counter removal is a cost (before the colon), but in this engine it is executed inside `on_activate_ability` (after the tap cost is paid). Because activated abilities resolve immediately in this engine (no stack for non-mana abilities), this ordering difference has no practical consequence. Pass.
- Legendary flag: `is_legendary = true` is explicitly set in `on_resolve` after `move_object`. The SBA code (`sba.rs` line 290) reads `obj.is_legendary` to enforce the legend rule. Pattern is consistent with other legendary cards (Geist of Saint Traft, Grimgrin). Pass.
- x_value preservation across zone change: `move_object` does not clear `x_value` when entering the battlefield (only clears state when leaving the battlefield). `on_resolve` reads `o.x_value` after `move_object` returns. Pass.
- Summoning sickness and {T} abilities: Fail — see Code issues above.

### Test coverage

- ETB with X +1/+1 counters: `mtg-engine/tests/tier15_cards.rs:1451` (`mikaeus_enters_with_x_counters`) — TESTED
- Ability 1 distributes counters to other creatures and removes counter from Mikaeus: `mtg-engine/tests/tier15_cards.rs:1469` (`mikaeus_distributes_counters`) — TESTED
- Ability 0 ({T}: add counter to Mikaeus): NOT TESTED
- Summoning sickness blocks {T} abilities on entry turn: NOT TESTED
- Ability 1 not available when no counters: NOT TESTED
- Legend rule (duplicate Mikaeus SBA): NOT TESTED
- X=0 entry (enters with no counters): NOT TESTED
