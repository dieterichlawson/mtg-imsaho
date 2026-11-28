# Draft loop: what the nights add up to

39 night(s), 312 decks, 1155 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 108 | 156-168-0 | 48% | 385-409 | 48% |
| conventional | 40 | 67-53-0 | 56% | 159-128 | 55% |
| unguided | 164 | 245-247-0 | 50% | 611-618 | 50% |

### The nights with all three (20)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 40 | 55-65-0 | 46% | 135-155 | 47% |
| conventional | 40 | 67-53-0 | 56% | 159-128 | 55% |
| unguided | 80 | 118-122-0 | 49% | 295-306 | 49% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 42 | 47-79-0 | 44% (318) |
| BR | 40 | 65-55-0 | 51% (298) |
| WR | 37 | 59-52-0 | 53% (267) |
| WU | 37 | 62-49-0 | 55% (275) |
| WG | 35 | 57-48-0 | 52% (256) |
| BG | 33 | 53-46-0 | 53% (240) |
| UR | 26 | 34-44-0 | 45% (191) |
| WB | 24 | 35-37-0 | 47% (177) |
| RG | 16 | 28-20-0 | 54% (123) |
| UG | 13 | 16-23-0 | 46% (98) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 2 | 3-3-0 | 47% (15) |

## Cards in winning decks (at least 78 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Slayer of the Wicked | W | uncommon | 39 | 281 | 61% | 1.8 | 45/79 |
| Sever the Bloodline | B | rare | 14 | 99 | 60% | 1.2 | 17/21 |
| Midnight Haunting | W | uncommon | 37 | 272 | 59% | 3.4 | 45/152 |
| Mentor of the Meek | W | rare | 15 | 112 | 58% | 3.7 | 16/59 |
| Desperate Ravings | R | uncommon | 11 | 82 | 57% | 9.9 | 49/485 |
| Kruin Outlaw | R | rare | 15 | 111 | 57% | 2.4 | 18/44 |
| Stromkirk Noble | R | rare | 15 | 111 | 57% | 4.7 | 16/75 |
| Disciple of Griselbrand | B | uncommon | 22 | 161 | 57% | 9.3 | 52/482 |
| Into the Maw of Hell | R | uncommon | 41 | 307 | 56% | 3.3 | 51/165 |
| Elite Inquisitor | W | rare | 15 | 111 | 56% | 2.6 | 17/45 |
| Bonds of Faith | W | common | 63 | 463 | 56% | 3.9 | 81/312 |
| Furor of the Bitten | R | common | 29 | 211 | 55% | 9.9 | 91/891 |
| Avacynian Priest | W | common | 66 | 480 | 55% | 5.6 | 84/467 |
| Galvanic Juggernaut | C | uncommon | 48 | 361 | 55% | 1.2 | 48/60 |
| Villagers of Estwald | G | common | 51 | 379 | 54% | 4.5 | 102/456 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.1 | 39/236 |
| Rage Thrower | R | uncommon | 27 | 199 | 54% | 9.2 | 59/544 |
| Thraben Sentry | W | common | 41 | 304 | 54% | 7.7 | 71/547 |
| Bloodgift Demon | B | rare | 13 | 94 | 54% | 1.1 | 13/14 |
| Cloistered Youth | W | uncommon | 25 | 179 | 54% | 5.8 | 41/238 |
| Doomed Traveler | W | common | 63 | 460 | 54% | 6.9 | 85/585 |
| Voiceless Spirit | W | common | 70 | 511 | 54% | 3.5 | 83/290 |
| Ashmouth Hound | R | common | 55 | 406 | 54% | 3.6 | 82/296 |
| Lumberknot | G | uncommon | 35 | 261 | 54% | 4.3 | 43/187 |
| Riot Devils | R | common | 51 | 381 | 54% | 7.6 | 85/650 |
| Avacyn's Pilgrim | G | common | 29 | 214 | 53% | 8.7 | 87/755 |
| Unruly Mob | W | common | 54 | 391 | 53% | 9.0 | 81/728 |
| Manor Gargoyle | C | rare | 19 | 141 | 53% | 2.3 | 19/43 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.0 | 17/34 |
| Silverchase Fox | W | common | 61 | 445 | 53% | 7.2 | 83/597 |
| Gatstaf Shepherd | G | uncommon | 39 | 288 | 53% | 5.2 | 58/301 |
| Silent Departure | U | common | 42 | 309 | 53% | 8.4 | 76/636 |
| Morkrut Banshee | B | uncommon | 33 | 248 | 53% | 1.8 | 43/76 |
| Feeling of Dread | W | common | 45 | 327 | 53% | 8.2 | 84/688 |
| Spider Spawning | G | uncommon | 27 | 198 | 53% | 4.3 | 47/203 |
| Woodland Sleuth | G | common | 50 | 366 | 52% | 6.6 | 87/572 |
| Crossway Vampire | R | common | 55 | 410 | 52% | 5.4 | 81/438 |
| Rebuke | W | common | 60 | 445 | 52% | 2.4 | 87/206 |
| Pitchburn Devils | R | common | 61 | 454 | 52% | 4.8 | 77/369 |
| Thraben Purebloods | W | common | 59 | 432 | 52% | 9.6 | 90/862 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Angelic Overseer | W | mythic | 1.1 | 7 | 5 |
| Mikaeus, the Lunarch | W | mythic | 1.1 | 7 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 13 | 9 |
| Bloodgift Demon | B | rare | 1.1 | 13 | 13 |
| Reaper from the Abyss | B | mythic | 1.1 | 7 | 7 |
| Olivia Voldaren | BR | mythic | 1.1 | 8 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 48 | 48 |
| Sever the Bloodline | B | rare | 1.2 | 17 | 14 |
| Balefire Dragon | R | mythic | 1.2 | 9 | 6 |
| Fiend Hunter | W | uncommon | 1.5 | 51 | 41 |
| Geist-Honored Monk | W | rare | 1.5 | 14 | 10 |
| Devil's Play | R | rare | 1.5 | 13 | 9 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 7 | 5 |
| Victim of Night | B | common | 1.6 | 83 | 59 |
| Slayer of the Wicked | W | uncommon | 1.8 | 45 | 39 |
| Falkenrath Noble | B | uncommon | 1.8 | 47 | 36 |
| Dead Weight | B | common | 1.8 | 86 | 57 |
| Morkrut Banshee | B | uncommon | 1.8 | 43 | 33 |
| Angel of Flight Alabaster | W | rare | 1.8 | 13 | 10 |
| Brimstone Volley | R | common | 1.9 | 79 | 53 |
| Skaab Ruinator | U | mythic | 1.9 | 9 | 6 |
| Claustrophobia | U | common | 2.0 | 83 | 59 |
| Chapel Geist | W | common | 2.0 | 68 | 49 |
| Dearly Departed | W | rare | 2.0 | 17 | 12 |
| Falkenrath Marauders | R | rare | 2.1 | 15 | 13 |
| Murder of Crows | U | uncommon | 2.2 | 44 | 36 |
| Liliana of the Veil | B | mythic | 2.2 | 4 | 4 |
| Kessig Cagebreakers | G | rare | 2.2 | 18 | 12 |
| Tribute to Hunger | B | uncommon | 2.3 | 44 | 36 |
