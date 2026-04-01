## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
**Scryfall type line**: Instant
**Scryfall mana cost**: {B}{B}
**Status**: PASS

Findings:
- Name: Correct.
- Mana cost: {B}{B} — correct.
- Type: Instant — correct.
- Oracle text: Matches.
- Targeting: `is_valid_target` checks the target is a creature on the battlefield and does NOT have Vampire, Werewolf, or Zombie subtypes. Correct.
- Target requirement: `NotSubtypes(["Vampire", "Werewolf", "Zombie"])`. Correct.
- Resolution: Uses `resolve_destroy` helper. Correct.
- Tests: `victim_of_night_kills_normal_creature` and `victim_of_night_cant_target_vampire` in tier2_spells.rs.

No issues found.
