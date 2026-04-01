## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature gets +2/+0.\nAs long as equipped creature is a Human, it gets an additional +1/+0.\nEquip {2}
**Scryfall type line**: Artifact — Equipment
**Mana cost**: {1}
**Status**: ISSUE

**Issue: Oracle text in implementation says "+3/+0 instead" but actual Oracle says "+2/+0" base and "an additional +1/+0" for Humans (total +3/+0 for Humans).**

The implementation oracle_text says "gets +2/+0. As long as equipped creature is a Human, it gets +3/+0 instead." while the actual Oracle text says "gets +2/+0" and "As long as equipped creature is a Human, it gets an additional +1/+0." The functional result is the same (+2 for non-Humans, +3 for Humans), so the behavior is correct, but the oracle_text string is a paraphrase rather than exact.

The `update_effects` method correctly applies +2/+0 for non-Humans and +3/+0 for Humans.

- Tests: 3 tests in tier9_equipment.rs covering data, non-Human +2, and Human +3

Functional behavior is correct; oracle text wording is a minor paraphrase.
