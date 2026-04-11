# Audit test progress

Flat index of which bugs in `audits/AUDIT_BUGS.md` have failing-tests
written against them. Each entry is in the format:

`- Bug XX (Family) — path::test_name — status: <status>`

Status legend:
- `failing-as-expected` — test runs, fails for the documented reason.
  Will pass once the bug is fixed.
- `not-reproduced` — investigation showed the bug doesn't exist (or has
  already been fixed). One-line note explains what was checked.
- `skipped-judgment-call` — harness/UI bug whose "correct" behavior is
  subjective; a test would encode an arbitrary preference.
- `blocked` — writing the test requires infrastructure that doesn't yet
  exist. One-line note describes the blocker.

See `prompts/AUDIT_TEST_AGENT_PROMPT.md` for the workflow.

---

## Tests

- Bug BD (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_bd_setup_game_populates_obj_subtypes_from_registry` — status: failing-as-expected
- Bug AX (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_ax_woodland_cemetery_untapped_with_swamp_in_play` — status: failing-as-expected
- Bug AT (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_at_slayer_of_the_wicked_targets_vampire_token` — status: failing-as-expected
- Bug AY (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_ay_olivia_vampire_steal_can_target_registry_vampire` — status: failing-as-expected
- Bug AU (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_au_moonmist_transforms_olivia_bitten_human_dfc` — status: failing-as-expected
- Bug 31-003 (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_31_003_urgent_exorcism_targets_spirit_token` — status: failing-as-expected
- Bug 31-002 (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_31_002_avacynian_priest_can_tap_transformed_werewolf` — status: failing-as-expected
- Bug 31-004 (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_31_004_elder_cathar_no_bonus_on_transformed_werewolf` — status: failing-as-expected
- Bug AO (Subtype filter family) — none — status: blocked — `combat::get_subtypes` is a private helper whose only in-set caller is Moonmist's combat-damage prevention, and Moonmist only checks Werewolf/Wolf (which all ISD werewolf back faces still carry), so no observable behavior path exposes the latent union-of-faces bug today
- Bug 99-002 (Subtype filter family) — `mtg-engine/tests/audit_subtype_family.rs::bug_99_002_delver_transform_updates_obj_subtypes` — status: failing-as-expected
- Bug T (Damage helper bypass) — `mtg-engine/tests/audit_damage_helper_family.rs::bug_t_skirsdag_cultist_pushes_damaged_by` — status: failing-as-expected
- Bug T (Damage helper bypass) — `mtg-engine/tests/audit_damage_helper_family.rs::bug_t_rolling_temblor_pushes_damaged_by` — status: failing-as-expected
- Bug BQ (Damage helper bypass) — `mtg-engine/tests/audit_damage_helper_family.rs::bug_bq_brimstone_volley_can_target_planeswalker` — status: failing-as-expected
- Bug BZ (Damage helper bypass) — `mtg-engine/tests/audit_damage_helper_family.rs::bug_bz_pitchburn_devils_offers_planeswalker_as_target` — status: failing-as-expected
- Bug 9F-002 (Damage helper bypass) — none — status: blocked — meta refactor bug; the individual consequences are tested by Bugs T (damaged_by hygiene), BQ/BZ (planeswalker enumeration), and BR (Olivia/Curse bypass). 9F-002 itself is the umbrella refactor and has no separate observable behavior beyond its constituent bugs
- Bug BR (Damage helper bypass) — none — status: blocked — Olivia's bite and Curse of the Pierced Heart's life-subtract bypass the central damage helper, but the observable consequences (skipped protection, planeswalker loyalty, lifelink interaction) all require additional in-set features that don't exist (no protection-from-source on the right targets, planeswalker enumeration is itself broken via Bug BQ, no lifelink-grant on Olivia). With Bug BQ unfixed, Olivia can't even be made to target a planeswalker via the legal-actions pipeline
- Bug 17-003 (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_17_003_pitchburn_devils_does_not_offer_opponent_hexproof_creature` — status: failing-as-expected
- Bug E1-001 (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_e1_001_grimgrin_attack_trigger_excludes_opponent_hexproof_creature` — status: failing-as-expected
- Bug 0F-003 (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_0f_003_falkenrath_noble_skips_player_with_witchbane_orb` — status: failing-as-expected
- Bug H (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_h_maw_of_hell_first_target_must_be_a_land` — status: failing-as-expected
- Bug AW (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_aw_prey_upon_rejects_two_of_your_own_creatures` — status: failing-as-expected
- Bug AD (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_ad_unburial_rites_only_targets_casters_graveyard` — status: failing-as-expected
- Bug 9F-001 (Hexproof / target-filter) — `mtg-engine/tests/audit_hexproof_filter_family.rs::bug_9f_001_snapcaster_can_grant_flashback_to_card_with_printed_flashback` — status: failing-as-expected
- Bug O (Hexproof / target-filter) — none — status: blocked — Memory's Journey's GraveyardCard enumerator is structurally similar to Bug AD, but the failure mode (opp's graveyard cards reachable via the player+graveyard TwoTargets pair) requires the player target to be different from the card-owner — testing this would require the resolution-time path, which the current Memory's Journey implementation auto-resolves via UpToTargets in a way that's difficult to assert against without exposing internal state. Recommend testing once the fix introduces a `GraveyardCardOf(target_index)` variant
- Bug 0F-002 (Token copy) — `mtg-engine/tests/audit_token_copy_family.rs::bug_0f_002_token_copy_of_legendary_creature_is_legendary` — status: failing-as-expected
- Bug BJ (Token copy) — `mtg-engine/tests/audit_token_copy_family.rs::bug_bj_evil_twin_survives_sba_before_copy_effect_resolves` — status: failing-as-expected
- Bug AV (Token copy) — none — status: not-reproduced — `state.create_token_copy` already patches `obj.card_id` after the `create_token_with_subtypes` call (state.rs:444), so `effective_power`/`effective_toughness` consults `dynamic_pt` via the registry and the token reports the correct CDA-derived P/T. The audit entry's "0/0 dies to SBA" claim doesn't reproduce on the primary token. The doubled-Parallel-Lives variant (Bug 0F-001) is genuinely buggy because the patch only touches the first id; that's tracked separately
- Bug 0F-001 (Token copy) — none — status: blocked — needs Parallel Lives setup AND a token-copy effect; requires reaching past `create_token_with_subtypes` to verify that doubled tokens have stale `card_id`. The helper's single-id return makes the doubled-token state hard to inspect without exposing internals
- Bug 4D-001 (Token copy) — none — status: blocked — same Parallel Lives setup; observable consequences (Army of the Damned doubled tokens not tapped, Kessig Cagebreakers doubled tokens not attacking) require both the doubler to be in play and a doubling-aware test for combat state
- Bug BY (Token copy) — none — status: skipped-judgment-call — Geist of Saint Traft's Angel-token defender mismatch is latent in 1v1 ISD (`state.opponent(controller)` happens to equal Geist's actual defender); the bug only manifests with planeswalker combat or multiplayer, neither of which have working in-engine support today
- Bug AP (Snapshot anthems) — `mtg-engine/tests/audit_snapshot_anthems_family.rs::bug_ap_rally_the_peasants_buffs_creatures_entering_later` — status: failing-as-expected
- Bug BK (Snapshot anthems) — `mtg-engine/tests/audit_snapshot_anthems_family.rs::bug_bk_instigator_gang_anthem_drops_when_source_leaves` — status: failing-as-expected
- Bug AZ (Snapshot anthems) — none — status: blocked — Spare from Evil's `GrantProtection` snapshot has the same shape as Bug AP but for protection rather than P/T. The protection-from-source query path is structurally separate from `effective_power`, and verifying that a creature entering after Spare from Evil resolves picks up the protection requires either casting an opponent's non-Human creature targeting the new creature OR querying `has_protection_from` directly, and the latter is the kind of internal helper test the audit prefers to avoid until the fix introduces a `GlobalGrantProtection` variant
- Bug D (Auto-pick) — `mtg-engine/tests/audit_auto_pick_family.rs::bug_d_moorland_haunt_does_not_auto_pick_creature_to_exile` — status: failing-as-expected
- Bug P (Auto-pick) — `mtg-engine/tests/audit_auto_pick_family.rs::bug_p_caravan_vigil_does_not_auto_pick_basic_land` — status: failing-as-expected
- Bug W (Auto-pick) — `mtg-engine/tests/audit_auto_pick_family.rs::bug_w_legend_rule_pauses_for_player_choice` — status: failing-as-expected
- Bug 76-003 (Auto-pick) — none — status: blocked — Traveler's Amulet is the same shape as Bug P (Caravan Vigil); the test pattern is identical and a fix touching the shared search-basic-land helper would unblock both at once. Recording as blocked rather than duplicating the assertion
- Bug E (Auto-pick) — none — status: blocked — Nevermore's auto-pick from opp's hand requires a `ResolutionChoiceKind::ChooseCardName` variant that doesn't yet exist; the bug is also a separate information-leak shape (the implementation reads opponent hand contents). The fix would introduce a new choice variant and a string-typed resolution; both depend on infrastructure that isn't in tree
- Bug F (Auto-pick) — none — status: blocked — `ExileCreaturesFromGraveyard` auto-picks at apply time, not at `legal_actions` time, so the only way to observe the wrong choice is to set up a scenario where the auto-pick differs from the player's preference and assert against the resulting board state. This requires Skaab Ruinator (3 exiles) or similar where the choice is non-trivial; in practice the audit notes the auto-pick was always the only legal option
- Bug J (Auto-pick) — none — status: blocked — Harvest Pyre's collapsed-X-cost cast options bug lives in `mtg-player/src/llm.rs`'s action collapsing, not in the engine. Tracked under harness bugs
- Bug U (Auto-pick) — none — status: blocked — Kessig Wolf Run's X-cost activated ability has no in-engine X enumeration. Testing the fix requires a particular shape (multiple ActivateAbility entries with different X values, or a follow-up X-prompt). Without knowing the fix shape, the test would either pass trivially (single entry today, single entry post-fix with prompt) or test internal state. Defer until the fix lands
- Bug BF (Auto-pick) — none — status: blocked — Traveler's Amulet "doesn't shuffle" is a deterministic-shuffle observation; testing it requires asserting that the post-search library order is randomized, which is non-deterministic and brittle. Possible to test by seeding RNG, but that's a separate fixture concern
- Bug BT (Trigger dispatch) — `mtg-engine/tests/audit_trigger_dispatch_family.rs::bug_bt_abattoir_ghoul_gains_life_on_simultaneous_death` — status: failing-as-expected
- Bug L (Trigger dispatch) — `mtg-engine/tests/audit_trigger_dispatch_family.rs::bug_l_charmbreaker_devils_does_not_buff_on_creature_spell` — status: failing-as-expected
- Bug CA (Misc) — `mtg-engine/tests/audit_trigger_dispatch_family.rs::bug_ca_moldgraf_monstrosity_uses_controller_not_owner` — status: failing-as-expected
- Bug 76-001 (Harness — labels) — `mtg-engine/tests/audit_misc_final.rs::bug_76_001_skirsdag_high_priest_label_has_no_object_id_debug` — status: failing-as-expected
- Bug 76-002 (Counter display) — `mtg-engine/tests/audit_misc_final.rs::bug_76_002_ludevic_hatchling_counters_not_in_card_state` — status: failing-as-expected
- Bug 99-001 (Counter display) — `mtg-engine/tests/audit_misc_final.rs::bug_99_001_gutter_grime_does_not_count_token_deaths` — status: failing-as-expected
