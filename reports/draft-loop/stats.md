# Draft loop: what the nights add up to

38 night(s), 304 decks, 1127 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 106 | 153-165-0 | 48% | 378-402 | 48% |
| conventional | 38 | 62-52-0 | 54% | 149-125 | 54% |
| unguided | 160 | 241-239-0 | 50% | 600-600 | 50% |

### The nights with all three (19)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 38 | 52-62-0 | 46% | 128-148 | 46% |
| conventional | 38 | 62-52-0 | 54% | 149-125 | 54% |
| unguided | 76 | 114-114-0 | 50% | 284-288 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BR | 40 | 65-55-0 | 51% (298) |
| UB | 40 | 45-75-0 | 44% (303) |
| WU | 36 | 59-49-0 | 54% (269) |
| WG | 35 | 57-48-0 | 52% (256) |
| WR | 35 | 55-50-0 | 52% (254) |
| BG | 31 | 51-42-0 | 54% (225) |
| UR | 26 | 34-44-0 | 45% (191) |
| WB | 24 | 35-37-0 | 47% (177) |
| RG | 15 | 27-18-0 | 56% (116) |
| UG | 13 | 16-23-0 | 46% (98) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 2 | 3-3-0 | 47% (15) |

## Cards in winning decks (at least 76 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Slayer of the Wicked | W | uncommon | 38 | 275 | 60% | 1.8 | 44/77 |
| Sever the Bloodline | B | rare | 13 | 92 | 60% | 1.2 | 16/20 |
| Midnight Haunting | W | uncommon | 36 | 266 | 58% | 3.4 | 44/149 |
| Mentor of the Meek | W | rare | 15 | 112 | 58% | 3.7 | 16/59 |
| Desperate Ravings | R | uncommon | 11 | 82 | 57% | 9.9 | 48/474 |
| Kruin Outlaw | R | rare | 15 | 111 | 57% | 2.4 | 18/44 |
| Stromkirk Noble | R | rare | 15 | 111 | 57% | 4.7 | 16/75 |
| Disciple of Griselbrand | B | uncommon | 22 | 161 | 57% | 9.2 | 51/470 |
| Elite Inquisitor | W | rare | 15 | 111 | 56% | 2.6 | 17/45 |
| Into the Maw of Hell | R | uncommon | 39 | 294 | 56% | 3.3 | 49/161 |
| Furor of the Bitten | R | common | 29 | 211 | 55% | 9.9 | 89/867 |
| Villagers of Estwald | G | common | 49 | 364 | 55% | 4.5 | 99/445 |
| Bonds of Faith | W | common | 61 | 451 | 55% | 3.9 | 78/301 |
| Avacynian Priest | W | common | 64 | 468 | 54% | 5.6 | 81/451 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.2 | 38/234 |
| Rage Thrower | R | uncommon | 27 | 199 | 54% | 9.2 | 58/531 |
| Thraben Sentry | W | common | 41 | 304 | 54% | 7.7 | 69/529 |
| Bloodgift Demon | B | rare | 13 | 94 | 54% | 1.1 | 13/14 |
| Lumberknot | G | uncommon | 34 | 253 | 54% | 4.3 | 42/182 |
| Ashmouth Hound | R | common | 53 | 392 | 54% | 3.7 | 80/293 |
| Galvanic Juggernaut | C | uncommon | 47 | 355 | 54% | 1.2 | 47/56 |
| Avacyn's Pilgrim | G | common | 28 | 206 | 54% | 8.7 | 84/732 |
| Voiceless Spirit | W | common | 68 | 498 | 54% | 3.5 | 81/281 |
| Woodland Sleuth | G | common | 47 | 344 | 53% | 6.5 | 84/547 |
| Spider Spawning | G | uncommon | 26 | 191 | 53% | 4.2 | 46/194 |
| Doomed Traveler | W | common | 61 | 448 | 53% | 6.9 | 83/569 |
| Riot Devils | R | common | 50 | 375 | 53% | 7.8 | 83/644 |
| Unruly Mob | W | common | 54 | 391 | 53% | 9.0 | 79/711 |
| Manor Gargoyle | C | rare | 19 | 141 | 53% | 2.3 | 19/43 |
| Gatstaf Shepherd | G | uncommon | 38 | 281 | 53% | 5.2 | 56/290 |
| Falkenrath Marauders | R | rare | 12 | 85 | 53% | 2.1 | 14/30 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Grave Bramble | G | common | 38 | 284 | 53% | 8.8 | 82/718 |
| Morkrut Banshee | B | uncommon | 33 | 248 | 53% | 1.8 | 43/76 |
| Hamlet Captain | G | uncommon | 34 | 256 | 53% | 7.3 | 45/328 |
| Kindercatch | G | common | 52 | 389 | 53% | 7.5 | 79/592 |
| Hollowhenge Scavenger | G | uncommon | 25 | 186 | 53% | 4.4 | 38/166 |
| Cloistered Youth | W | uncommon | 24 | 173 | 53% | 5.8 | 40/234 |
| Tormented Pariah | R | common | 60 | 443 | 52% | 7.0 | 107/744 |
| Crossway Vampire | R | common | 54 | 403 | 52% | 5.3 | 79/422 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Angelic Overseer | W | mythic | 1.1 | 7 | 5 |
| Bloodline Keeper | B | rare | 1.1 | 13 | 9 |
| Bloodgift Demon | B | rare | 1.1 | 13 | 13 |
| Olivia Voldaren | BR | mythic | 1.1 | 7 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 47 | 47 |
| Mikaeus, the Lunarch | W | mythic | 1.2 | 6 | 4 |
| Sever the Bloodline | B | rare | 1.2 | 16 | 13 |
| Reaper from the Abyss | B | mythic | 1.2 | 6 | 6 |
| Balefire Dragon | R | mythic | 1.2 | 8 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 49 | 39 |
| Geist-Honored Monk | W | rare | 1.5 | 13 | 9 |
| Devil's Play | R | rare | 1.5 | 13 | 9 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 7 | 5 |
| Victim of Night | B | common | 1.6 | 81 | 57 |
| Slayer of the Wicked | W | uncommon | 1.8 | 44 | 38 |
| Falkenrath Noble | B | uncommon | 1.8 | 46 | 35 |
| Dead Weight | B | common | 1.8 | 84 | 55 |
| Morkrut Banshee | B | uncommon | 1.8 | 43 | 33 |
| Angel of Flight Alabaster | W | rare | 1.8 | 13 | 10 |
| Brimstone Volley | R | common | 1.9 | 78 | 52 |
| Claustrophobia | U | common | 2.0 | 81 | 57 |
| Chapel Geist | W | common | 2.0 | 67 | 48 |
| Skaab Ruinator | U | mythic | 2.0 | 8 | 6 |
| Murder of Crows | U | uncommon | 2.1 | 43 | 35 |
| Falkenrath Marauders | R | rare | 2.1 | 14 | 12 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Liliana of the Veil | B | mythic | 2.2 | 4 | 4 |
| Tribute to Hunger | B | uncommon | 2.3 | 42 | 34 |
| Manor Gargoyle | C | rare | 2.3 | 19 | 19 |
