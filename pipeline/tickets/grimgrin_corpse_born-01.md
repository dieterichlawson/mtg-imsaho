---
id: grimgrin_corpse_born-01
status: new
card: Grimgrin, Corpse-Born
card_file: mtg-engine/src/cards/isd/grimgrin_corpse_born.rs
created: 2026-04-14T20:55:26Z
audit_run_id: 2026-04-14-grimgrin_corpse_born-audit
audit_model: opus
audit_tokens: 15240
audit_duration: 306
---

## Audit Finding

**Oracle text:**
> Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.

**Code:**
> `triggers.rs:1332-1337`: `PendingTrigger::AttacksTrigger` resolution dispatches directly to `behavior.on_attacks(state, object_id, &chosen_targets, registry)` without any target legality re-check. Compare with spell resolution at `stack.rs:87-108` which explicitly re-checks target legality per CR 608.2b before calling `on_resolve`.

> `engine.rs:3727-3737`: `PendingEffect::DestroyThenCounter` unconditionally calls `try_destroy` then `add_counters` — the counter is always added regardless of whether the target is still legal.

**Description:**
The engine checks target legality at resolution time for spells (CR 608.2b fizzle check in `stack.rs:87-108`) but has no equivalent check for triggered abilities. When Grimgrin's attack trigger resolves, `resolve_next_trigger` passes `chosen_targets` directly to `on_attacks` without verifying the target is still a legal creature on the battlefield controlled by the defending player. If the target creature is killed, bounced, exiled, or gains hexproof/protection between targeting (when the trigger goes on the stack) and resolution, the `DestroyThenCounter` handler still runs: `try_destroy` silently fails on the absent creature, but `add_counters` on Grimgrin executes unconditionally. Per CR 608.2b and the Scryfall ruling, the entire ability should fail to resolve with no effects when all targets are illegal.

**Engine path:**
- `triggers.rs:1332-1337` — `AttacksTrigger` resolution, no target re-check
- `stack.rs:87-108` — spell resolution HAS the fizzle check (for comparison)
- `engine.rs:3727-3737` — `DestroyThenCounter` handler, unconditional counter add
- `grimgrin_corpse_born.rs:117-129` — `on_attacks` passes target to effect without validation

**Required check:** 8b (trigger dispatch/resolution) + 8j (ruling coverage)

**Affected cards:**
- Grimgrin, Corpse-Born (counter added when it shouldn't be)
- All cards with targeted triggered abilities (engine-wide: no trigger resolution fizzle check)

