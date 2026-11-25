# Draft loop: what the nights add up to

36 night(s), 288 decks, 1067 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 102 | 148-158-0 | 48% | 365-387 | 49% |
| conventional | 34 | 53-49-0 | 52% | 128-116 | 52% |
| unguided | 152 | 231-225-0 | 51% | 574-564 | 50% |

### The nights with all three (17)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 34 | 47-55-0 | 46% | 115-133 | 46% |
| conventional | 34 | 53-49-0 | 52% | 128-116 | 52% |
| unguided | 68 | 104-100-0 | 51% | 258-252 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BR | 37 | 59-52-0 | 51% (272) |
| UB | 37 | 42-69-0 | 44% (281) |
| WR | 34 | 52-50-0 | 51% (247) |
| WU | 34 | 55-47-0 | 53% (253) |
| WG | 32 | 54-42-0 | 53% (236) |
| BG | 31 | 51-42-0 | 54% (225) |
| UR | 25 | 34-41-0 | 46% (185) |
| WB | 24 | 35-37-0 | 47% (177) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 11 | 14-19-0 | 47% (83) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 2 | 3-3-0 | 47% (15) |

## Cards in winning decks (at least 72 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 12 | 84 | 60% | 1.3 | 14/18 |
| Slayer of the Wicked | W | uncommon | 36 | 260 | 59% | 1.8 | 42/74 |
| Disciple of Griselbrand | B | uncommon | 21 | 154 | 58% | 9.2 | 49/451 |
| Mentor of the Meek | W | rare | 15 | 112 | 58% | 3.7 | 16/59 |
| Midnight Haunting | W | uncommon | 34 | 251 | 57% | 3.5 | 41/144 |
| Desperate Ravings | R | uncommon | 11 | 82 | 57% | 9.8 | 45/443 |
| Falkenrath Marauders | R | rare | 11 | 79 | 57% | 2.1 | 13/27 |
| Villagers of Estwald | G | common | 43 | 321 | 56% | 4.6 | 89/405 |
| Stromkirk Noble | R | rare | 14 | 103 | 56% | 4.4 | 15/66 |
| Elite Inquisitor | W | rare | 15 | 111 | 56% | 2.6 | 17/45 |
| Avacyn's Pilgrim | G | common | 25 | 186 | 55% | 8.9 | 80/709 |
| Into the Maw of Hell | R | uncommon | 37 | 277 | 55% | 3.2 | 46/147 |
| Furor of the Bitten | R | common | 28 | 203 | 55% | 9.8 | 83/805 |
| Bonds of Faith | W | common | 57 | 424 | 55% | 3.9 | 74/286 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.5 | 17/42 |
| Gatstaf Shepherd | G | uncommon | 35 | 261 | 55% | 5.1 | 52/264 |
| Lumberknot | G | uncommon | 32 | 239 | 55% | 4.3 | 40/174 |
| Ashmouth Hound | R | common | 49 | 360 | 55% | 3.7 | 76/280 |
| Avacynian Priest | W | common | 62 | 454 | 55% | 5.5 | 77/424 |
| Woodland Sleuth | G | common | 43 | 316 | 54% | 6.5 | 79/510 |
| Galvanic Juggernaut | C | uncommon | 45 | 340 | 54% | 1.2 | 45/53 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.2 | 38/234 |
| Thraben Sentry | W | common | 41 | 304 | 54% | 7.7 | 69/529 |
| Rage Thrower | R | uncommon | 26 | 190 | 54% | 9.1 | 56/509 |
| Voiceless Spirit | W | common | 64 | 471 | 54% | 3.5 | 76/266 |
| Grave Bramble | G | common | 34 | 254 | 54% | 8.7 | 77/673 |
| Spider Spawning | G | uncommon | 26 | 191 | 53% | 4.1 | 44/180 |
| Kindercatch | G | common | 49 | 366 | 53% | 7.5 | 75/559 |
| Doomed Traveler | W | common | 58 | 424 | 53% | 6.8 | 79/540 |
| Unruly Mob | W | common | 51 | 369 | 53% | 9.0 | 75/678 |
| Manor Gargoyle | C | rare | 18 | 134 | 53% | 2.3 | 18/41 |
| Hollowhenge Scavenger | G | uncommon | 23 | 170 | 53% | 4.4 | 36/160 |
| Bloodgift Demon | B | rare | 12 | 87 | 53% | 1.1 | 12/13 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Riot Devils | R | common | 45 | 337 | 53% | 7.8 | 78/608 |
| Hamlet Captain | G | uncommon | 31 | 233 | 53% | 7.2 | 42/301 |
| Morkrut Banshee | B | uncommon | 33 | 248 | 53% | 1.8 | 43/76 |
| Festerhide Boar | G | common | 49 | 363 | 53% | 6.6 | 80/530 |
| Kessig Wolf | R | common | 57 | 425 | 52% | 7.6 | 81/607 |
| Pitchburn Devils | R | common | 58 | 432 | 53% | 4.7 | 72/341 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 13 | 9 |
| Bloodgift Demon | B | rare | 1.1 | 12 | 12 |
| Olivia Voldaren | BR | mythic | 1.1 | 7 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 45 | 45 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Mikaeus, the Lunarch | W | mythic | 1.2 | 6 | 4 |
| Reaper from the Abyss | B | mythic | 1.2 | 5 | 5 |
| Sever the Bloodline | B | rare | 1.3 | 14 | 12 |
| Balefire Dragon | R | mythic | 1.3 | 7 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 46 | 36 |
| Devil's Play | R | rare | 1.5 | 13 | 9 |
| Geist-Honored Monk | W | rare | 1.6 | 11 | 7 |
| Victim of Night | B | common | 1.6 | 76 | 52 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 80 | 53 |
| Slayer of the Wicked | W | uncommon | 1.8 | 42 | 36 |
| Falkenrath Noble | B | uncommon | 1.8 | 43 | 33 |
| Morkrut Banshee | B | uncommon | 1.8 | 43 | 33 |
| Angel of Flight Alabaster | W | rare | 1.8 | 12 | 9 |
| Chapel Geist | W | common | 1.9 | 62 | 44 |
| Brimstone Volley | R | common | 1.9 | 73 | 48 |
| Claustrophobia | U | common | 2.0 | 77 | 54 |
| Skaab Ruinator | U | mythic | 2.0 | 8 | 6 |
| Murder of Crows | U | uncommon | 2.1 | 41 | 33 |
| Falkenrath Marauders | R | rare | 2.1 | 13 | 11 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Rebuke | W | common | 2.3 | 82 | 56 |
| Tribute to Hunger | B | uncommon | 2.3 | 38 | 32 |
| Manor Gargoyle | C | rare | 2.3 | 18 | 18 |
