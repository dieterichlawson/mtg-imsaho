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
