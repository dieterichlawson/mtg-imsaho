## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample
When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Type line**: Creature — Insect
**Status**: ISSUE

### Code issues

- **`on_dies` unconditionally exiles regardless of current zone, violating ruling 2 about simultaneous Monstrosity deaths**
  - `mtg-engine/src/cards/isd/moldgraf_monstrosity.rs` lines 48-51
  - Oracle text says (per Scryfall ruling 2011-09-22): `"If two Moldgraf Monstrosities die simultaneously, the first ability to resolve could return the other Moldgraf Monstrosity to the battlefield. If it does, the second Moldgraf Monstrosity's ability won't exile it but it will return two more creature cards to the battlefield."`
  - Code does: `state.move_object(object_id, Zone::Exile);` unconditionally, with no check that the object is currently in the Graveyard. When two Monstrosities die at once, the first trigger can return the second to the Battlefield (since it is also in the Graveyard). When the second trigger then resolves, `state.get_object(object_id)` returns the now-Battlefield Monstrosity, and `state.move_object(object_id, Zone::Exile)` exiles it FROM THE BATTLEFIELD (emitting a `LeftBattlefield` event, resetting battlefield state). The ruling says the exile "won't" happen; the code instead performs an illegal Battlefield→Exile move.

### Tricky interactions checked

- **Exile-first ordering** (`exile it, then return two creature cards`): PASS — code calls `state.move_object(object_id, Zone::Exile)` before collecting the graveyard list, so the Monstrosity is correctly excluded from the creature pool.
- **Ruling 1: ability fires even if exile fails** (`If Moldgraf Monstrosity's ability can't exile it...`): PASS — trigger resolution does not gate the creature returns on the exile succeeding. `move_object` is a zone-agnostic setter; if the Monstrosity was already exiled by another effect, the call silently re-sets it to Exile with no harmful side-effects, and the graveyard search proceeds.
- **Ruling 2: simultaneous Monstrosity deaths — second exile "won't" happen**: FAIL — as described above, `on_dies` has no zone guard before calling `move_object(object_id, Zone::Exile)`. If the first trigger returned the second Monstrosity to the Battlefield, the second trigger incorrectly exiles it from the Battlefield.
- **`SelfDies` trigger dispatch**: PASS — `triggers.rs` lines 401-415 create a `PendingTrigger::SelfDies` for every `GameEvent::CreatureDied`, and Moldgraf Monstrosity is a registered card with a `TriggerKind::SelfDies` entry. `resolve_next_trigger` calls `behavior.on_dies` unconditionally (correct for a dies trigger — no battlefield-presence guard needed).
- **Trigger fires without source needing to be in any specific zone at resolution**: PASS — `resolve_next_trigger` for `SelfDies` has no zone check (unlike `EnteredBattlefield` which guards `zone == Battlefield`). The ability correctly resolves regardless of where the Monstrosity ends up before the trigger resolves.
- **Random creature selection** (`at random`): PASS — code shuffles the candidate list with `rand::seq::SliceRandom::shuffle` then takes the first two, producing a uniform random selection.
- **Up-to-two semantics when fewer creatures exist**: PASS — `take(2)` naturally returns fewer if the graveyard has 0 or 1 creature cards.
- **Creature-card detection** (`creature cards`): PASS — filter `o.power.is_some()` identifies creatures. Tokens cannot exist in the graveyard, so no false positives there. The filter is applied after the zone query, excluding non-creatures.
- **Returned creatures come under correct controller** (`to the battlefield`): PASS for the common case; minor edge-case concern when Monstrosity is controlled by a non-owner (see note below). In normal gameplay owner == controller, so `obj.controller = controller` (which holds `o.owner`) correctly sets ownership of returned creatures.
- **`"your graveyard"` when controlled by opponent (edge case)**: The variable named `controller` on line 43 is assigned `o.owner`, not `o.controller`. In the edge case where a Mind Control effect gives an opponent control of the Monstrosity, `objects_in_zone(Zone::Graveyard, controller)` would search the original owner's graveyard rather than the ability's controller's graveyard. This is technically wrong per oracle ("your" = ability controller), but requires a control-change effect to manifest and is not the focus of this audit.
- **Trample keyword declared**: PASS — `keywords: vec![Keyword::Trample]` in `card_data()`.
- **Mana cost** `{4}{G}{G}{G}`: PASS — `ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Green), ManaSymbol::Colored(Green), ManaSymbol::Colored(Green)])`.
- **P/T 8/8**: PASS — `power: Some(8), toughness: Some(8)`.
- **Subtype Insect**: PASS — `subtypes: vec!["Insect".into()]`.

### Test coverage

- **Basic death trigger (exile self + return 2 creatures)**: `tier15_cards.rs:1786` — TESTED. Note: the test calls `behavior.on_dies()` directly while the Monstrosity is still on the Battlefield (not in the Graveyard), which differs from the actual in-game state when the trigger resolves. The test still exercises the core logic correctly.
- **Ruling 1 — exile fails gracefully, creatures still returned**: NOT TESTED
- **Ruling 2 — two simultaneous Monstrosity deaths, second exile "won't" happen**: NOT TESTED
- **Fewer than 2 creature cards in graveyard**: NOT TESTED
- **Trigger fires through full destroy→collect→resolve pipeline**: NOT TESTED (test uses direct `on_dies` call)
- **Random selection is uniform (statistical)**: NOT TESTED
