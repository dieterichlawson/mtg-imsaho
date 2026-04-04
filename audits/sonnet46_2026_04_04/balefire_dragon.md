## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying\nWhenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
**Type line**: Creature — Dragon
**Status**: ISSUE

### Code issues

- Battlefield guard in `on_combat_damage_to_player` suppresses the triggered effect if Balefire Dragon has left the battlefield at resolution time (`mtg-engine/src/cards/isd/balefire_dragon.rs`, lines 40–42)
  - Oracle text says: `Whenever Balefire Dragon deals combat damage to a player, it deals that much damage to each creature that player controls.`
  - Code does: `if !state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { return; }` — this early-return silently drops the entire effect if the dragon is no longer on the battlefield when the trigger resolves, a condition not present in the oracle text.

  Per MTG CR 112.7a, once a triggered ability is on the stack it exists independently of its source; destroying or removing the source does not affect the ability. The oracle text has no "if Balefire Dragon is still on the battlefield" condition and no intervening-if clause, so the triggered damage must be dealt regardless of the dragon's location at resolution time.

  In the current engine, `resolve_next_trigger` for `CombatDamageToPlayer` (triggers.rs lines 921–924) correctly omits a battlefield check at the engine level, delegating entirely to the card handler. The handler then re-introduces the guard incorrectly. In practice, this is only exploitable when a non-active-player trigger from the same event batch (e.g., a `DamageToPlayerWatch` or `AnyCombatDamageToPlayer` watcher controlled by the defending player) resolves before the dragon's trigger and kills the dragon. The engine's APNAP ordering places NAP triggers on top of the stack, so they resolve first within the same `process_triggers` call. If such a card exists, Balefire Dragon's trigger would be collected (dragon was on battlefield during `collect_triggers`), but the effect would be silently dropped at resolution — wrong per oracle text and CR 112.7a.

### Tricky interactions checked

- **Non-combat damage typing**: The handler emits `NonCombatDamageDealt` (not `CombatDamageDealt`) for the splash damage to creatures — consistent with the official ruling "The damage dealt by Balefire Dragon's triggered ability isn't combat damage." PASS
- **Trigger dispatch filter**: `collect_triggers` for `CombatDamageDealt` to a player checks `obj.zone == Zone::Battlefield && obj.power.is_some()` on the source at collection time; Balefire Dragon is always on the battlefield when combat damage events are collected (no SBA is run between `deal_combat_damage` emitting the event and `process_triggers` being called from the main loop in the normal unblocked-attacker scenario). PASS
- **"each creature that player controls" scope**: Handler iterates `state.objects.values()` filtered by `zone == Battlefield && power.is_some() && controller == damaged_player`. This correctly captures all creatures (including tokens, which store state on the object) controlled by the damaged player, and excludes the dragon itself (which is controlled by the attacking player). PASS
- **Dragon's own creatures excluded**: Filter checks `o.controller == damaged_player`; the dragon's controller is the attacking player, so the dragon's own team is never hit. PASS
- **Amount passed correctly**: The trigger carries `amount` (the actual combat damage dealt to the player) through `PendingTrigger::CombatDamageToPlayer { amount, .. }` and passes it to the handler verbatim. PASS
- **Battlefield check at resolution is never guarded in engine**: `resolve_next_trigger` for `CombatDamageToPlayer` (triggers.rs lines 921–924) does not check `obj.zone == Zone::Battlefield` before calling the card handler, meaning the engine correctly delegates. The bug is the card-level guard, not a missing engine check. ISSUE (see Code Issues)
- **Trigger fires when dragon is unblocked (normal case)**: Dragon has Flying; it is only blocked by Flying/Reach creatures. When unblocked, `deal_damage_to_player` emits `CombatDamageDealt { target: Player(...) }`, `collect_triggers` picks it up (dragon still on battlefield), description is non-empty, trigger is collected and resolves correctly. PASS
- **Trigger does not fire when dragon is blocked without trample**: Dragon has no trample; all damage goes to blockers, not the player, so no `CombatDamageDealt { target: Player }` event is emitted and the trigger never fires. PASS

### Test coverage

- Basic sweep of opponent's creatures when dragon deals combat damage: `mtg-engine/tests/tier6_cards.rs:329` (`balefire_dragon_sweeps_opponent_creatures`) — TESTED
- Own creatures are unaffected: `mtg-engine/tests/tier6_cards.rs:350` (assertion within same test) — TESTED
- Damage to creatures is non-combat damage (event type check): NOT TESTED
- Trigger resolves correctly when Balefire Dragon has left the battlefield before resolution (e.g., killed by a co-resolving NAP trigger): NOT TESTED
- Dragon with trample dealing combat damage to a player while also being killed by first-strike + deathtouch (trigger fires but dragon dies in SBA before `collect_triggers`): NOT TESTED
