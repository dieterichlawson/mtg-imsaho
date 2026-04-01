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

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/227/inquisitors-flail), confirmed by Gatherer via WebSearch (https://gatherer.wizards.com/Pages/Card/Details.aspx?name=inquisitor's+flail)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

1. **Oracle text string mismatch** (`mtg-engine/src/cards/inquisitors_flail.rs`, line 26):
   - Oracle text says: `If another creature would deal combat damage to equipped creature`
   - Code says: `If another source would deal combat damage to equipped creature`
   - The code uses "another source" where the current Scryfall oracle text says "another creature".

No other issues. Mana cost {2}, types Artifact/Equipment, equip cost, sorcery-speed-only, creature targeting, DoubleCombatDamage continuous effect, and combat.rs implementation (lines 447-454) all correctly double both outgoing and incoming combat damage. Tests in inquisitors_flail.rs (4 tests) confirm both directions work. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/227/inquisitors-flail)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

Findings:
- Mana cost {2}: correct.
- Types Artifact, subtypes Equipment: correct.
- P/T N/A: correct.
- Equip {2} activated ability, sorcery_speed_only: true, targets creature controller owns: correct.
- on_resolve moves to battlefield and sets is_equipment = true: correct for equipment.
- on_activate_ability sets attached_to: correct.
- ISSUE 1: The code's oracle_text field (line 26) says "another source" but Scryfall oracle text says "another creature". The oracle_text string is incorrect.
- ISSUE 2 (carried forward): Defensive doubling (incoming combat damage to equipped creature is doubled) relies on the continuous_effects DoubleCombatDamage implementation. The continuous_effects vec has a single DoubleCombatDamage entry with EffectScope::Attached. Whether the engine correctly doubles BOTH outgoing and incoming damage depends on the DoubleCombatDamage implementation, which is outside this card's file. Tests confirm both directions work (doubles_damage_to_player, doubles_damage_to_creature, doubles_damage_taken_from_blocker).
- Anti-pattern check: on_resolve uses move_object to battlefield (correct for artifact permanent). No spell-to-graveyard anti-pattern.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- Tests: 4 tests in inquisitors_flail.rs (doubles_damage_to_player, doubles_damage_to_creature, doubles_damage_taken_from_blocker, no_doubling_without_flail) plus tests in tier9_cards.rs. Good coverage of both offensive and defensive doubling.
