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
| 1 | Intervening-if triggers (CR 603.4) | 12 | **done** |
| — | Confirm-and-close already-fixed tickets | 19 | **done** |
| 3 | Enters-tapped replacement effects | ~8 | not started |
| 4 | Card code reading empty object-level fields | 7 | **done** |
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

### Cluster 1 — intervening-if trigger conditions (CR 603.4)

**Root cause.** An intervening-if clause is checked when the ability *would*
trigger, not only when it resolves; a false condition means the ability never
goes on the stack. `collect_triggers` had no notion of this — it queued a
trigger for every permanent whose active face declared one and left the
condition to the resolution handler. The board ended up correct (which is why
no existing test caught it), but a phantom stack entry appeared and opened a
priority window the rules say shouldn't exist.

**Fix.** New `CardBehavior::should_trigger`, consulted at dispatch time on both
the step (upkeep / end step / end of combat) and ETB paths in
`collect_triggers`. It follows the existing `should_trigger_on_spell_cast` /
`_on_blocks` / `_on_becomes_blocked` gates, which solve the same problem for
CR 603.2 event conditions. Defaults to `true`. It takes the `TriggerKind`, so a
face with both a conditional and an unconditional trigger (Howlpack Alpha's
upkeep transform vs. its end-step Wolf token) gates only the conditional one.

Two `helpers` functions carry the shared conditions instead of duplicating
them across 15 cards: `werewolf_should_trigger` delegates to the card's own
`should_transform` — which is what stops dispatch-time and resolution-time
checks from ever disagreeing — and `morbid_should_trigger` reads
`creature_died_this_turn`.

