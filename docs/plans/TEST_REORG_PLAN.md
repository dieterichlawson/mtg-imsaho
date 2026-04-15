# Test Suite Reorganisation Plan

Follow-on to [TEST_SUITE_AUDIT_PLAN.md](./TEST_SUITE_AUDIT_PLAN.md). The audit
and cleanup passes left the suite functionally healthy but structurally
muddled: 85 flat files in `tests/` organised by four overlapping axes
(implementation tier, per-card, mechanic family, bug provenance). This plan
reshapes the directory layout so a reader can answer "is this behaviour
tested?" by navigating to one obvious location.

## Goals

1. **Card-specific tests** live under `tests/cards/<set>/<card>.rs` — one
   card per file, grouped by set.
2. **Engine / rules tests** live under `tests/engine/<mechanic>.rs` — one
   mechanic per file.
3. **Bug regressions** live under `tests/regressions/<family>.rs` — one
   file per audit family, consolidating the `audit_*_family.rs` and
   `bug_fixes.rs` / `card_fixes.rs` / `engine_bugs.rs` scatter.
4. **One binary per directory.** Each subdirectory compiles as a single
   integration-test binary via `tests/<dir>/main.rs` that `mod`-declares
   each file. This reduces link time (currently 85 linker runs → 3-4)
   and enables shared setup within a binary.
5. **Preserve git history.** All moves use `git mv`; move and refactor
   happen in separate commits.

## Target layout

```
tests/
  common/mod.rs                        # (unchanged) shared helpers
  
  engine/
    main.rs
    combat.rs
    keywords.rs
    activated_abilities.rs
    flashback.rs
    mana.rs                            # lands_and_mana + mana_tap_bug
    priority.rs                        # priority + apnap
    sba.rs                             # state_based_actions
    stack.rs                           # fizzle + spell_fizzle + instant_interaction
    triggers.rs                        # trigger_priority + phantom_triggers + empty_triggers
    mulligan.rs
    match_play_draw.rs
    turn_structure.rs                  # turn_structure + summoning_sickness + zones_and_state
    x_cost.rs                          # x_cost_spells + x_cost_funding_flow + funding_build_options
    view_layer.rs                      # transformed_display (renamed)
    death_triggers.rs                  # death_trigger_bugs
    ltb.rs                             # ltb_controller
    sacrifice.rs                       # sacrifice_choice
    equipment_mechanics.rs             # equipment_autotap + Human-conditional parametric test
    tokens_counters_triggers.rs
    edge_cases.rs                      # keep as-is for now
  
  cards/
    isd/
      main.rs
      avacynian_priest.rs
      back_from_the_brink.rs
      bloodcrazed_neonate.rs
      bloodline_keeper.rs
      burning_vengeance.rs
      champion_of_the_parish.rs
      charmbreaker_devils.rs
      civilized_scholar.rs
      creepy_doll.rs
      curse_of_bloody_tome.rs
      curse_of_deaths_hold.rs
      curse_of_nightly_hunt.rs
      curse_of_oblivion.rs
      curse_of_pierced_heart.rs
      curse_of_stalked_prey.rs
      daybreak_ranger.rs
      dearly_departed.rs
      devils_play.rs
      dual_lands.rs                    # parametric over 5 checklands
      gatstaf_shepherd.rs
      geist_of_saint_traft.rs
      ghoulcallers_chant.rs
      graveyard_shovel.rs
      gutter_grime.rs
      hanweir_watchkeep.rs
      harvest_pyre.rs
      howlpack_alpha.rs                # Mayor of Avabruck back face
      human_conditional_equipment.rs   # parametric for Dagger/Cleaver/Pitchfork
      infernal_plunge.rs
      inquisitors_flail.rs
      kessig_wolf_run.rs
      kruin_outlaw.rs
      mayor_of_avabruck.rs
      memorys_journey.rs
      mentor_of_the_meek.rs
      mikaeus_the_lunarch.rs
      moonmist.rs
      olivia_voldaren.rs
      reckless_waif.rs
      splinterfright.rs
      stitchers_apprentice.rs
      stromkirk_noble.rs
      thraben_purebloods.rs
      tormented_pariah.rs
      trepanation_blade.rs
      ulvenwald_mystics.rs
      unbreathing_horde.rs
      villagers_of_estwald.rs
      werewolf_transform_rules.rs      # multi-werewolf common-behavior tests
      witchbane_orb.rs
      wooden_stake.rs
      # ... roughly 80 files, one per card
    
    core/                              # basic / non-set cards
      main.rs
      counterspell.rs
      direct_damage_spells.rs          # parametric for Lightning Bolt, Lava Axe, etc.
      divination.rs
      doom_blade.rs
      giant_growth.rs
      kalonian_tusker.rs
      lightning_bolt.rs                # creature-kill-specific tests only
      pacifism.rs
      swords_to_plowshares.rs
  
  regressions/
    main.rs
    auto_pick.rs                       # audit_auto_pick_family
    combat_rules.rs                    # audit_combat_rules_family
    damage_helper.rs                   # audit_damage_helper_family
    harness_display.rs                 # audit_harness_display_family
    hexproof_filter.rs                 # audit_hexproof_filter_family
    misc.rs                            # audit_misc_final + bug_fixes + card_fixes + engine_bugs
    snapshot_anthems.rs                # audit_snapshot_anthems_family
    subtype.rs                         # audit_subtype_family
    token_copy.rs                      # audit_token_copy_family
    transform_dfc.rs                   # audit_transform_dfc_family
    trigger_dispatch.rs                # audit_trigger_dispatch_family
    xcost_mana.rs                      # audit_xcost_mana_family
    sonnet_4_6_audit.rs                # audit_bugs + audit_bugs2 consolidated
```

