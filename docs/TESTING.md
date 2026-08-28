# Where a test goes

Every file in `mtg-engine/tests/` compiles to its own binary. That is the
constraint that shapes this layout: a file per card would mean ~250 more
binaries and a slower build for everyone, so tests are grouped, and the
grouping is by **the rule under test** — with one exception, below.

The suite used to be named after the process that produced each file:
`audit_bugs.rs`, `pipeline_bugs_merged_*.rs`, `tier15_cards.rs`,
`engine_regressions.rs`, `combat_regressions.rs`. Nothing about those names
said what was inside, so the only way to find "the tests for replacement
effects" was to grep. If you are adding a file, name it after the rule or the
card, never after the run, the ticket, or the batch.

Two guards in `test_suite_guards.rs` keep this page honest:
`the_testing_guide_names_every_rule_test_file` fails the build if a rule file
is missing from the tables below or if this page names one that is gone, and
`a_cards_file_covers_exactly_the_cards_its_module_doc_lists` does the same for
each `cards_*.rs` file's own card index.

## Deciding

**Is your test about a rule the engine implements generally?**
Put it in the file for that rule. If none fits, make one, named for the rule.

| area | file |
|---|---|
| replacement effects (CR 614) | `replacement_effects.rs`, `enters_tapped_replacement.rs` |
| triggered abilities (CR 603) | `trigger_dispatch.rs`, `trigger_snapshots.rs`, `trigger_priority.rs`, `intervening_if.rs`, `enter_trigger_conditions.rs`, `empty_triggers.rs`, `phantom_triggers.rs`, `simultaneous_events.rs` |
| a trigger outliving its source (CR 113.7a) | `trigger_source_independence.rs`, `trigger_independence.rs`, `ltb_controller.rs` |
| trigger targets (CR 603.3d, 608.2b) | `trigger_targets_declared.rs`, `trigger_target_recheck.rs` |
| whose upkeep / whose permanent a trigger watches (CR 603.2) | `your_upkeep_scope.rs`, `curse_and_equip_scope.rs` |
| continuous effects (CR 611) | `continuous_effects.rs`, `snapshot_anthems.rs`, `attacking_creatures_anthem.rs`, `equipment_human_conditional.rs` |
| costs (CR 601.2b/f) | `spell_costs.rs`, `tap_cost_legality.rs`, `counter_costs.rs`, `xcost_mana.rs`, `x_cost_spells.rs`, `x_cost_funding_flow.rs`, `funding_build_options.rs` |
| mana | `lands_and_mana.rs`, `mana_filters.rs`, `mana_ability_offers.rs`, `equipment_autotap.rs` |
| casting and resolution (CR 601, 608) | `spells.rs`, `spell_cleanup.rs`, `multi_target_and_mill.rs`, `instant_interaction.rs` |
| flashback (CR 702.33) | `flashback.rs`, `flashback_multiple_instances.rs` |
| targeting and legality | `characteristics_targeting.rs`, `ability_target_protection.rs`, `hexproof_filter.rs`, `player_protection.rs`, `resolution_time_checks.rs`, `submitted_targets.rs` |
| fizzling | `fizzle.rs` |
| activated abilities (CR 602) | `activated_abilities.rs`, `activated_no_stack.rs` |
| regeneration (CR 701.15) | `regeneration.rs` |
| summoning sickness (CR 302.6) | `summoning_sickness.rs` |
| planeswalkers, loyalty (CR 306, 606, 704.5i) | `planeswalkers.rs` |
| combat | `combat.rs`, `combat_rules.rs` |
| damage | `damage_pipeline.rs`, `damage_helper.rs` (any target includes planeswalkers), `inline_damage.rs` |
| state-based actions (CR 704) | `state_based_actions.rs` |
| tokens and counters | `tokens_counters_triggers.rs`, `token_is_not_a_card.rs`, `token_copy.rs` |
| copying (CR 706) | `copy_effects.rs` (what is copied), `token_copy.rs` (what `create_token_*` must carry across) |
| transform / DFCs (CR 712) | `transform_dfc.rs`, `dfc_zone_cleanup.rs`, `transformed_display.rs`, `werewolf_cards.rs`, `werewolf_subtype_after_transform.rs` |
| zones and object identity (CR 400.7) | `zones_and_state.rs`, `zone_change_resets_object.rs`, `until_eot_object_identity.rs`, `token_is_not_a_card.rs` |
| control and duration | `control_change.rs`, `control_durations.rs`, `enters_under_control.rs` |
| turn structure and priority | `turn_structure.rs`, `priority.rs`, `apnap.rs` |
| starting the game (CR 103) | `mulligan.rs`, `match_play_draw.rs` |
| choices the engine must not make for a player | `auto_pick.rs`, `sacrifice_choice.rs` |
| what the player is shown | `harness_display.rs` |
| characteristics (the `state.rs` layer) | `characteristics_invariant.rs`, `characteristics_card_sweep.rs`, `card_data_invariants.rs`, `subtype.rs`, `keywords.rs`, `enchantments.rs` |

