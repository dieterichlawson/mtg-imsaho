## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Scryfall type line**: Legendary Creature — Vampire
**Status**: ISSUE

- Name: Correct ("Olivia Voldaren")
- Cost: {2}{B}{R} - Correct
- Type: Legendary Creature — Vampire - Correct (supertypes: [Legendary])
- P/T: 3/3 - Correct
- Keywords: Flying - Correct
- Ability 0 ({1}{R}): Deals 1 damage to another target creature, makes it a Vampire, puts +1/+1 counter on Olivia. Implementation checks `target_id == object_id` to enforce "another". Correct.

Issues:
1. **Ability 1 target filter incorrect**: The second ability ({3}{B}{B}: Gain control of target Vampire) uses `TargetFilter::Any` instead of filtering for Vampires only. While the on_activate_ability checks for Vampire subtype before executing, the ability should only allow targeting Vampires in the first place. A player could waste mana activating it on a non-Vampire and get no effect.
2. **Control effect duration**: Oracle says "for as long as you control Olivia Voldaren" but the implementation grants permanent control change without tracking that Olivia must remain on the battlefield. If Olivia leaves, the control should revert.
3. **Oracle text mismatch**: The implementation's oracle_text says "Gain control of target Vampire." but Oracle text says "Gain control of target Vampire for as long as you control Olivia Voldaren."

- Tests: No dedicated Olivia Voldaren test file found.

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Scryfall type line**: Legendary Creature — Vampire
**Status**: ISSUE

1. Ability 0 targeting uses TargetFilter::Any which allows targeting Olivia herself, but the Oracle text says "another target creature". The code does check for self-targeting in on_activate_ability (line 95: `if *target_id == object_id { return; }`), but the target filter should exclude self to prevent the player from selecting an invalid target. (File: /home/user/mtg-imsaho/mtg-engine/src/cards/olivia_voldaren.rs, line 52)
2. Ability 1 target filter uses TargetFilter::Any instead of filtering for Vampires. The Vampire check only happens at resolution time (line 124-127). Should filter for Vampires at targeting time. (File: /home/user/mtg-imsaho/mtg-engine/src/cards/olivia_voldaren.rs, line 68)
3. Ability 1 Oracle says "Gain control of target Vampire for as long as you control Olivia Voldaren" but the control change is permanent — it doesn't revert if Olivia leaves the battlefield. (File: /home/user/mtg-imsaho/mtg-engine/src/cards/olivia_voldaren.rs, line 129)
