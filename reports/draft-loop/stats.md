# Draft loop: what the nights add up to

22 night(s), 176 decks, 652 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 74 | 108-114-0 | 49% | 266-279 | 49% |
| conventional | 6 | 10-8-0 | 56% | 22-19 | 54% |
| unguided | 96 | 146-142-0 | 51% | 364-354 | 51% |

### The nights with all three (3)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 6 | 7-11-0 | 39% | 16-25 | 39% |
| conventional | 6 | 10-8-0 | 56% | 22-19 | 54% |
| unguided | 12 | 19-17-0 | 53% | 48-42 | 53% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 25 | 41-34-0 | 54% (183) |
| WU | 25 | 37-38-0 | 50% (185) |
| UB | 23 | 28-41-0 | 46% (177) |
| WR | 20 | 33-27-0 | 55% (145) |
| BR | 18 | 33-21-0 | 54% (131) |
| WG | 18 | 29-25-0 | 51% (129) |
| UR | 16 | 20-28-0 | 45% (119) |
| WB | 14 | 19-23-0 | 45% (105) |
| RG | 9 | 14-13-0 | 51% (68) |
| R | 2 | 3-3-0 | 50% (16) |
| U | 2 | 2-4-0 | 36% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 44 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 6 | 46 | 70% | 1.5 | 6/9 |
| Falkenrath Marauders | R | rare | 7 | 49 | 67% | 2.4 | 8/19 |
| Mentor of the Meek | W | rare | 9 | 68 | 63% | 4.4 | 10/44 |
| Moorland Haunt | C | rare | 6 | 44 | 61% | 11.1 | 10/111 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.6 | 25/264 |
| Slayer of the Wicked | W | uncommon | 22 | 156 | 60% | 1.6 | 26/42 |
| Instigator Gang | R | rare | 6 | 47 | 60% | 2.0 | 6/12 |
| Elite Inquisitor | W | rare | 10 | 76 | 59% | 2.9 | 12/35 |
| Skirsdag Cultist | R | uncommon | 15 | 113 | 58% | 8.0 | 24/191 |
| Rally the Peasants | W | uncommon | 17 | 121 | 58% | 7.1 | 33/235 |
| Furor of the Bitten | R | common | 15 | 107 | 58% | 10.4 | 50/519 |
| Ashmouth Hound | R | common | 31 | 227 | 58% | 4.0 | 46/185 |
| Gatstaf Shepherd | G | uncommon | 21 | 156 | 58% | 4.6 | 29/132 |
| Midnight Haunting | W | uncommon | 20 | 145 | 57% | 4.1 | 24/99 |
| Spider Spawning | G | uncommon | 18 | 133 | 57% | 3.4 | 31/105 |
| Morkrut Banshee | B | uncommon | 20 | 154 | 57% | 1.7 | 23/39 |
| Bonds of Faith | W | common | 33 | 244 | 57% | 4.0 | 45/180 |
| Into the Maw of Hell | R | uncommon | 25 | 190 | 57% | 3.6 | 28/102 |
| Disciple of Griselbrand | B | uncommon | 15 | 111 | 57% | 9.0 | 33/298 |
| Lumberknot | G | uncommon | 19 | 141 | 57% | 3.9 | 24/94 |
| Silver-Inlaid Dagger | C | uncommon | 18 | 130 | 56% | 6.8 | 26/176 |
| Woodland Sleuth | G | common | 26 | 187 | 56% | 5.6 | 47/263 |
| Typhoid Rats | B | common | 34 | 248 | 56% | 3.2 | 48/155 |
| Thraben Sentry | W | common | 27 | 201 | 56% | 7.6 | 44/334 |
| Avacyn's Pilgrim | G | common | 15 | 108 | 56% | 9.1 | 50/456 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Moldgraf Monstrosity | G | rare | 6 | 45 | 56% | 4.2 | 8/34 |
| Galvanic Juggernaut | C | uncommon | 26 | 195 | 55% | 1.2 | 26/31 |
| Stromkirk Noble | R | rare | 9 | 67 | 55% | 5.4 | 9/49 |
| Villagers of Estwald | G | common | 24 | 178 | 55% | 3.9 | 54/213 |
| Manor Gargoyle | C | rare | 10 | 75 | 55% | 2.4 | 10/24 |
| Avacynian Priest | W | common | 34 | 248 | 54% | 5.5 | 46/254 |
| Traitorous Blood | R | common | 12 | 85 | 54% | 9.5 | 51/483 |
| Unruly Mob | W | common | 28 | 202 | 54% | 9.1 | 45/410 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Rage Thrower | R | uncommon | 15 | 108 | 54% | 9.7 | 33/320 |
| Tormented Pariah | R | common | 30 | 220 | 53% | 7.1 | 56/397 |
| Kindercatch | G | common | 29 | 216 | 53% | 7.0 | 46/324 |
| Grave Bramble | G | common | 22 | 162 | 53% | 8.9 | 46/409 |
| Victim of Night | B | common | 35 | 253 | 53% | 1.7 | 46/76 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 6 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 6 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 26 | 26 |
| Angelic Overseer | W | mythic | 1.2 | 5 | 3 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 27 | 23 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Slayer of the Wicked | W | uncommon | 1.6 | 26 | 22 |
| Falkenrath Noble | B | uncommon | 1.6 | 26 | 19 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 50 | 37 |
| Victim of Night | B | common | 1.7 | 46 | 35 |
| Morkrut Banshee | B | uncommon | 1.7 | 23 | 20 |
| Claustrophobia | U | common | 1.8 | 48 | 35 |
| Devil's Play | R | rare | 1.8 | 6 | 4 |
| Murder of Crows | U | uncommon | 1.9 | 27 | 23 |
| Brimstone Volley | R | common | 1.9 | 46 | 28 |
| Dearly Departed | W | rare | 1.9 | 9 | 9 |
| Angel of Flight Alabaster | W | rare | 1.9 | 9 | 8 |
| Chapel Geist | W | common | 2.0 | 38 | 27 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Instigator Gang | R | rare | 2.0 | 6 | 6 |
| Rebuke | W | common | 2.2 | 52 | 36 |
| Tribute to Hunger | B | uncommon | 2.3 | 25 | 23 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 11 | 6 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Falkenrath Marauders | R | rare | 2.4 | 8 | 7 |
