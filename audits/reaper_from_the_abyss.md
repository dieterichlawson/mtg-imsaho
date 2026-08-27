## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/112/reaper-from-the-abyss
**Oracle text**:
```
Flying
Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
```
**Type line**: `Creature — Demon` — {3}{B}{B}{B}, 6/6, Keywords: Flying, Morbid
**Ruling [2011-09-22]**: The morbid ability is mandatory. If you control the only non-Demon creature when the ability triggers, you must choose it as the target.

**Status**: ISSUE (2 found, both fixed)

### Code issues

1. **Morbid enforced as target legality (CR 603.3c) rather than as an intervening-if (CR 603.4)** — `reaper_from_the_abyss.rs`, `is_valid_target`.
   - Oracle text says: `Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.`
   - Code did: `fn is_valid_target(...) { if !state.creature_died_this_turn { return false; } ... }`, with the comment *"if no creature died this turn, no creature is a legal target, so the trigger is removed from the stack per CR 603.3c."*
   - Those are different rules. 603.4 says an ability with an unmet intervening-if **never triggers**; 603.3c says an ability with no legal targets **is put on the stack and then removed**. The board state came out the same, which is why the existing test passed — but the difference is player-visible in the log. Probed directly: with no creature dead, the engine emitted
     `Trigger removed: no legal targets (Reaper from the Abyss's end step trigger (if morbid, destroy target non-Demon creature))`
     for an ability that by 603.4 never triggered. "Each end step" means this fired twice a turn, every turn the Reaper survived without a death.
   - Fixed: morbid moved to `should_trigger` (the hook 16 other ISD cards use); `is_valid_target` now tests only properties of the target. The resolution-time re-check in `on_end_step` stays, per CR 603.4's second check.
   - Reproduction: extended `intervening_if.rs::reaper_from_the_abyss_end_step_trigger_respects_its_morbid_clause` to assert no "Trigger removed" line — failed before the fix with the exact message above.

2. **Hand-rolled half of the `is_creature` accessor** — same function.
   - Code did: `if obj.zone != Zone::Battlefield || obj.power.is_none() { return false; }`
   - `state.is_creature(id, registry)` is the accessor for this and is documented as `has_card_type(Creature) || obj.power.is_some()` — card types *plus* the object-level P/T sentinel that tokens and `*/*` creatures carry. Inlining one half is the `obj.power`-instead-of-registry anti-pattern from step 9.
   - Fixed: calls `state.is_creature`. Worth recording that my first attempt swapped in `has_card_type` alone and broke the positive arm of the existing test — the P/T sentinel is load-bearing, not redundant.

### Tricky interactions checked
- **Ruling (mandatory, may have to target your own creature)**: PASS. The filter is `NotSubtypes(["Demon"])` with no controller restriction, so the controller's own non-Demons are offered.
- **Trigger outliving its source (CR 113.7a)**: PASS. `on_end_step` deliberately ignores `self_id`; covered by `trigger_independence.rs:74`.
- **Target chosen at trigger time (CR 603.3d)**: PASS. `target_requirement` is declared on the `TriggeredAbilityDef`, so the engine locks the target as the trigger goes on the stack; `on_end_step` reads `chosen_targets` rather than re-picking.
- **Destroy vs indestructible**: PASS. Uses `PendingEffect::DestroyCreature` through `apply_pending_effect` → the `try_destroy` pipeline, so indestructible and regeneration apply. Oracle says "destroy", not "sacrifice".
- **"each end step", not "your end step"**: PASS. No `step_trigger_scope` override, so it defaults to `TriggerScope::Each`.
- **Morbid is an ability word, not a keyword**: correct to omit from `keywords` (only `Flying` is declared). Scryfall lists ability words in its `keywords` array; they confer nothing.
- **Demon targeting itself**: PASS. Reaper is a Demon, so the `NotSubtypes` filter excludes it — matching "target non-Demon creature" without needing a self-exclusion clause.

### Test coverage
- Morbid gates the trigger, both arms: `intervening_if.rs:236`
- No phantom "Trigger removed" log when morbid is unmet: `intervening_if.rs:251` — **added by this audit**
- Trigger resolves after the Reaper dies: `trigger_independence.rs:74`
- Ruling (must target your own creature when it is the only non-Demon): NOT TESTED
- Destroy respects indestructible: NOT TESTED for this card (the pipeline is covered generally in `state_based_actions.rs`)
- Non-Demon filter excludes another Demon: NOT TESTED
