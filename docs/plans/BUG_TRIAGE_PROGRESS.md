# Bug triage progress log

Running record of work against the `new_pipeline/tickets/` backlog. Append to
this file as clusters land — it is the resumable state for any agent or device.

**Companion doc:** `docs/plans/SESSION_HANDOFF.md` holds the original cluster
analysis and rationale. This file holds *what has actually been done since*.

**Branch:** `claude/bug-triage-fixes-7516rt`

## How to run the tests here

`cargo test --workspace` reports 6 failures in `mtg-player`'s
`llm_conversation` suite in any sandbox without an API key —
`mtg-player/src/llm.rs:487` panics on a missing `ANTHROPIC_API_KEY`. These are
environment noise, not regressions. Run:

```
ANTHROPIC_API_KEY=dummy cargo test --workspace
```

With that set, all 16 llm_conversation tests pass. Per CLAUDE.md: check the
exit code and read the full output for both `FAILED` and `could not compile`;
never filter through `grep FAILED`.

## Cluster status

| # | Cluster | Tickets | Status |
|---|---------|--------:|--------|
| 2 | Protection-from-source in ability targeting | ~10 | **done** |
| 1 | Intervening-if triggers (CR 603.4) | ~14 | not started |
| — | Confirm-and-close already-fixed tickets | ~20 | not started |
| 3 | Enters-tapped replacement effects | ~8 | not started |
| 4 | Card code reading empty object-level fields | ~7 | not started |
| 5 | Control-on-entry ordering | ~4 | not started |
| 6 | Targeted trigger declared untargeted | ~4 | not started |
| 7 | Card-specific one-offs | ~45 | not started |

## Done

### Cluster 2 — protection from source during activated-ability targeting

**Root cause.** `generate_ability_targets` filtered candidates through a
`can_be_targeted` wrapper that hard-coded `source_id: None`. The
protection-from-source check inside `can_be_targeted_by` only runs when
`source_id` is `Some`, so with `None` it was silently skipped: every creature
with protection from the ability's source still showed up as a legal target
(CR 702.16b). The spell path (`valid_targets_for_req`) threaded the source
through correctly; the ability path never did.

**Fix** (`mtg-engine/src/engine.rs`):

- All five `can_be_targeted` call sites in `generate_ability_targets`
  (`Creature`, `CreatureWithFilter`, `PlayerOrPlaneswalker`, `AnyTarget`,
  `PermanentWithFilter`) now call
  `can_be_targeted_by(..., Some(source_id), ...)`.
- Found the same bug on the **spell** path while fixing it: the
  `PlayerOrPlaneswalker` branch of `valid_targets_for_req` (was engine.rs:1681)
  passed `None` rather than `Some(spell_id)`, so a planeswalker with protection
  from the spell was targetable. Fixed too.
- Deleted the `can_be_targeted` wrapper entirely, so the `None` default cannot
  be reintroduced by a future call site.

**Test.** `mtg-engine/tests/ability_target_protection.rs` — two tests covering
both target-requirement shapes (Avacynian Priest / `CreatureWithFilter`,
Elder of Laurels / `Creature`). Each asserts the unprotected creature *is*
offered as a target first, so the test cannot pass vacuously. Verified red
against the pre-fix engine and green after.

**Tickets closed:** avacynian_priest-01, cobbled_wings-02, daybreak_ranger-01,
elder_of_laurels-01, kessig_wolf_run-01, sharpened_pitchfork-01,
stensia_bloodhall-01, inquisitor_s_flail-01, skirsdag_cultist-03, evil_twin-06.

## Next up

1. Cluster 1, intervening-if (CR 603.4) — biggest single win at ~14 tickets.
   Condition must be checked when the trigger *would fire*, not at resolution;
   dispatch site is `mtg-engine/src/triggers.rs:843`.
2. Confirm-and-close the ~20 tickets `SESSION_HANDOFF.md` believes are already
   fixed. (That doc explicitly corrects one: **unbreathing_horde-01 is still
   open** — it is about counting Zombie *cards* excluding tokens on entry, not
   the fight-damage prevention the damage pipeline fixed.)
3. Then clusters 3 → 4 → 5 → 6, and the one-off tail last.
