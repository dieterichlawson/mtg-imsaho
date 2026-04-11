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
