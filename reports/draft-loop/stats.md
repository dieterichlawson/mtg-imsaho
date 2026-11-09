# Draft loop: what the nights add up to

20 night(s), 160 decks, 593 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 70 | 102-108-0 | 49% | 253-264 | 49% |
| conventional | 2 | 3-3-0 | 50% | 6-6 | 50% |
| unguided | 88 | 135-129-0 | 51% | 334-323 | 51% |

### The nights with all three (1)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 2 | 1-5-0 | 17% | 3-10 | 23% |
| conventional | 2 | 3-3-0 | 50% | 6-6 | 50% |
| unguided | 4 | 8-4-0 | 67% | 18-11 | 62% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 23 | 39-30-0 | 54% (167) |
| UB | 21 | 26-37-0 | 46% (162) |
| WU | 21 | 31-32-0 | 51% (156) |
| WR | 17 | 27-24-0 | 54% (123) |
| BR | 16 | 30-18-0 | 56% (115) |
| UR | 16 | 20-28-0 | 45% (119) |
| WG | 15 | 24-21-0 | 50% (109) |
| WB | 14 | 19-23-0 | 45% (105) |
| RG | 9 | 14-13-0 | 51% (68) |
| R | 2 | 3-3-0 | 50% (16) |
| U | 2 | 2-4-0 | 36% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 40 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 7 | 49 | 67% | 2.4 | 8/19 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.5 | 23/242 |
| Morkrut Banshee | B | uncommon | 18 | 137 | 60% | 1.7 | 21/36 |
| Spider Spawning | G | uncommon | 17 | 127 | 60% | 3.3 | 27/89 |
| Stromkirk Noble | R | rare | 7 | 51 | 59% | 5.6 | 7/39 |
| Mentor of the Meek | W | rare | 7 | 53 | 58% | 4.9 | 8/39 |
| Slayer of the Wicked | W | uncommon | 19 | 137 | 58% | 1.7 | 23/38 |
| Elite Inquisitor | W | rare | 9 | 67 | 58% | 2.7 | 11/30 |
| Gatstaf Shepherd | G | uncommon | 21 | 156 | 58% | 4.6 | 29/132 |
| Skirsdag Cultist | R | uncommon | 14 | 104 | 58% | 7.8 | 22/172 |
| Ashmouth Hound | R | common | 28 | 205 | 58% | 3.8 | 41/155 |
| Furor of the Bitten | R | common | 12 | 85 | 58% | 10.4 | 44/456 |
| Midnight Haunting | W | uncommon | 18 | 131 | 57% | 4.4 | 22/96 |
| Thraben Sentry | W | common | 23 | 173 | 57% | 7.6 | 36/275 |
| Rally the Peasants | W | uncommon | 13 | 93 | 57% | 7.3 | 29/213 |
| Disciple of Griselbrand | B | uncommon | 15 | 111 | 57% | 8.8 | 29/256 |
| Into the Maw of Hell | R | uncommon | 23 | 173 | 57% | 3.7 | 26/97 |
| Typhoid Rats | B | common | 31 | 226 | 57% | 3.2 | 44/143 |
| Lumberknot | G | uncommon | 18 | 133 | 56% | 4.1 | 22/91 |
| Woodland Sleuth | G | common | 23 | 167 | 56% | 5.5 | 43/235 |
| Silver-Inlaid Dagger | C | uncommon | 18 | 130 | 56% | 6.8 | 25/169 |
| Galvanic Juggernaut | C | uncommon | 24 | 179 | 56% | 1.2 | 24/29 |
| Avacyn's Pilgrim | G | common | 12 | 88 | 56% | 9.2 | 47/432 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Unruly Mob | W | common | 25 | 184 | 55% | 9.3 | 41/381 |
| Villagers of Estwald | G | common | 24 | 178 | 55% | 3.9 | 54/213 |
| Bonds of Faith | W | common | 29 | 216 | 55% | 4.1 | 41/167 |
| Prey Upon | G | common | 24 | 175 | 54% | 2.7 | 35/93 |
| Traitorous Blood | R | common | 12 | 85 | 54% | 9.7 | 47/455 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Victim of Night | B | common | 32 | 229 | 54% | 1.6 | 41/66 |
| Rakish Heir | R | uncommon | 11 | 82 | 54% | 6.4 | 17/109 |
| Woodland Cemetery | C | rare | 6 | 41 | 54% | 9.2 | 14/129 |
| Avacynian Priest | W | common | 30 | 220 | 54% | 5.6 | 42/236 |
| Hamlet Captain | G | uncommon | 20 | 151 | 54% | 6.6 | 25/164 |
| Manor Gargoyle | C | rare | 9 | 69 | 54% | 2.6 | 9/23 |
| Curse of the Pierced Heart | R | common | 16 | 114 | 54% | 8.6 | 43/368 |
| Boneyard Wurm | G | uncommon | 18 | 135 | 53% | 7.5 | 24/179 |
| Hanweir Watchkeep | R | uncommon | 14 | 107 | 53% | 6.5 | 20/129 |
| Tormented Pariah | R | common | 30 | 220 | 53% | 7.1 | 56/397 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 6 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 24 | 24 |
| Angelic Overseer | W | mythic | 1.2 | 5 | 3 |
| Devil's Play | R | rare | 1.2 | 5 | 4 |
| Geist-Honored Monk | W | rare | 1.4 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 25 | 21 |
| Falkenrath Noble | B | uncommon | 1.6 | 24 | 17 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Victim of Night | B | common | 1.6 | 41 | 32 |
| Sever the Bloodline | B | rare | 1.6 | 5 | 5 |
| Slayer of the Wicked | W | uncommon | 1.7 | 23 | 19 |
| Morkrut Banshee | B | uncommon | 1.7 | 21 | 18 |
| Claustrophobia | U | common | 1.8 | 43 | 33 |
| Dead Weight | B | common | 1.8 | 45 | 32 |
| Murder of Crows | U | uncommon | 1.8 | 25 | 21 |
| Brimstone Volley | R | common | 1.9 | 42 | 26 |
| Chapel Geist | W | common | 2.0 | 35 | 25 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Dearly Departed | W | rare | 2.0 | 8 | 8 |
| Angel of Flight Alabaster | W | rare | 2.0 | 7 | 6 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Rebuke | W | common | 2.3 | 46 | 31 |
| Tribute to Hunger | B | uncommon | 2.3 | 24 | 22 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 11 | 6 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Falkenrath Marauders | R | rare | 2.4 | 8 | 7 |
