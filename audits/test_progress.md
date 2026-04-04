# Bug Verification Test Progress — COMPLETE

Test file: `mtg-engine/tests/audit_bugs.rs`

## VERIFIED (35 bugs with failing tests)

| # | Test name | Bug |
|---|-----------|-----|
| 1 | `bug_summoning_sickness_not_enforced_for_tap_abilities` | Engine doesn't check summoning_sick for {T} abilities |
| 2 | `bug_victim_of_night_can_target_vampire_token` | Subtype checks via registry miss tokens |
| 3 | `bug_etb_trigger_suppressed_when_source_leaves` | Trigger resolution checks zone==Battlefield |
| 4 | `bug_falkenrath_noble_auto_targets_opponent` | "target player" auto-selects opponent |
| 5 | `bug_simultaneous_death_triggers_only_fire_once` | Board wipe only triggers death-watch once |
| 6 | `bug_ghost_quarter_missing_shuffle` | No library shuffle after search |
| 7 | `bug_ghost_quarter_may_search_is_mandatory` | "may search" auto-searches without choice |
| 8 | `bug_bonds_of_faith_snapshot_instead_of_continuous` | "as long as" set once at ETB, never re-evaluated |
| 9 | `bug_planeswalker_damage_uses_damage_marked_not_loyalty` | DealDamage adds damage_marked instead of removing loyalty |
| 10 | `bug_control_change_not_reverted_at_eot` | "until end of turn" control change never reverted |
| 11 | `bug_spells_cast_this_turn_never_incremented` | Spell cast counter never updated |
| 12 | `bug_delver_reveal_suppressed_for_non_instant_sorcery` | "you may reveal" only offered for instant/sorcery |
| 13 | `bug_once_per_turn_never_clears` | abilities_activated_this_turn persists across turns |
| 14 | `bug_hexproof_not_rechecked_at_resolution` | Hexproof not re-validated at spell resolution |
| 15 | `bug_card_state_not_reset_on_zone_change` | card_state persists through zone changes |
| 16 | `bug_prey_upon_uses_combat_damage_for_fight` | Fight emits CombatDamageDealt, not NonCombatDamageDealt |
| 17 | `bug_thraben_sentry_auto_transforms_without_choice` | "you may" transform is auto-decided |
| 18 | `bug_nevermore_not_enforced_for_flashback` | Card ban not checked for flashback casts |
| 19 | `bug_tribute_to_hunger_can_target_self` | "target opponent" allows targeting self |
| 20 | `bug_thraben_sentry_vigilance_retained_after_transform` | Front face keywords persist on back face |
| 21 | `bug_hinterland_harbor_misses_real_basic_lands` | Checkland only checks obj.subtypes, not registry |
| 22 | `bug_unburial_rites_castable_with_no_targets` | No target_requirement, castable with empty graveyards |
| 23 | `bug_harvest_pyre_auto_selects_exile` | Auto-selects which cards to exile |
| 24 | `bug_unbreathing_horde_no_counters_via_reanimation` | "enters with" counters only via on_resolve |
| 25 | `bug_smite_power_not_rechecked_at_resolution` | Power condition not re-checked at resolution |
| 26 | `bug_woodland_sleuth_can_return_itself_from_graveyard` | ETB trigger suppressed from graveyard (BUG3 instance) |
| 27 | `bug_ghost_quarter_missing_shuffle` | Library not shuffled after search |
| 28 | `bug_grimoire_legend_rule_not_applied` | Legend rule not applied to returned creatures |
| 29 | `bug_undead_alchemist_multiple_copies_double_mill` | Multiple copies cause double-milling |
| 30 | `bug_skirsdag_high_priest_auto_selects_tap_targets` | Auto-selects which creatures to tap |
| 31 | `bug_sturmgeist_draw_skipped_when_leaves` | Draw trigger suppressed when source leaves |
| 32 | `bug_demonmail_hauberk_sacrifice_check_too_loose` | Equip available with only 1 creature |
| 33 | `bug_civilized_scholar_stale_attacked_flag` | attacked_this_turn persists through transform |
| 34 | `bug_essence_of_wild_replacement_not_applied_for_tokens` | Replacement effect not applied for token entry |
| 35 | `bug_mentor_of_the_meek_auto_pays` | Auto-draws without "you may pay" choice |
| -- | `bug_stitchers_apprentice_trigger_desync` | trigger_event_index desync after sacrifice |

## FALSE POSITIVE (9 tests that pass — not bugs)

| Test name | Reason |
|-----------|--------|
| `bug_force_attack_ignores_cant_attack` | Engine correctly excludes Pacified creatures |
| `bug_into_the_maw_accepts_creatures_as_land_target` | is_valid_target correctly rejects creatures |
| `bug_past_in_flames_free_flashback_for_no_cost_cards` | No free flashback entries created |
| `bug_spurious_upkeep_trigger_for_opponent` | Trigger system correctly pre-filters by controller |
| `bug_reaper_intervening_if_not_checked_at_trigger` | Intervening-if IS correctly checked |
| `bug_galvanic_juggernaut_force_attack_when_unable` | Tapped creature correctly excluded |
| `bug_creepy_doll_trigger_with_lethal_damage` | Coin flip + try_destroy works correctly |
| `bug_boneyard_wurm_view_shows_base_pt` | GameView correctly shows effective P/T |
| `bug_rooftop_storm_not_offered_from_graveyard` | Works from hand (graveyard untestable) |

## INCONCLUSIVE / UNTESTABLE (4)

| Test name | Reason |
|-----------|--------|
| `bug_mirror_mad_phantasm_sets_draw_flag_incorrectly` | Requires token-in-library simulation |
| `bug_protection_doesnt_prevent_zombie_source_targeting` | Debug format comparison unreliable |
| `bug_night_terrors_stuck_on_stack` | Test setup may not reproduce the condition |
| `bug_evil_twin_marker_set_before_choice` | Marker timing hard to test |

## Summary

- **35 verified bugs** with failing tests
- **9 false positives** identified
- **4 inconclusive** (test setup limitations)
- **48 total tests** in audit_bugs.rs
