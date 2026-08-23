# Session handoff — engine correctness pass + ticket backlog

**Last updated:** 2026-08-22. Written so this work can be resumed from any
device (e.g. Claude Code on the web / iOS). Everything below is on `master`.

## How to resume on another device

Claude Code on the web (claude.com/code) and the iOS app run in a **cloud**
sandbox against this GitHub repo — they don't share a local terminal's
conversation history. To continue:

1. Open the Claude iOS app (or claude.com/code in a mobile browser) → connect
   the `dieterichlawson/mtg-imsaho` repo.
2. Start a session and say: *"Read docs/plans/SESSION_HANDOFF.md and continue
   with the next steps."*

All code, tests, and this brief are on `master`, so a fresh cloud session has
full context. (Note: the local `~/.claude/.../memory/` notes do **not** sync to
the cloud — this doc is the portable version.)

## Repo state

- **Single branch: `master`** (all prior feature/fix/pipeline branches merged
  or deleted). Working tree clean.
- `cargo test --workspace` → **all green (113 suites), zero warnings.**
- Two pipelines exist: `pipeline/` (retired) and `new_pipeline/` (current).
  Tickets live in `new_pipeline/tickets/` (116 files: 114 `new`, 2 `fixed`).

## What was fixed this session (all on `master`, each with regression tests)

Engine foundations: characteristics layer on `GameState` (object → active face
→ registry); unified damage pipeline (`damage::deal_damage`); engine-owned
spell cleanup (CR 608.2m); trigger dispatch — event-time conditions +
ETB target locking (CR 603.2/603.3d); two combat damage steps with a priority
window (CR 510.4/510.5).

Self-audit fixes: Evil Twin SBA guard (CR 614.1d); `change_control` summoning
sickness + Olivia Vampire-subtype recognition (CR 302.6/508.1a); until-EOT
effects end on zone change (CR 400.7); combat declaration validation
(CR 509.1a); CopyCreature copies the legendary supertype (CR 707.2);
`create_token_copy` generic-token fallback (cackling).

## Ticket reconciliation: ~20 fixed, ~94 open

**Fixed (confirm & close):** ghost_quarter-01, maw_of_the_mire-01,
into_the_maw_of_hell-01, blazing_torch-01/02/03, heretic_s_punishment-01/02,
skirsdag_cultist-01/02, daybreak_ranger-03, divine_reckoning-02,
charmbreaker_devils-01, fiend_hunter-01, thraben_sentry-01, wooden_stake-01,
runechanter_s_pike-01, olivia_voldaren-03, evil_twin-03, cackling_counterpart-01.
(Run the `new_pipeline` test-phase on these to auto-confirm and close.)

**Open — by cluster, highest leverage first:**

1. **Intervening-if triggers (CR 603.4) — ~14 tickets.** No engine support for
   checking an `if` condition when a trigger would fire. Almost all werewolf
   day/night transforms: gatstaf_shepherd, reckless_waif, tormented_pariah,
   ulvenwald_mystics, villagers_of_estwald, grizzled_outcasts, kruin_outlaw,
   hanweir_watchkeep, village_ironsmith, woodland_sleuth, instigator_gang-01,
   mentor_of_the_meek-01. **One engine feature closes the whole cluster.**
2. **Protection-from-source not enforced during ability targeting — ~8.**
   `generate_ability_targets` calls `can_be_targeted` with `source_id: None`.
   avacynian_priest-01, cobbled_wings-02, daybreak_ranger-01, elder_of_laurels-01,
   kessig_wolf_run-01, sharpened_pitchfork-01, stensia_bloodhall-01,
   inquisitor_s_flail-01, skirsdag_cultist-03, evil_twin-06. Small targeted fix.
3. **"Enters tapped" replacement effects (CR 614.1c/d) — ~8.** Dual lands etc.:
   clifftop_retreat, hinterland_harbor, isolated_chapel, sulfur_falls,
   woodland_cemetery. Needs an entering-replacement mechanism.
4. **Card code reading empty object-level fields — ~7.** The characteristics
   trap, but in card files: garruk_relentless-01 (card_types),
   curse_of_the_pierced_heart-01 (planeswalker), hamlet_captain-01,
   butcher_s_cleaver-02, silver_inlaid_dagger-01, olivia_voldaren-01,
   grimoire_of_the_dead-01. Switch these cards to the `GameState` characteristics
   accessors (`has_subtype`/`has_card_type`/`is_creature`).
5. **Control-on-entry ordering — ~4.** `move_object` then set controller, so ETB
   fires under the wrong controller: moldgraf_monstrosity-02,
   grimoire_of_the_dead-03, fiend_hunter-02, splinterfright-02.
6. **Targeted trigger declared untargeted — ~4.** elder_cathar-01,
   bloodgift_demon-01, selhoff_occultist-01, curse_of_the_pierced_heart-02.
   Fix with the Fiend Hunter target-locking pattern (declare
   `target_requirement`, consume `chosen_targets`).
7. **~45 card-specific one-offs** — memory's_journey can't be cast,
   frightful_delusion auto-counter, grimgrin add-counters-after-leaving,
   snapcaster flashback cost, etc.

Correction to note: **unbreathing_horde-01 is still open** — it's about counting
Zombie *cards* (excluding tokens) on entry, not the fight-damage prevention the
damage pipeline fixed.

## Recommended next steps

1. Run `new_pipeline` test-phase on the ~20 fixed candidates → confirm & close.
2. Implement the **intervening-if** engine feature (closes ~14).
3. Thread `source_id` into `generate_ability_targets` targeting (closes ~8).
4. Then the enters-tapped mechanism, the card-level characteristics sweep, and
   the remaining one-offs.

Also pending: annotate `new_pipeline/prompts/auditor-insights.md` marking which
documented patterns are now fixed (committed as-is this session).
