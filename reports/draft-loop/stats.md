# Draft loop: what the nights add up to

37 night(s), 296 decks, 1097 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 104 | 151-161-0 | 48% | 372-394 | 49% |
| conventional | 36 | 58-50-0 | 54% | 139-120 | 54% |
| unguided | 156 | 235-233-0 | 50% | 586-583 | 50% |

### The nights with all three (18)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 36 | 50-58-0 | 46% | 122-140 | 47% |
| conventional | 36 | 58-50-0 | 54% | 139-120 | 54% |
| unguided | 72 | 108-108-0 | 50% | 270-271 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BR | 39 | 63-54-0 | 51% (289) |
| UB | 38 | 42-72-0 | 43% (288) |
| WR | 35 | 55-50-0 | 52% (254) |
| WU | 35 | 57-48-0 | 53% (261) |
| WG | 33 | 55-44-0 | 53% (242) |
| BG | 31 | 51-42-0 | 54% (225) |
| UR | 25 | 34-41-0 | 46% (185) |
| WB | 24 | 35-37-0 | 47% (177) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 13 | 16-23-0 | 46% (98) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 2 | 3-3-0 | 47% (15) |

## Cards in winning decks (at least 74 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Slayer of the Wicked | W | uncommon | 37 | 267 | 60% | 1.7 | 43/75 |
| Sever the Bloodline | B | rare | 13 | 92 | 60% | 1.2 | 16/20 |
| Midnight Haunting | W | uncommon | 35 | 258 | 58% | 3.5 | 42/145 |
| Mentor of the Meek | W | rare | 15 | 112 | 58% | 3.7 | 16/59 |
| Desperate Ravings | R | uncommon | 11 | 82 | 57% | 9.8 | 46/451 |
| Falkenrath Marauders | R | rare | 11 | 79 | 57% | 2.1 | 13/27 |
| Kruin Outlaw | R | rare | 15 | 111 | 57% | 2.4 | 18/44 |
| Stromkirk Noble | R | rare | 15 | 111 | 57% | 4.7 | 16/75 |
| Disciple of Griselbrand | B | uncommon | 22 | 161 | 57% | 9.2 | 50/458 |
| Elite Inquisitor | W | rare | 15 | 111 | 56% | 2.6 | 17/45 |
| Furor of the Bitten | R | common | 29 | 211 | 55% | 9.9 | 87/848 |
| Angel of Flight Alabaster | W | rare | 10 | 74 | 55% | 1.8 | 13/24 |
| Villagers of Estwald | G | common | 46 | 342 | 55% | 4.5 | 94/423 |
| Into the Maw of Hell | R | uncommon | 38 | 286 | 55% | 3.3 | 48/156 |
| Bonds of Faith | W | common | 59 | 437 | 55% | 3.9 | 76/295 |
| Ashmouth Hound | R | common | 51 | 377 | 55% | 3.7 | 78/289 |
| Avacynian Priest | W | common | 63 | 462 | 55% | 5.6 | 79/442 |
| Avacyn's Pilgrim | G | common | 26 | 192 | 55% | 8.8 | 82/719 |
| Galvanic Juggernaut | C | uncommon | 46 | 347 | 54% | 1.2 | 46/55 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.2 | 38/234 |
| Rage Thrower | R | uncommon | 27 | 199 | 54% | 9.1 | 57/521 |
| Lumberknot | G | uncommon | 33 | 245 | 54% | 4.4 | 41/179 |
| Thraben Sentry | W | common | 41 | 304 | 54% | 7.7 | 69/529 |
| Voiceless Spirit | W | common | 66 | 484 | 54% | 3.5 | 79/275 |
| Riot Devils | R | common | 47 | 352 | 54% | 7.8 | 80/628 |
| Gatstaf Shepherd | G | uncommon | 37 | 275 | 53% | 5.1 | 54/275 |
| Spider Spawning | G | uncommon | 26 | 191 | 53% | 4.1 | 45/186 |
| Unruly Mob | W | common | 52 | 377 | 53% | 9.1 | 77/698 |
| Woodland Sleuth | G | common | 45 | 330 | 53% | 6.5 | 82/535 |
| Doomed Traveler | W | common | 59 | 432 | 53% | 6.8 | 81/553 |
| Manor Gargoyle | C | rare | 19 | 141 | 53% | 2.3 | 19/43 |
| Pitchburn Devils | R | common | 59 | 439 | 53% | 4.8 | 74/355 |
| Bloodgift Demon | B | rare | 12 | 87 | 53% | 1.1 | 12/13 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Kindercatch | G | common | 51 | 381 | 53% | 7.4 | 77/573 |
| Morkrut Banshee | B | uncommon | 33 | 248 | 53% | 1.8 | 43/76 |
| Cloistered Youth | W | uncommon | 24 | 173 | 53% | 5.8 | 40/234 |
| Kessig Wolf | R | common | 57 | 425 | 52% | 7.6 | 82/617 |
| Rally the Peasants | W | uncommon | 35 | 252 | 52% | 6.7 | 57/384 |
| Crossway Vampire | R | common | 53 | 394 | 52% | 5.4 | 77/414 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 13 | 9 |
| Bloodgift Demon | B | rare | 1.1 | 12 | 12 |
| Olivia Voldaren | BR | mythic | 1.1 | 7 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 46 | 46 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Mikaeus, the Lunarch | W | mythic | 1.2 | 6 | 4 |
| Sever the Bloodline | B | rare | 1.2 | 16 | 13 |
| Reaper from the Abyss | B | mythic | 1.2 | 5 | 5 |
| Balefire Dragon | R | mythic | 1.3 | 7 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 48 | 38 |
| Devil's Play | R | rare | 1.5 | 13 | 9 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 7 | 5 |
| Geist-Honored Monk | W | rare | 1.6 | 12 | 8 |
| Victim of Night | B | common | 1.6 | 78 | 54 |
| Slayer of the Wicked | W | uncommon | 1.7 | 43 | 37 |
| Dead Weight | B | common | 1.7 | 82 | 54 |
| Morkrut Banshee | B | uncommon | 1.8 | 43 | 33 |
| Angel of Flight Alabaster | W | rare | 1.8 | 13 | 10 |
| Chapel Geist | W | common | 1.9 | 64 | 46 |
| Falkenrath Noble | B | uncommon | 1.9 | 44 | 34 |
| Claustrophobia | U | common | 2.0 | 79 | 55 |
| Brimstone Volley | R | common | 2.0 | 75 | 50 |
| Skaab Ruinator | U | mythic | 2.0 | 8 | 6 |
| Falkenrath Marauders | R | rare | 2.1 | 13 | 11 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Murder of Crows | U | uncommon | 2.2 | 42 | 34 |
| Essence of the Wild | G | mythic | 2.2 | 6 | 4 |
| Tribute to Hunger | B | uncommon | 2.3 | 40 | 33 |
| Manor Gargoyle | C | rare | 2.3 | 19 | 19 |
