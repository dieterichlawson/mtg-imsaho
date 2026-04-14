---
id: gutter_grime-02
status: new
card: Gutter Grime
card_file: mtg-engine/src/cards/isd/gutter_grime.rs
created: 2026-04-14T21:27:53Z
audit_run_id: 2026-04-14-gutter_grime-audit
audit_model: opus
audit_tokens: 18563
audit_duration: 350
---

## Audit Finding

**Oracle text:**
> Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token...

**Code:**
> triggers.rs:613-619:
> ```rust
> let simultaneously_dead: Vec<ObjectId> = events.iter().filter_map(|e| {
>     if let GameEvent::CreatureDied { object, .. } = e {
>         Some(*object)
>     } else {
>         None
>     }
> }).collect();
> ```
> triggers.rs:620-622:
> ```rust
> let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
>     .filter(|o| o.id != dead_id &&
>         (o.zone == Zone::Battlefield || simultaneously_dead.contains(&o.id)))
> ```

**Description:**
The `simultaneously_dead` list is built exclusively from `CreatureDied` events. Gutter Grime is an enchantment, not a creature — when destroyed, it generates a `LeftBattlefield` event, not `CreatureDied`. If Gutter Grime and nontoken creatures are destroyed simultaneously (e.g., Planar Cleansing destroys all nonland permanents), Gutter Grime won't be in `simultaneously_dead`. Since it's also no longer on the battlefield (`o.zone == Zone::Battlefield` is false), the watcher filter excludes it entirely. The death-watch trigger is never created. Per CR 603.6c + 603.10, Gutter Grime was on the battlefield when the creatures died (last-known information), so its triggered ability should fire for each nontoken creature that died simultaneously.

**Engine path:**
- triggers.rs:613-619 (simultaneously_dead only tracks CreatureDied)
- triggers.rs:620-622 (watcher filter misses non-creature permanents destroyed simultaneously)

**Required check:** 8b

**Affected cards:**
- Gutter Grime
- Any non-creature permanent with `AnyCreatureDies` triggers that could be destroyed simultaneously with creatures (e.g., enchantments or artifacts with death-watch abilities)

