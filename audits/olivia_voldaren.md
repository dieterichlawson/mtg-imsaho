# Audit: Olivia Voldaren

## Official Oracle
- **Name:** Olivia Voldaren
- **Cost:** {2}{B}{R}
- **Type:** Legendary Creature — Vampire
- **Oracle:** Flying. {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
- **P/T:** 3/3

## Implementation: `mtg-engine/src/cards/olivia_voldaren.rs`
- **Name:** Olivia Voldaren -- CORRECT
- **Cost:** {2}{B}{R} -- CORRECT
- **Type:** Creature, Legendary -- CORRECT
- **Subtypes:** Vampire -- CORRECT
- **P/T:** 3/3 -- CORRECT
- **Keywords:** Flying -- CORRECT

### Ability 0: {1}{R} ping
- **Cost:** {1}{R} -- CORRECT
- **Targets:** Another creature -- CORRECT (enforced in on_activate_ability with self-check)
- **Effect:** 1 damage, makes Vampire, +1/+1 counter on Olivia -- CORRECT
- **NonCombatDamageDealt event:** Emitted -- CORRECT
- **damaged_by tracking:** Added -- CORRECT

### Ability 1: {3}{B}{B} steal
- **Cost:** {3}{B}{B} -- CORRECT
- **Target:** Vampire creature -- CORRECT (checked in on_activate_ability)

## Issues
1. **Control duration missing:** Oracle says "Gain control of target Vampire **for as long as you control Olivia Voldaren**." The implementation changes controller permanently without the "for as long as" condition. If Olivia leaves the battlefield, the stolen creature should revert to its original controller.
2. **Ability 1 target filter too broad:** The activated ability definition uses `TargetFilter::Any` for ability 1 but should filter to Vampires only. The Vampire check is only in on_activate_ability, which means the AI may try to target non-Vampires and waste the activation.

## Verdict
**FAIL** -- 2 issues: (1) Steal effect should end when Olivia leaves; (2) Ability 1 target filter should be Vampire-only.
