# Remaining Bugs to Write Failing Tests For

## Current test file: mtg-engine/tests/audit_bugs.rs

## 4 New Engine Patterns

### E1. Spurious trigger firing (4 issues)
- charmbreaker_devils: Upkeep trigger fires during opponent's upkeep
- charmbreaker_devils: SpellCast trigger fires for opponent's spells
- civilized_scholar: EndStep trigger on front face causes spurious stack entry when not transformed
- cloistered_youth: Spurious upkeep trigger for transformed state (Unholy Fiend)
- curse_of_oblivion: Upkeep trigger fires during every player's upkeep
- curse_of_the_pierced_heart: upkeep trigger fires every player's upkeep
- delver_of_secrets: transformed face gets spurious upkeep trigger

### E2. Hexproof not re-checked at resolution (3 issues)
- sensory_deprivation: Engine does not check hexproof at resolution
- sever_the_bloodline: hexproof not re-checked at resolution
- witchbane_orb: hexproof not re-validated at resolution for player targets

### E3. Targeting at resolution not cast time (3 issues)
- geistcatchers_rig: target selection deferred to resolution
- unburial_rites: missing target_requirement, target at resolution not cast

### E4. Zone change state reset (2 issues)
- ludevics_test_subject: card_state not reset on zone change
- ludevics_test_subject: is_transformed not reset on zone change

## 3 Architectural Issues
- elite_inquisitor: protection targeting not enforced (can_be_targeted lacks source)
- grave_bramble: Grimgrin can target despite protection from Zombies
- angelic_overseer: Sequential SBA processing — Human dies before Overseer checked

## 51 Card-Specific Bugs

1. balefire_dragon: trigger suppressed if leaves battlefield before resolution
2. bitterheart_witch: hexproof not filtered when building target list
3. bloodgift_demon: trigger fizzles if leaves battlefield
4. boneyard_wurm: view.rs shows base P/T not dynamic
5. burning_vengeance: SpellCast dispatch restricted to instant/sorcery
6. burning_vengeance: checks cast_with_flashback not "cast from graveyard"
7. civilized_scholar: stale attacked_this_turn flag
8. creepy_doll: lethal damage + regeneration + coin flip interaction
9. curse_of_the_nightly_hunt: forced-attack no can_attack check
10. demonmail_hauberk: engine checks any creature exists not specific sacrifice choice
11. essence_of_the_wild: non-on_resolve path no replacement effect
12. evil_twin: is_evil_twin marker set before copy choice
13. furor_of_the_bitten: ForceAttack + PreventAttack conflict
14. galvanic_juggernaut: force attack ignores can_attack
15. geistcatchers_rig: optional conflates target with "you may"
16. grimoire_of_the_dead: legend rule not applied to returned creatures
17. grizzled_outcasts: log wrong name on back-to-front transform
18. hamlet_captain: trigger doesn't resolve if leaves battlefield
19. harvest_pyre: player cannot choose which cards to exile
20. hinterland_harbor: subtypes in CardData not detected by checkland
21. into_the_maw_of_hell: is_valid_target accepts creatures for land slot
22. liliana_of_the_veil: discards resolve sequentially not simultaneously
23. mentor_of_the_meek: auto-pays without choice
24. mirror_mad_phantasm: draw_top_card sets has_drawn_from_empty incorrectly
25. nevermore: ban not enforced for flashback casts
26. night_terrors: never moved off stack with multiple nonland cards
27. night_terrors: wrong PendingEffect variant
28. past_in_flames: no-cost cards get free flashback
29. prey_upon: uses CombatDamageDealt instead of NonCombatDamageDealt
30. reaper_from_the_abyss: intervening-if not enforced at trigger collection
31. rooftop_storm: alternative cost not offered from graveyard
32. skirsdag_high_priest: tap two creatures is player choice not auto-selected
33. smite_the_monstrous: power condition not re-checked at resolution
34. stitchers_apprentice: trigger_event_index desync
35. sturmgeist: draw skipped when leaves battlefield before trigger
36. thraben_sentry: auto-transforms without "you may" choice
37. thraben_sentry: vigilance retained on back face after transform
38. tribute_to_hunger: missing target opponent restriction
39. unbreathing_horde: enters-with-counters not applied via reanimation
40. unburial_rites: can cast with no legal targets
41. undead_alchemist: multiple alchemists cause double milling
42. undead_alchemist: lifelink incorrectly grants life with replacement
43. woodland_sleuth: can return itself if dies in response to own trigger

## Progress
- Done: E1 (spurious: FP), E2 (hexproof), E3 (partially via unburial_rites), E4 (zone reset)
- Done: #5-6, #20, #24, #25, #29, #36-38, #39, #40
- Done from architectural: protection targeting (tested via code inspection, not unit test)
- Next: #1 balefire_dragon, #7 civilized_scholar, #9 curse_nightly_hunt, #16 grimoire_of_dead, #21 into_maw, #22 liliana, #26-27 night_terrors, #28 past_in_flames, #30 reaper, #31 rooftop_storm, #32 skirsdag_hp, #33 smite, #41-42 undead_alchemist, #43 woodland_sleuth
