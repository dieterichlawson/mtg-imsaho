# Bug Fix Progress

Fixing 54 verified bugs from the Sonnet 4.6 audit.
Each fix should make its corresponding test in audit_bugs.rs / audit_bugs2.rs PASS.

## Priority order (by impact / number of affected cards)

### Engine bugs (fix once, fixes many cards)
1. [x] Summoning sickness for {T} abilities (engine.rs:356) — 3 cards
2. [x] Spell cast counter never incremented — all werewolves
3. [x] Once-per-turn never clears between turns — 3 cards
4. [x] ETB trigger suppressed when source leaves (triggers.rs:893) — 11 cards
5. [x] Simultaneous death triggers only fire once — 9 cards
6. [x] Token subtype check misses registry — Victim of Night fixed (17 others need same pattern)
7. [x] Hexproof not re-checked at resolution (stack.rs) — 3 cards
8. [ ] "Target player" auto-selects opponent — 15 cards
9. [ ] Planeswalker damage uses damage_marked not loyalty — 3 cards
10. [ ] card_state not reset on zone change — 2 cards
11. [ ] Control change not reverted at EOT — 1 card
12. [ ] SpellCast trigger filter excludes non-instant/sorcery — 1 card (Burning Vengeance)
13. [ ] SBA ordering (simultaneous destruction) — 1 card (Angelic Overseer)
14. [ ] Protection not checked for targeting — 2 cards
15. [ ] Protection incorrectly prevents blocking — 1 card
16. [ ] Protection non-combat damage not prevented — 1 card

### Card-specific bugs
17. [ ] Ghost Quarter: missing shuffle + "may" is mandatory
18. [ ] Bonds of Faith: "as long as" snapshot
19. [ ] Delver of Secrets: reveal suppressed for non-instant/sorcery
20. [ ] Thraben Sentry: auto-transforms + vigilance retained
21. [ ] Hinterland Harbor: checkland misses registry subtypes
22. [ ] Unburial Rites: no target_requirement
23. [ ] Harvest Pyre: auto-selects exile
24. [ ] Unbreathing Horde: no counters via reanimation
25. [ ] Smite: power not re-checked
26. [ ] Grimoire of the Dead: legend rule
27. [ ] Undead Alchemist: double mill with multiple copies
28. [ ] Skirsdag High Priest: auto-selects tap targets
29. [ ] Demonmail Hauberk: sacrifice check too loose
30. [ ] Civilized Scholar: stale attacked_this_turn
31. [ ] Essence of the Wild: replacement not for tokens
32. [ ] Mentor of the Meek: auto-pays
33. [ ] Evil Twin: marker before choice + ability inaccessible
34. [ ] Brain Weevil: incomplete discard
35. [ ] Nevermore: not enforced for flashback
36. [ ] Tribute to Hunger: can target self
37. [ ] Night Terrors: wrong PendingEffect + stuck on stack
38. [ ] Prey Upon: wrong damage type + partial fizzle
39. [ ] Dearly Departed: graveyard watcher
40. [ ] Garruk Relentless: is_legendary not set
41. [ ] Inquisitor's Flail: fight damage doubled
42. [ ] Cackling Counterpart: colors not copied
43. [ ] Bitterheart Witch: hexproof not filtered
44. [ ] Mask of Avacyn: duplicate equip action
45. [ ] Memory's Journey: missing player target
46. [ ] Spare from Evil: protection non-combat damage

## Current: Working on #5 (simultaneous death triggers)
## Completed: #1, #2, #3, #4
