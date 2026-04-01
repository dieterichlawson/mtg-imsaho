# Audit: Inquisitor's Flail

## Oracle (Official)
- **Name:** Inquisitor's Flail
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle:** If equipped creature would deal combat damage, it deals double that damage instead. If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
- **P/T:** N/A

## Implementation
- Name: "Inquisitor's Flail" -- CORRECT
- Cost: {2} -- CORRECT
- Type: Artifact -- CORRECT
- Subtypes: ["Equipment"] -- CORRECT
- Equip {2}, sorcery speed, targets creature you control -- CORRECT
- Oracle text: says "another creature" in code comment but oracle says "another source" -- the oracle_text string in code is correct

## Issues
1. **ISSUE (simplification):** Offensive double damage is approximated by granting +P/+0 equal to creature's effective power via `dynamic_pt`. This is an approximation rather than a true damage replacement effect. The comment acknowledges this.
2. **ISSUE (missing):** Defensive doubling (equipped creature takes double combat damage from other sources) is NOT implemented. Comment acknowledges this.
3. **ISSUE (minor):** The `dynamic_pt` approach means the power bonus is visible outside combat, which could affect other game interactions differently than the real card.

## Verdict: PASS (with noted simplifications)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

Findings:
- Mana cost {2}: correct.
- Types Artifact, subtypes Equipment: correct.
- P/T N/A: correct.
- Equip {2} activated ability, sorcery_speed_only: true, targets creature: correct.
- on_resolve uses `move_object(object_id, Zone::Battlefield)` -- this is acceptable for a permanent (artifact), not a spell anti-pattern issue.
- on_activate_ability attaches via `obj.attached_to = Some(*creature_id)`: correct.
- continuous_effects: `DoubleCombatDamage { scope: EffectScope::Attached }`: only models offensive damage doubling.
- ISSUE 1 (carried forward): Defensive doubling (incoming combat damage to equipped creature is doubled) is NOT implemented. The continuous_effects vec only has one entry for outgoing damage. The oracle has two separate replacement effects.
- ISSUE 2 (carried forward): The offensive doubling is implemented as a continuous effect rather than a damage replacement effect. Depending on engine implementation of DoubleCombatDamage, this may or may not be accurate.
- ISSUE 3 (Scryfall discrepancy): Scryfall oracle says "another creature" for the defensive clause. The code's oracle_text string says "another source". The actual current Scryfall oracle text says "another creature" -- the code's oracle_text string is incorrect.
- No CombatDamageDealt misuse (the card modifies damage, does not deal it).
- No triggered_abilities declared, none needed: correct.
- Tests found in tier9_cards.rs and inquisitors_flail.rs.
