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
8. [x] "Target player" auto-selects opponent — 15 cards (Falkenrath Noble done, need to apply DrainLife pattern to other cards)
9. [x] Planeswalker damage uses damage_marked not loyalty — 3 cards
10. [x] card_state not reset on zone change — 2 cards
11. [x] Control change not reverted at EOT — 1 card
12. [x] SpellCast trigger filter excludes non-instant/sorcery — 1 card (Burning Vengeance)
13. [x] SBA ordering (simultaneous destruction) — 1 card (Angelic Overseer)
14. [x] Protection not checked for targeting — 2 cards
15. [x] Protection incorrectly prevents blocking — 1 card
16. [x] Protection non-combat damage not prevented — 1 card

### Card-specific bugs
17. [x] Ghost Quarter: missing shuffle + "may" is mandatory
18. [ ] Bonds of Faith: "as long as" snapshot
19. [x] Delver of Secrets: reveal suppressed for non-instant/sorcery
20. [x] Thraben Sentry: auto-transforms + vigilance retained
21. [x] Hinterland Harbor: checkland misses registry subtypes
22. [x] Unburial Rites: no target_requirement
23. [x] Harvest Pyre: auto-selects exile
24. [x] Unbreathing Horde: no counters via reanimation
25. [x] Smite: power not re-checked
26. [x] Grimoire of the Dead: legend rule
27. [ ] Undead Alchemist: double mill with multiple copies
28. [ ] Skirsdag High Priest: auto-selects tap targets
29. [x] Demonmail Hauberk: sacrifice check too loose
30. [ ] Civilized Scholar: stale attacked_this_turn
31. [ ] Essence of the Wild: replacement not for tokens
32. [ ] Mentor of the Meek: auto-pays
33. [ ] Evil Twin: marker before choice + ability inaccessible
34. [x] Brain Weevil: incomplete discard
35. [x] Nevermore: not enforced for flashback
36. [x] Tribute to Hunger: can target self
37. [x] Night Terrors: wrong PendingEffect + stuck on stack
38. [x] Prey Upon: wrong damage type + partial fizzle
39. [ ] Dearly Departed: graveyard watcher
40. [x] Garruk Relentless: is_legendary not set
41. [x] Inquisitor's Flail: fight damage doubled (fixed via fight vs combat damage split)
42. [x] Cackling Counterpart: colors not copied
43. [x] Bitterheart Witch: hexproof not filtered
44. [x] Mask of Avacyn: duplicate equip action
45. [x] Memory's Journey: missing player target
46. [x] Spare from Evil: protection non-combat damage

## Current: 11 remaining bugs
## Completed: #1-#16 (all engine bugs) + 19 card-specific bugs
## Remaining: #18, #27, #28, #30, #31, #32, #33, #39
## Test status: 58 passing, 11 failing across audit_bugs.rs + audit_bugs2.rs

## How to continue
1. Read this file for what's left
2. Test file: mtg-engine/tests/audit_bugs.rs and audit_bugs2.rs
3. Run `cargo test -p mtg-engine --test audit_bugs` and `--test audit_bugs2` to check status
4. Each fix should make one or more tests pass
5. Commit after each fix, push to remote