**Is it "does this card do what its oracle text says"?**
That is the exception. Those go in a `cards_*.rs` file, grouped by what the
cards have in common, because the useful index for an acceptance test is the
card name and `grep` already provides it. Each file's module doc lists the
cards it covers — that list is checked against the file, so browsing works
too.

| what the cards have in common | file |
|---|---|
| several interacting abilities at once | `cards_complex_creatures.rs` |
| turning into something else | `cards_transforming_permanents.rs` |
| vanilla and keyword creatures, combat instants, auras | `cards_vanilla_and_keywords.rs` |
| combat damage triggers | `cards_combat_damage_triggers.rs` |
| death triggers and token makers | `cards_death_triggers_and_tokens.rs` |
| morbid, and leaves-the-battlefield | `cards_morbid_and_ltb.rs` |
| upkeep triggers and Curses | `cards_upkeep_triggers_and_curses.rs` |
| removal and bounce | `cards_removal_and_bounce.rs` |
| graveyard interaction | `cards_graveyard_interaction.rs` |
| lands and other mana sources | `cards_lands_and_mana_sources.rs` |
| equipment | `cards_equipment_and_artifacts.rs`, `cards_equipment_costs.rs` |
| sacrifice and other additional costs | `cards_sacrifice_and_additional_costs.rs` |
| activated abilities on permanents | `cards_activated_abilities.rs` |
| spells and enchantments | `cards_spells_and_enchantments.rs` |
| evasion, and P/T that depends on the board | `cards_evasion_and_graveyard_pt.rs` |
| cards that change what the rules allow | `cards_rule_modifiers.rs` |
| where the implementation takes a shortcut | `cards_shortcuts_taken.rs` |

A card with enough tests to be worth its own binary gets its own file, named
after the card:

| card | file |
|---|---|
| Geist of Saint Traft, and the delayed end-of-combat exile (CR 603.7d) | `geist_of_saint_traft.rs` |
| Olivia Voldaren | `olivia_voldaren.rs` |
| Moonmist | `moonmist.rs` |
| Graveyard Shovel | `graveyard_shovel.rs` |

"Enough" has meant five or more in practice — thirteen
files with three or four tests apiece were folded back into the `cards_*.rs`
group, because a binary per three assertions costs everyone build time and
tells a reader nothing the module doc's card list would not.

**Is it a guard rather than a behaviour test?**
Some tests read the source tree and fail if an invariant is broken — one
replacement mechanism, one trigger construction site, one continuous-effect
walk, one cost determination, one owner for spell cleanup. They live beside
the behaviour tests for the same rule, except the ones about the test suite
itself, which live in `test_suite_guards.rs`. Add one when a refactor
establishes an invariant that nothing else would notice being violated:

- `characteristics_invariant.rs` — card code reads characteristics through the accessors
- `engine_knows_no_cards.rs` — no card names in the engine
- `replacement_effects.rs::replacement_has_exactly_one_mechanism`
- `trigger_source_independence.rs::triggers_are_built_in_one_place`
- `continuous_effects.rs::continuous_effects_are_read_in_one_place`
- `spell_costs.rs::spell_costs_are_determined_in_one_place`
- `test_suite_guards.rs` — the guards on the suite itself: no test may claim
  it is failing, call a hook the card leaves at its default, assert a card's
  own data back at itself, end the turn by hand, assemble a `CombatState` by
  hand, or name a source file that does not exist; no card may move a spell
  off the stack itself; only the damage pipeline marks damage or removes
  loyalty for it; and this page must name every rule file.

## Helpers

`tests/common/mod.rs` holds the shared setup: `registry()`, `game_at_step`,
`named_permanent`, `spell_in_hand`, `declare_combat` / `attacks_unblocked` /
`attacks_blocked_by`, `attach_curse_to_player`, `counters_of`, and the rest.
Every test file does `mod common; use common::*;`.

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