**Deliberately excluded** (they read as intervening-if but aren't): Cloistered
Youth ("you may transform"), Screeching Bat ("you may pay {2}{B}{B}"), Delver
of Secrets (looks at the top card regardless, then transforms on what it finds).

**Test.** `mtg-engine/tests/intervening_if.rs`, 8 tests. They assert on the
stack after dispatch, not the resolved board, since the board was already
right. Every condition is covered in both directions across the whole family,
so the gate can't pass by suppressing everything. 5 of the 8 verified red
against the pre-fix engine.

**Bug found along the way.** The family test caught `instigator_gang-01`
independently: Wildblood Pack declared only its `AnyCreatureAttacks` ability,
and `face_trigger_description` reads the *visible* face's list — so the back
face had no upkeep trigger and could never turn back into a Human, no matter
how many spells were cast. Fixed in the same pass.

**Tickets closed:** daybreak_ranger-02, gatstaf_shepherd-01,
grizzled_outcasts-01, hanweir_watchkeep-01, instigator_gang-01, kruin_outlaw-01,
reckless_waif-01, tormented_pariah-01, ulvenwald_mystics-01,
village_ironsmith-01, villagers_of_estwald-01, woodland_sleuth-01.

**Correction to `SESSION_HANDOFF.md`'s grouping.** It listed
`mentor_of_the_meek-01` and `wooden_stake-01` in this cluster; reading the
tickets, neither is intervening-if. `mentor_of_the_meek-01` wants the entering
creature's power *snapshotted* into `PendingTrigger::EnterWatch` at trigger
time (CR 603.2 event condition). `wooden_stake-01` is a "blocks a Vampire"
event condition, which the existing `should_trigger_on_blocks` hook already
covers. Both are tracked as one-offs instead.

### Confirm-and-close pass — 19 tickets

`SESSION_HANDOFF.md` listed ~20 tickets believed fixed by the previous
session's engine work. Each was verified by reading the ticket's own claim and
then the current code at the path it names — not by trusting the list. All 19
are genuinely fixed, and each has regression-test coverage already in
`mtg-engine/tests/`. Per-ticket verification notes are in each ticket's
`fix_note`. The recurring shapes were:

- **Inline damage → `damage::deal_damage`** (blazing_torch-01,
  heretic_s_punishment-01, skirsdag_cultist-01, daybreak_ranger-03 via
  `combat::fight`). The central pipeline applies protection, the planeswalker
  loyalty branch, `PreventDamageRemoveCounter` and lifelink; each inline copy
  had missed some subset.
- **Characteristics layer** (ghost_quarter-01, maw_of_the_mire-01,
  into_the_maw_of_hell-01, blazing_torch-02, heretic_s_punishment-02,
  skirsdag_cultist-02, olivia_voldaren-03) — filters now go through
  `state.has_card_type` / `has_subtype` rather than reading the empty
  object-level vectors.
- **Dispatch-time trigger conditions** (charmbreaker_devils-01 via
  `should_trigger_on_spell_cast`, wooden_stake-01 via
  `should_trigger_on_blocks`, thraben_sentry-01 via
  `triggered_abilities_of`) — the same family as the new `should_trigger`.
- One-offs: fiend_hunter-01 (target locking), divine_reckoning-02 (engine-owned
  spell cleanup), runechanter_s_pike-01 (equipment `dynamic_pt` leaking into
  its own P/T), evil_twin-03 (`entering_copy_source` never cleared),
  blazing_torch-03 (attached-ability controller guard).

The one correction in the handoff held up: **unbreathing_horde-01 is still
open** and is not covered by the damage-pipeline work.

### Cluster 4 — characteristics reads in card and condition code

Three distinct root causes, not one:

1. **Empty vector read as "has no types".** Garruk Relentless' -3 filtered on
   `o.card_types.contains(Creature)`, false for every non-token creature, so
   the +X/+X and trample hit nothing. Curse of the Pierced Heart scanned the
   same field for Planeswalker, making "or a planeswalker that player
   controls" dead code. Both now use `state.is_creature` / `has_card_type`.
2. **Non-empty vector read as the whole truth.**
   `EffectCondition::AttachedHasSubtype` treated `obj.subtypes` as
   authoritative once anything was in it, so a Human that Olivia Voldaren had
   turned into a Vampire stopped counting as a Human — Silver-Inlaid Dagger's
   +1/+0 and Butcher's Cleaver's lifelink both vanished, and
   `AttachedLacksSubtype` inverted the wrong answer on top. Now goes through
   `has_subtype`, matching what `CreatureFilter::HasSubtype` already did.
3. **Front-face-only registry read.** `registry.card_data` always returns the
   front face, so Hamlet Captain buffed transformed werewolves as if still
   Human (CR 712.8d). Full Moon's Rise had the same read and was switched over
   with it (harmless today — werewolf front faces are Werewolves too).

Plus the mirror of (2) on the way out: CR 400.7 runtime grants — Olivia's
"Vampire", Grimoire of the Dead's "Zombie" and black — were surviving a zone
change, so a creature reanimated by Grimoire stayed a black Zombie in the
graveyard. `move_object` now clears `subtypes` and `colors` on leaving the
battlefield, **before** the CR 712.8a front-face revert (order matters: the
clear would otherwise wipe what that revert writes). Tokens are exempt —
their object-level fields *are* their printed characteristics.

**Deliberately not done:** clearing `card_types` on zone change. Neither
ticket asks for it, nothing in the set grants a card type at runtime (only the
copy-effect and token paths write that field), and clearing it broke test
fixtures that use `card_types` as a stand-in for a real card. That is a
separate behavioural change with its own blast radius.

**Test.** `mtg-engine/tests/characteristics_card_sweep.rs`, 6 tests; 5
verified red pre-fix, the 6th guards the token exemption.

**Two existing tests were re-based on real levers.** Both simulated "stops
being a Human" by overwriting `obj.subtypes`, which models nothing the engine
does — outside transform, subtypes are only ever added to. They now transform
a real DFC (Cloistered Youth → Unholy Fiend; Villagers of Estwald → Howlpack
of Estwald), which is the lever the equipment test file's own opening note
already described. Their assertions are unchanged in intent.

**Tickets closed:** garruk_relentless-01, curse_of_the_pierced_heart-01,
hamlet_captain-01, butcher_s_cleaver-02, silver_inlaid_dagger-01,
olivia_voldaren-01, grimoire_of_the_dead-01.

## Next up

1. Clusters 3 → 5 → 6, then the one-off tail.
2. `should_trigger` is now the hook for any future intervening-if card — check
   for one whenever a ticket says "the condition is only evaluated at
   resolution".

**Backlog count: 50 fixed / 66 open** (was 2 / 114 at the start of this pass).
