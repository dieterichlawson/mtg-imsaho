# Draft loop: what the nights add up to

33 night(s), 264 decks, 977 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 96 | 141-147-0 | 49% | 344-361 | 49% |
| conventional | 28 | 42-42-0 | 50% | 103-98 | 51% |
| unguided | 140 | 213-207-0 | 51% | 530-518 | 51% |

### The nights with all three (14)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 28 | 40-44-0 | 48% | 94-107 | 47% |
| conventional | 28 | 42-42-0 | 50% | 103-98 | 51% |
| unguided | 56 | 86-82-0 | 51% | 214-206 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 35 | 41-64-0 | 45% (266) |
| BR | 33 | 52-47-0 | 50% (240) |
| WR | 33 | 52-47-0 | 52% (240) |
| WU | 31 | 49-44-0 | 52% (232) |
| BG | 29 | 48-39-0 | 54% (210) |
| WG | 28 | 46-38-0 | 52% (204) |
| UR | 22 | 29-37-0 | 45% (163) |
| WB | 22 | 32-34-0 | 47% (163) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 9 | 12-15-0 | 49% (68) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 66 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 9 | 69 | 61% | 1.3 | 11/14 |
| Slayer of the Wicked | W | uncommon | 33 | 238 | 61% | 1.8 | 39/70 |
| Disciple of Griselbrand | B | uncommon | 20 | 146 | 60% | 9.2 | 45/412 |
| Falkenrath Marauders | R | rare | 10 | 71 | 59% | 2.2 | 12/26 |
| Midnight Haunting | W | uncommon | 30 | 222 | 58% | 3.7 | 37/137 |
| Mentor of the Meek | W | rare | 14 | 104 | 58% | 3.7 | 15/56 |
| Elite Inquisitor | W | rare | 14 | 104 | 57% | 2.7 | 16/43 |
| Villagers of Estwald | G | common | 40 | 297 | 57% | 4.5 | 83/377 |
| Furor of the Bitten | R | common | 27 | 196 | 57% | 9.6 | 76/722 |
| Stromkirk Noble | R | rare | 13 | 96 | 56% | 4.6 | 14/65 |
| Galvanic Juggernaut | C | uncommon | 42 | 317 | 56% | 1.2 | 42/50 |
| Lumberknot | G | uncommon | 29 | 215 | 55% | 4.4 | 37/164 |
| Into the Maw of Hell | R | uncommon | 34 | 256 | 55% | 3.3 | 41/135 |
| Avacynian Priest | W | common | 54 | 396 | 55% | 5.5 | 69/382 |
| Bonds of Faith | W | common | 52 | 385 | 55% | 3.9 | 68/265 |
| Gatstaf Shepherd | G | uncommon | 33 | 245 | 55% | 5.1 | 48/245 |
| Manor Gargoyle | C | rare | 17 | 127 | 55% | 2.2 | 17/38 |
| Grave Bramble | G | common | 30 | 222 | 55% | 9.0 | 69/620 |
| Woodland Sleuth | G | common | 39 | 284 | 55% | 6.2 | 72/448 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.6 | 16/41 |
| Ashmouth Hound | R | common | 46 | 337 | 55% | 3.7 | 70/261 |
| Voiceless Spirit | W | common | 60 | 442 | 55% | 3.5 | 70/246 |
| Avacyn's Pilgrim | G | common | 21 | 154 | 54% | 9.1 | 73/664 |
| Thraben Sentry | W | common | 38 | 280 | 54% | 7.5 | 62/467 |
| Rally the Peasants | W | uncommon | 31 | 222 | 54% | 6.8 | 51/345 |
| Spider Spawning | G | uncommon | 25 | 184 | 53% | 3.8 | 41/156 |
| Hanweir Watchkeep | R | uncommon | 20 | 152 | 53% | 6.4 | 34/217 |
| Typhoid Rats | B | common | 50 | 363 | 53% | 3.4 | 72/242 |
| Rage Thrower | R | uncommon | 24 | 175 | 53% | 9.2 | 51/469 |
| Unruly Mob | W | common | 47 | 339 | 53% | 9.1 | 69/628 |
| Kindercatch | G | common | 44 | 326 | 53% | 7.3 | 69/507 |
| Morkrut Banshee | B | uncommon | 30 | 226 | 53% | 1.7 | 37/63 |
| Festerhide Boar | G | common | 46 | 340 | 53% | 6.6 | 74/487 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Hollowhenge Scavenger | G | uncommon | 22 | 163 | 53% | 4.2 | 33/138 |
| Doomed Traveler | W | common | 53 | 387 | 53% | 7.0 | 72/504 |
| Riot Devils | R | common | 41 | 306 | 53% | 7.8 | 72/561 |
| Victim of Night | B | common | 50 | 366 | 52% | 1.6 | 70/110 |
| Pitchburn Devils | R | common | 55 | 407 | 52% | 4.7 | 67/317 |
| Hysterical Blindness | U | common | 15 | 109 | 52% | 10.2 | 74/756 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Reaper from the Abyss | B | mythic | 1.0 | 4 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 11 | 8 |
| Bloodgift Demon | B | rare | 1.1 | 9 | 9 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 42 | 42 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Olivia Voldaren | BR | mythic | 1.2 | 4 | 0 |
| Sever the Bloodline | B | rare | 1.3 | 11 | 9 |
| Balefire Dragon | R | mythic | 1.4 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 42 | 34 |
| Geist-Honored Monk | W | rare | 1.6 | 10 | 6 |
| Victim of Night | B | common | 1.6 | 70 | 50 |
| Devil's Play | R | rare | 1.6 | 11 | 7 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 74 | 50 |
| Morkrut Banshee | B | uncommon | 1.7 | 37 | 30 |
| Slayer of the Wicked | W | uncommon | 1.8 | 39 | 33 |
| Falkenrath Noble | B | uncommon | 1.8 | 39 | 31 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 57 | 41 |
| Brimstone Volley | R | common | 2.0 | 68 | 44 |
| Claustrophobia | U | common | 2.1 | 70 | 48 |
| Murder of Crows | U | uncommon | 2.1 | 38 | 32 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Falkenrath Marauders | R | rare | 2.2 | 12 | 10 |
| Manor Gargoyle | C | rare | 2.2 | 17 | 17 |
| Skaab Ruinator | U | mythic | 2.2 | 6 | 4 |
| Daybreak Ranger | G | rare | 2.3 | 18 | 12 |
| Tribute to Hunger | B | uncommon | 2.3 | 38 | 32 |
