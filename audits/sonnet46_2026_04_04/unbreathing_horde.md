## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues

- "Enters with counters" replacement effect does not fire when Unbreathing Horde enters the battlefield via reanimation (e.g., Unburial Rites)
  - Oracle text says: `"This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard."`
  - Code does: The counter logic lives entirely in `on_resolve` (`mtg-engine/src/cards/isd/unbreathing_horde.rs:42-81`). When Unburial Rites reanimates the Horde it calls `state.move_object(id, Zone::Battlefield)` directly (`mtg-engine/src/cards/isd/unburial_rites.rs:46` for the single-target path; `mtg-engine/src/engine.rs:2212` for the pending-effect path). `move_object` fires `EnteredBattlefield`, which routes to `on_enter_battlefield` via `triggers.rs:897`. `UnbreathingHorde` does not implement `on_enter_battlefield` — the default empty body runs — so no counters are placed. The Horde enters with 0 counters instead of the correct number.

  Additionally, the ruling "If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with" is also unimplemented for the reanimation path: by the time `on_enter_battlefield` would fire the Horde has already been moved off the graveyard, so even a naive port of the graveyard-count logic to `on_enter_battlefield` would miss the self-count.

### Tricky interactions checked

- **"Each other Zombie you control" excludes self**: PASS — `battlefield_zombies` filter uses `o.id != object_id` and `o.zone == Zone::Battlefield`; when cast normally the Horde is on Zone::Stack so it would be excluded regardless.
- **Zombie tokens counted for battlefield**: PASS — code checks both `registry.card_data(o.card_id)` subtypes AND `o.subtypes` on the object, so tokens (which store subtypes on the object) are counted.
- **Graveyard count before move**: PASS (for normal cast) — `graveyard_zombies` is computed before `state.move_object(object_id, Zone::Battlefield)` is called, so the snapshot is correct.
- **"Count itself from graveyard" ruling**: FAIL (reanimation path not implemented — see Code Issues above).
- **Damage prevented with no counters remaining**: PASS — both `apply_pending_effect` (`engine.rs:2162-2191`) and `apply_prevent_damage_remove_counter` (`combat.rs:276-297`) check `has_prevent` first and skip normal damage application unconditionally; the counter-removal block is inside a nested `if counter_count > 0`, so damage is always prevented even when counter count is 0.
- **Only one counter removed per damage event, regardless of amount**: PASS — both damage paths remove exactly one counter per invocation; `amount` is never inspected for the removal decision.
- **Non-combat damage prevention**: PASS — `apply_pending_effect` handles `PendingEffect::DealDamage` and checks the `PreventDamageRemoveCounter` effect before applying damage, covering non-combat sources such as Lightning Bolt.
- **Combat damage prevention**: PASS — `deal_damage_to_creature` in `combat.rs:445` calls `apply_prevent_damage_remove_counter` and returns early if it returns `true`.
- **`EffectScope::OnSelf` wiring**: PASS — `effect_applies_to` returns `creature_id == source_id` for `OnSelf` (`state.rs:699`), and `has_continuous_effect` scans all battlefield objects, so the effect correctly self-targets.
- **Damage dealt by Horde to others not prevented**: PASS — `apply_prevent_damage_remove_counter` is only called with the *target* as its argument; the attacker's outbound damage goes through the normal path.

### Test coverage

- Normal cast counter count (battlefield + graveyard Zombies): `tests/unbreathing_horde.rs:73` and `tests/tier15_cards.rs:783` — TESTED
- Zombie tokens counted on battlefield: `tests/tier15_cards.rs:783` — TESTED
- Combat damage prevented, counter removed: `tests/unbreathing_horde.rs:21` — TESTED
- Damage dealt by Horde to others not prevented: `tests/unbreathing_horde.rs:47` — TESTED
- Damage prevented when no counters remain (ruling 2011-09-22): NOT TESTED
- Only one counter removed per damage event regardless of amount (ruling 2011-09-22): NOT TESTED
- "Count itself from graveyard" when reanimated (ruling 2011-09-22): NOT TESTED
- Enters with counters when reanimated via Unburial Rites: NOT TESTED
