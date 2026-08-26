# Where a test goes

Every file in `mtg-engine/tests/` compiles to its own binary. That is the
constraint that shapes this layout: a file per card would mean ~250 more
binaries and a slower build for everyone, so tests are grouped, and the
grouping is by **the rule under test** — with one exception, below.

The suite used to be named after the process that produced each file:
`audit_bugs.rs`, `pipeline_bugs_merged_*.rs`, `tier15_cards.rs`. Nothing about
those names said what was inside, so the only way to find "the tests for
replacement effects" was to grep. If you are adding a file, name it after the
rule or the card, never after the run, the ticket, or the batch.

## Deciding

**Is your test about a rule the engine implements generally?**
Put it in the file for that rule. If none fits, make one, named for the rule.

| area | file |
|---|---|
| replacement effects (CR 614) | `replacement_effects.rs`, `enters_tapped_replacement.rs` |
| triggered abilities (CR 603) | `trigger_dispatch.rs`, `trigger_snapshots.rs`, `trigger_priority.rs`, `intervening_if.rs`, `enter_trigger_conditions.rs`, `empty_triggers.rs`, `phantom_triggers.rs` |
| a trigger outliving its source (CR 113.7a) | `trigger_source_independence.rs`, `trigger_independence.rs` |
| trigger targets (CR 603.3d, 608.2b) | `trigger_targets_declared.rs`, `trigger_target_recheck.rs` |
| continuous effects (CR 611) | `continuous_effects.rs`, `snapshot_anthems.rs`, `attacking_creatures_anthem.rs` |
| costs (CR 601.2b/f) | `spell_costs.rs`, `tap_cost_legality.rs`, `counter_costs.rs`, `xcost_mana.rs`, `x_cost_spells.rs`, `x_cost_funding_flow.rs`, `funding_build_options.rs` |
| mana | `lands_and_mana.rs`, `mana_filters.rs`, `mana_tap_bug.rs`, `equipment_autotap.rs` |
| targeting and legality | `characteristics_targeting.rs`, `ability_target_protection.rs`, `hexproof_filter.rs`, `player_protection.rs`, `resolution_time_checks.rs` |
| fizzling | `fizzle.rs` |
| regeneration (CR 701.15) | `regeneration.rs` |
| planeswalkers, loyalty (CR 306, 606, 704.5i) | `planeswalkers.rs` |
| combat | `combat.rs`, `combat_rules.rs`, `combat_regressions.rs` |
| damage | `damage_pipeline.rs`, `damage_helper.rs` (any target includes planeswalkers), `inline_damage.rs` |
| state-based actions (CR 704) | `state_based_actions.rs`, `evil_twin_sba_guard.rs` |
| copying (CR 706) | `copy_effects.rs` (what is copied), `token_copy.rs` (what `create_token_*` must carry across) |
| transform / DFCs (CR 712) | `transform_dfc.rs`, `dfc_zone_cleanup.rs`, `transformed_display.rs`, `werewolf_cards.rs`, `werewolf_subtype_after_transform.rs` |
| zones and object identity (CR 400.7) | `zones_and_state.rs`, `zone_change_resets_object.rs`, `until_eot_object_identity.rs`, `token_is_not_a_card.rs` |
| control and duration | `control_change.rs`, `control_durations.rs`, `enters_under_control.rs` |
| turn structure and priority | `turn_structure.rs`, `priority.rs`, `apnap.rs`, `your_upkeep_scope.rs`, `instant_interaction.rs` |
| choices the engine must not make for a player | `auto_pick.rs`, `sacrifice_choice.rs` |
| what the player is shown | `harness_display.rs` |
| characteristics (the `state.rs` layer) | `characteristics_invariant.rs`, `characteristics_card_sweep.rs`, `subtype.rs`, `keywords.rs` |

**Is it "does this card do what its oracle text says"?**
That is the exception. Those go in a `cards_*.rs` file, grouped by what the
cards have in common, because the useful index for an acceptance test is the
card name and `grep` already provides it. Each file's module doc lists the
cards it covers, so browsing works too.

A card with enough tests to be worth its own file gets one, named after the
card — `geist_of_saint_traft.rs`, `olivia_voldaren.rs`, `moonmist.rs`.

**Is it a guard rather than a behaviour test?**
Some tests read the source tree and fail if an invariant is broken — one
replacement mechanism, one trigger construction site, one continuous-effect
walk, one cost determination. They live beside the behaviour tests for the
same rule. Add one when a refactor establishes an invariant that nothing else
would notice being violated:

- `characteristics_invariant.rs` — card code reads characteristics through the accessors
- `engine_knows_no_cards.rs` — no card names in the engine
- `replacement_effects.rs::replacement_has_exactly_one_mechanism`
- `trigger_source_independence.rs::triggers_are_built_in_one_place`
- `continuous_effects.rs::continuous_effects_are_read_in_one_place`
- `spell_costs.rs::spell_costs_are_determined_in_one_place`

## Helpers

`tests/common/mod.rs` holds the shared setup: `registry()`, `game_at_step`,
`named_creature`, `spell_in_hand`, `attach_curse_to_player`, `counters_of`,
and the rest. Every test file does `mod common; use common::*;`.

Do not write a local copy of a helper that already exists there — `registry()`
alone had been written out 89 times. If your setup is genuinely different
(`player_protection.rs` builds a registry with an extra card registered), that
is a different helper, not a copy, and it can stay local.

## Running

    ANTHROPIC_API_KEY=dummy cargo test --workspace

The key can be anything; without it the six `llm_conversation` tests in
`mtg-player` fail at construction.

Check the exit code, not a grep of the output — a test binary that fails to
compile reports zero failures because its tests never ran. See the "Verifying
test results" section of `CLAUDE.md`.
