## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Morbid — When this creature enters, if a creature died this turn, you gain 5 life.
**Type line**: Creature — Elemental
**Status**: ISSUE

### Code issues

- ETB trigger resolution silently skipped when source leaves battlefield before trigger resolves (`mtg-engine/src/triggers.rs:893-899`)
  - Oracle text says: `"you gain 5 life"` (the effect has no dependency on the source remaining on the battlefield)
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(...) }` — if the Scavenger is destroyed in response to its own ETB trigger (e.g., opponent casts a destroy spell while the trigger is on the stack), the condition is false and `on_enter_battlefield` is never called; the life gain is silently skipped. Per MTG rules (CR 603.6), triggered abilities don't require their source to remain on the battlefield, especially for effects such as life gain that don't reference the source object. The engine gives players genuine priority after ETB triggers are placed on the stack (`engine.rs:3207-3210`), so an opponent CAN destroy the Scavenger before the trigger resolves.

### Tricky interactions checked

- `creature_died_this_turn` set by all death paths: `destruction.rs:100` sets flag in the normal destroy pipeline; `sba.rs:96` sets it for zero-toughness deaths; `sba.rs:144` sets it in the fallback lethal-damage path. All paths correctly signal morbid. PASS
- `creature_died_this_turn` reset timing: Reset in `engine.rs:2888` inside the `None` branch of `state.step.next()`, which fires only when the turn advances past Cleanup to the next player's Untap. Flag therefore persists for the entire current turn, which is correct for "a creature died this turn." PASS
- Mana cost, types, P/T: `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green), ManaSymbol::Colored(Color::Green)])`, `vec![CardType::Creature]`, subtypes `vec!["Elemental".into()]`, `power: Some(4), toughness: Some(5)` all match oracle data exactly. PASS
- Life gain amount: `old_life + 5` and `new_life` correctly produce +5 life for the controller. PASS
- Controller lookup: `state.get_object(object_id).map(|o| o.controller)` correctly reads the controller; object remains in `state.objects` (with zone = Graveyard) even after death, so `o.controller` stays valid. The battlefield check at line 894-898 is therefore not needed for correctness of the handler body itself — which underscores that the check is wrong. PASS (handler would work without the battlefield guard)
- Intervening-if clause not checked at collection time: In `collect_triggers` (`triggers.rs:344-392`), the ETB trigger is collected for any registered card with no check on whether the morbid condition is currently true. Per CR 603.4, the trigger should only go on the stack if "a creature died this turn" is already true. The code defers the check to resolution. In practice this causes a spurious ETB trigger entry on the stack when morbid is not satisfied, giving players an extra priority window around a trigger that will do nothing. Game outcome is unchanged (no life gain when morbid is false), but the extra stack entry is a rules violation. Flagged as secondary; the primary issue is the battlefield guard.
- Source leaves battlefield between trigger-on-stack and resolution: As explained in Code Issues, the battlefield guard at `triggers.rs:894-898` incorrectly prevents the life gain. FAIL
- Life gain event emitted correctly: `GameEvent::LifeChanged { player: controller, old: old_life, new_life }` matches the `LifeChanged` struct definition in `events.rs:30`. PASS
- `keywords: vec![]` — Morbid is an ability word, not a keyword ability. Correct to omit from keywords list. PASS

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Morbid path (creature_died_this_turn = true → gain 5 life): NOT TESTED
- No-morbid path (creature_died_this_turn = false → no life gain): NOT TESTED
- Source destroyed in response to ETB trigger (life gain must still resolve): NOT TESTED
- creature_died_this_turn set by SBA lethal damage: `mtg-engine/tests/card_mechanics.rs:28` (`morbid_flag_set_on_creature_death`) — TESTED (engine-level, not card-specific)
- creature_died_this_turn reset at turn boundary: `mtg-engine/tests/card_mechanics.rs:43` (`morbid_flag_resets_on_new_turn`) — TESTED (engine-level, not card-specific)
- Intervening-if spurious trigger when morbid not satisfied: NOT TESTED