### Rust build mechanics

Each subdirectory is a single integration-test binary. `tests/engine/main.rs`:
```rust
#[path = "../common/mod.rs"]
mod common;

mod combat;
mod keywords;
mod activated_abilities;
// ... etc.
```

Each submodule (`tests/engine/combat.rs`) loses its `mod common;` line —
`common` is already imported as a sibling module from `main.rs`.

`Cargo.toml` gains three entries:
```toml
[[test]]
name = "engine"
path = "tests/engine/main.rs"

[[test]]
name = "cards_isd"
path = "tests/cards/isd/main.rs"

[[test]]
name = "cards_core"
path = "tests/cards/core/main.rs"

[[test]]
name = "regressions"
path = "tests/regressions/main.rs"
```

Cargo's default auto-discovery of `tests/*.rs` will stop finding anything
once the files are moved; no explicit opt-out needed.

## File-by-file assignment

### Engine-only (moves to `tests/engine/`)

Straightforward mechanics tests, one card used per scenario but testing
the rule:

- `combat.rs` → `engine/combat.rs`
- `keywords.rs` → `engine/keywords.rs`
- `activated_abilities.rs` → `engine/activated_abilities.rs`
- `flashback.rs` → `engine/flashback.rs`
- `lands_and_mana.rs` + `mana_tap_bug.rs` → `engine/mana.rs`
- `priority.rs` + `apnap.rs` → `engine/priority.rs`
- `state_based_actions.rs` → `engine/sba.rs`
- `fizzle.rs` + `spell_fizzle.rs` + `instant_interaction.rs` → `engine/stack.rs`
- `trigger_priority.rs` + `phantom_triggers.rs` + `empty_triggers.rs` → `engine/triggers.rs`
- `mulligan.rs` → `engine/mulligan.rs`
- `match_play_draw.rs` → `engine/match_play_draw.rs`
- `turn_structure.rs` + `summoning_sickness.rs` + `zones_and_state.rs` → `engine/turn_structure.rs`
- `x_cost_spells.rs` + `x_cost_funding_flow.rs` + `funding_build_options.rs` → `engine/x_cost.rs`
- `transformed_display.rs` → `engine/view_layer.rs`
- `death_trigger_bugs.rs` → `engine/death_triggers.rs`
- `ltb_controller.rs` → `engine/ltb.rs`
- `sacrifice_choice.rs` → `engine/sacrifice.rs`
- `equipment_autotap.rs` + existing `equipment_human_conditional.rs` → `engine/equipment_mechanics.rs`
- `tokens_counters_triggers.rs` → `engine/tokens_counters_triggers.rs`
- `edge_cases.rs` → `engine/edge_cases.rs`

### Card-specific (moves to `tests/cards/isd/`)

Existing single-card files (~14): `creepy_doll`, `olivia_voldaren`,
`geist_of_saint_traft`, `gutter_grime`, `infernal_plunge`, `unbreathing_horde`,
`civilized_scholar_triggers`, `graveyard_shovel`, `ghoulcallers_chant`,
`witchbane_orb`, `kessig_wolf_run`, `kruin_outlaw`, `memorys_journey`,
`moonmist`, `inquisitors_flail`.

