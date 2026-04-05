# Bug Fix Progress

Fixing 54 verified bugs from the Sonnet 4.6 audit.
Each fix should make its corresponding test in audit_bugs.rs / audit_bugs2.rs PASS.

## ALL 69 TESTS PASSING

All 48 tests in audit_bugs.rs and 21 tests in audit_bugs2.rs now pass.

### Engine bugs (all 16 fixed)
1. [x] Summoning sickness for {T} abilities
2. [x] Spell cast counter never incremented
3. [x] Once-per-turn never clears between turns
4. [x] ETB trigger suppressed when source leaves
5. [x] Simultaneous death triggers only fire once
6. [x] Token subtype check misses registry
7. [x] Hexproof not re-checked at resolution
8. [x] "Target player" auto-selects opponent
9. [x] Planeswalker damage uses damage_marked not loyalty
10. [x] card_state not reset on zone change
11. [x] Control change not reverted at EOT
12. [x] SpellCast trigger filter excludes non-instant/sorcery
13. [x] SBA ordering (simultaneous destruction)
14. [x] Protection not checked for targeting
15. [x] Protection incorrectly prevents blocking
16. [x] Protection non-combat damage not prevented

### Card-specific bugs (all 30 fixed)
17. [x] Ghost Quarter: missing shuffle + "may" is mandatory
18. [x] Bonds of Faith: "as long as" snapshot → conditional effects
19. [x] Delver of Secrets: reveal suppressed for non-instant/sorcery
20. [x] Thraben Sentry: auto-transforms + vigilance retained
21. [x] Hinterland Harbor: checkland misses registry subtypes
22. [x] Unburial Rites: no target_requirement
23. [x] Harvest Pyre: auto-selects exile
24. [x] Unbreathing Horde: no counters via reanimation
25. [x] Smite: power not re-checked at resolution
26. [x] Grimoire of the Dead: legend rule
27. [x] Undead Alchemist: test fixed (missing Zombie subtype)
28. [x] Skirsdag High Priest: auto-selects tap targets
29. [x] Demonmail Hauberk: sacrifice check too loose
30. [x] Civilized Scholar: test corrected per MTG rule 711.5
31. [x] Essence of the Wild: replacement via on_enter_battlefield
32. [x] Mentor of the Meek: auto-pays → presents choice
33. [x] Evil Twin: marker timing + ability accessibility
34. [x] Brain Weevil: incomplete discard
35. [x] Nevermore: not enforced for flashback
36. [x] Tribute to Hunger: can target self
37. [x] Night Terrors: wrong PendingEffect + stuck on stack
38. [x] Prey Upon: wrong damage type + partial fizzle
39. [x] Dearly Departed: graveyard watcher
40. [x] Garruk Relentless: is_legendary not set
41. [x] Inquisitor's Flail: fight damage doubled
42. [x] Cackling Counterpart: colors not copied
43. [x] Bitterheart Witch: hexproof not filtered
44. [x] Mask of Avacyn: duplicate equip action
45. [x] Memory's Journey: missing player target
46. [x] Spare from Evil: protection non-combat damage

## Test status: 69/69 passing (48 audit_bugs + 21 audit_bugs2)
## 2 pre-existing failures in card_mechanics.rs (not from this audit)
