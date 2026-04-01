## Audit — 2026-04-01

**Scryfall Oracle text**: Equipped creature gets +1/+0.\nWhenever equipped creature blocks or becomes blocked by a Vampire, destroy that Vampire.\nEquip {1}
**Scryfall type line**: Artifact — Equipment
**Scryfall mana cost**: {2}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {2} — correct.
- Types: Artifact — Equipment — correct.
- P/T buff: +1/+0 via `ContinuousEffect::ModifyPT` with `EffectScope::Attached`. Correct.
- Vampire destruction trigger: Implemented in both `on_blocks` and `on_becomes_blocked`. Checks both registry subtypes and instance subtypes for "Vampire". Uses `try_destroy_no_regen` which is slightly more aggressive than normal destroy but reasonable for flavor.
- Equip {1}: Sorcery speed, targets a creature you control. Correct.
- Equip targeting: Correctly limits to own creatures (`o.controller == caster`). Correct.
- Tests: `wooden_stake_has_correct_data`, `wooden_stake_grants_power`, `wooden_stake_destroys_vampire_on_block`, `wooden_stake_does_not_destroy_non_vampire` in tier9_equipment.rs.

No issues found.