Parametric multi-card that stay in `cards/isd/` as "dual_lands.rs" and
"human_conditional_equipment.rs". Werewolves stay consolidated in
`werewolves.rs` or `werewolf_transform_rules.rs` (multi-card by design).

### Non-ISD cards (moves to `tests/cards/core/`)

- `spells.rs` → split into `lightning_bolt.rs`, `counterspell.rs`,
  `divination.rs`, `doom_blade.rs`, `direct_damage_spells.rs` (parametric),
  etc.

### Regression families (moves to `tests/regressions/`)

- 11 `audit_*_family.rs` files → rename to drop the prefix
  (`audit_auto_pick_family.rs` → `auto_pick.rs`)
- `audit_bugs.rs` + `audit_bugs2.rs` + `audit_misc_final.rs` →
  `sonnet_4_6_audit.rs` (they're all from the same audit run)
- `bug_fixes.rs` + `card_fixes.rs` + `engine_bugs.rs` → merge into
  `misc.rs`

### Files that must be split

These contain tests for many distinct cards and need to be broken up
before the move:

- `tier2_spells.rs` (17 tests): split per spell (Rebuke, Victim of Night,
  Geistflame, Brimstone Volley, Dissipate, Frightful Delusion, Smite the
  Monstrous, Urgent Exorcism, etc.) into `cards/isd/` per-card files.
- `tier3_cards.rs` (18 tests): per card (Doomed Traveler, Mausoleum
  Guard, Moan of the Unhallowed, Silverchase Fox, etc.).
- `tier5_cards.rs` (12 tests), `tier6_cards.rs` (18 tests),
  `tier7_cards.rs` (13 tests), `tier8_cards.rs` (29 tests),
  `tier9_cards.rs` (22 tests), `tier9_equipment.rs` (22 tests),
  `tier10_cards.rs` (18 tests), `tier11_cards.rs` (13 tests),
  `tier12_cards.rs` (18 tests), `tier14_cards.rs` (21 tests),
  `tier15_cards.rs` (97 tests): same — split per card.
- `innistrad_cards.rs` (28 tests), `innistrad_simple_cards.rs` (34
  tests): same. Dual-land parametric test stays as `dual_lands.rs`.
- `spells.rs` (15 tests) → split into `cards/core/` per card.
- `enchantments.rs` (9 tests) → judge per-test; likely split between
  `engine/edge_cases.rs` and `cards/isd/` per-card.
- `card_mechanics.rs` (44 tests) — the most mixed file; needs
  per-test review.
- `card_shortcuts.rs` (13 tests) — same; per-test review.
- `werewolf_cards.rs` (34 tests) — keep most together in
  `cards/isd/werewolves.rs`; split off `werewolf_subtype_after_transform.rs`
  if it's tracking a separate bug.

## Ambiguous files — judgment calls needed

- **`keywords.rs`** tests keyword mechanics *using* specific cards (Abbey
  Griffin for vigilance, Grave Bramble for defender, etc.). Engine or
  card? I'd put it in `engine/keywords.rs` — the tests check the *rule*,
  the card is incidental.
- **`card_mechanics.rs`** is 44 tests of mixed origin. Probably split
  half into `engine/` (morbid, protection rules, token anthem math) and
  half into `cards/isd/` (specific-card tests for Elder Cathar, Grave
  Bramble, etc.).
- **`edge_cases.rs`** is a coverage-gap catch-all. Keep as
  `engine/edge_cases.rs` for now; each test is its own thing.
- **`card_shortcuts.rs`** is failing regression tests for implementation
  corners. Consider: `regressions/card_shortcuts.rs`.

## Execution plan — phased for safety

### Phase 1 — split multi-card files (no moves yet)

**Goal:** every card's tests live in a dedicated top-level file before
anything moves directories.

For each of the 14 multi-card files, one commit that:
1. Creates new per-card files at the `tests/` root (e.g.,
   `tests/tier8_stitchers_apprentice.rs`).
2. `git mv`s nothing yet — the source file is edited, tests cut and
   pasted into the new files.
3. Leaves the source file with whatever is genuinely shared (rare).

Commits of the form "Split tier8_cards.rs into per-card files".

Estimated: ~15 commits. Each is a mechanical copy-paste plus an
`#[test]` dedup check.

### Phase 2 — directory skeleton

One commit:
- Create `tests/engine/`, `tests/cards/isd/`, `tests/cards/core/`,
  `tests/regressions/`.
- Empty `main.rs` in each with `#[path = "../common/mod.rs"] mod common;`
  and no submodule declarations.
- Add `[[test]]` entries to `Cargo.toml`.

At this point the new binaries compile but contain no tests.

### Phase 3 — move engine tests

One commit per directory's worth of moves is ideal for reviewability:
- `git mv tests/combat.rs tests/engine/combat.rs` (etc.)
- Update each file to delete the `mod common;` line (now inherited
  from `main.rs`).
- Update `tests/engine/main.rs` to add `mod combat;` (etc.).
- Run `cargo test --test engine` to confirm all engine tests still pass.

Roughly 3-4 commits, grouped by theme (combat/priority/stack, mana/x_cost,
turn/mulligan, everything else).

### Phase 4 — move card tests

Same pattern per set directory:
- `tests/cards/isd/main.rs` gets a `mod creepy_doll; mod olivia_voldaren; …`
- Move the files with `git mv`.
- Edit each to drop `mod common;`.

Probably one commit per letter range to keep PRs reviewable
(a-d, e-l, m-r, s-z).

### Phase 5 — move regressions

One commit.

### Phase 6 — cleanup

- Delete the (now-empty) top-level test files that got consolidated.
- Verify `find tests -maxdepth 1 -name "*.rs"` returns nothing.
- Update `CLAUDE.md`'s "Repository layout" section with the new convention.
- Update `docs/plans/TEST_SUITE_AUDIT_PLAN.md` status.

### Phase 7 — (optional) compile-time win

Measure `cargo test --no-run` before and after. Expected: noticeably
faster, maybe 30-50%, because we're down from 85 linker invocations to
4.

## Risks

- **Tests that depend on other tests' side effects.** None currently do
  (Rust integration tests are isolated), but moving files into one
  binary means they share a process. Tests that mutate global state
  (e.g., `rand::thread_rng()`) could interfere. Mitigation: run
  `cargo test` after each phase.
- **A split missing a helper.** Multi-card files sometimes have a
  file-local helper function used by several tests. When splitting,
  either promote to `common/mod.rs` or duplicate into each resulting
  file. Review per split.
- **Git blame churn.** `git mv` preserves history, but viewing blame
  across directories is harder. Mitigation: *never* combine a move with
  a content edit in the same commit. Move, then edit in a follow-up
  commit so `git blame --follow` works cleanly.
- **Cargo.toml conflicts.** The `[[test]]` entries are merge-hostile if
  multiple branches add them. Land the skeleton commit (Phase 2) first
  and rebase the rest on top.

## Non-goals

- **No coverage changes.** The reorg should preserve every single test
  verbatim; moves are moves only.
- **No new parametric tests.** The earlier dedup pass consolidated
  what's genuinely shared; don't invent new parametrizations during
  the move.
- **No renaming tests.** The test *function names* stay as-is; only
  file locations change.

## Open questions to resolve before starting

1. **Is `regressions/` the right category?** Alternative: fold each
   audit family into the corresponding `engine/` or `cards/` file. Pros:
   single location for a mechanic's coverage. Cons: loses the audit's
   provenance narrative ("this was found by Sonnet 4.6 in March 2026")
   and mixes regressions with in-domain tests.
   Recommended: keep `regressions/` for provenance, add pointer comments
   from `engine/combat.rs` to `regressions/combat_rules.rs`.

2. **How aggressive to be with `keywords.rs` splitting?** Current file
   is 24 tests covering 8+ keywords. Could become `engine/keywords/*.rs`
   (per-keyword file within a keywords subdir). Recommended: not yet;
   one file per mechanic is simpler and we can split later if it grows.

3. **Where do parametric tests live?** `dual_lands.rs` tests five
   cards. It's clearly "card tests" but not "one card's tests". Putting
   it under `cards/isd/dual_lands.rs` is correct; future parametric
   card tests follow the same rule.

4. **What about `phantom_triggers.rs` and `empty_triggers.rs`?** These
   are small engine-side trigger-mechanic tests. Consolidate into
   `engine/triggers.rs` or keep separate? Recommended: consolidate —
   they're short and thematically identical.

## Estimated effort

- Phase 1 (splits): 4-6 hours, highest friction (care per test).
- Phase 2-5 (moves): 2-3 hours, mostly mechanical.
- Phase 6 (cleanup): 30 minutes.
- Phase 7 (measure): 15 minutes.

Total: ~1 day of focused work, spread across ~20 commits.

## Suggested first step

Before starting any moves, do a **dry-run classification pass**: create
this file (done) and a companion spreadsheet / markdown table listing
every current test file and its target destination. Review the
classification, then start Phase 1. The spreadsheet becomes a
commit-by-commit checklist.
