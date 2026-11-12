# Draft loop: what the nights add up to

23 night(s), 184 decks, 681 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 76 | 113-115-0 | 50% | 276-284 | 49% |
| conventional | 8 | 13-11-0 | 54% | 29-26 | 53% |
| unguided | 100 | 150-150-0 | 50% | 376-371 | 50% |

### The nights with all three (4)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 8 | 12-12-0 | 50% | 26-30 | 46% |
| conventional | 8 | 13-11-0 | 54% | 29-26 | 53% |
| unguided | 16 | 23-25-0 | 48% | 60-59 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 25 | 41-34-0 | 54% (183) |
| WU | 25 | 37-38-0 | 50% (185) |
| UB | 23 | 28-41-0 | 46% (177) |
| WR | 21 | 36-27-0 | 56% (153) |
| BR | 20 | 35-25-0 | 53% (146) |
| WG | 20 | 33-27-0 | 52% (143) |
| UR | 17 | 20-31-0 | 43% (126) |
| WB | 14 | 19-23-0 | 45% (105) |
| RG | 9 | 14-13-0 | 51% (68) |
| U | 3 | 4-5-0 | 48% (21) |
| UG | 3 | 2-7-0 | 39% (23) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 46 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 6 | 46 | 70% | 1.5 | 6/9 |
| Falkenrath Marauders | R | rare | 7 | 49 | 67% | 2.4 | 8/19 |
| Mentor of the Meek | W | rare | 9 | 68 | 63% | 4.4 | 10/44 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.6 | 26/276 |
| Instigator Gang | R | rare | 6 | 47 | 60% | 2.1 | 7/15 |
| Slayer of the Wicked | W | uncommon | 23 | 163 | 60% | 1.6 | 27/44 |
| Elite Inquisitor | W | rare | 10 | 76 | 59% | 2.9 | 12/35 |
| Furor of the Bitten | R | common | 16 | 115 | 59% | 10.3 | 53/548 |
| Rally the Peasants | W | uncommon | 18 | 129 | 59% | 7.0 | 35/244 |
| Midnight Haunting | W | uncommon | 21 | 153 | 58% | 4.0 | 25/100 |
| Gatstaf Shepherd | G | uncommon | 21 | 156 | 58% | 4.6 | 29/132 |
| Bonds of Faith | W | common | 35 | 259 | 58% | 4.0 | 47/189 |
| Disciple of Griselbrand | B | uncommon | 16 | 119 | 57% | 9.1 | 35/318 |
| Into the Maw of Hell | R | uncommon | 25 | 190 | 57% | 3.6 | 29/103 |
| Lumberknot | G | uncommon | 20 | 148 | 57% | 3.8 | 25/96 |
| Thraben Sentry | W | common | 29 | 216 | 56% | 7.6 | 47/357 |
| Ashmouth Hound | R | common | 32 | 234 | 56% | 4.0 | 49/195 |
| Spider Spawning | G | uncommon | 19 | 140 | 56% | 3.4 | 32/109 |
| Silver-Inlaid Dagger | C | uncommon | 18 | 130 | 56% | 6.8 | 26/176 |
| Woodland Sleuth | G | common | 27 | 194 | 56% | 5.7 | 49/280 |
| Manor Gargoyle | C | rare | 11 | 82 | 56% | 2.3 | 11/25 |
| Skirsdag Cultist | R | uncommon | 16 | 120 | 56% | 8.1 | 26/211 |
| Avacyn's Pilgrim | G | common | 16 | 115 | 56% | 9.1 | 52/474 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Morkrut Banshee | B | uncommon | 21 | 161 | 55% | 1.6 | 25/41 |
| Stromkirk Noble | R | rare | 9 | 67 | 55% | 5.4 | 9/49 |
| Villagers of Estwald | G | common | 24 | 178 | 55% | 3.9 | 54/213 |
| Typhoid Rats | B | common | 36 | 263 | 55% | 3.3 | 50/165 |
| Avacynian Priest | W | common | 35 | 255 | 55% | 5.5 | 48/266 |
| Unruly Mob | W | common | 30 | 216 | 54% | 9.1 | 48/436 |
| Galvanic Juggernaut | C | uncommon | 27 | 202 | 54% | 1.2 | 27/32 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Curse of the Pierced Heart | R | common | 18 | 128 | 54% | 8.4 | 49/411 |
| Voiceless Spirit | W | common | 44 | 326 | 53% | 3.8 | 50/188 |
| Woodland Cemetery | C | rare | 8 | 58 | 53% | 9.8 | 16/156 |
| Victim of Night | B | common | 36 | 261 | 53% | 1.6 | 47/77 |
| Grave Bramble | G | common | 23 | 169 | 53% | 8.9 | 48/429 |
| Tormented Pariah | R | common | 30 | 220 | 53% | 7.1 | 56/397 |
| Selfless Cathar | W | common | 26 | 192 | 53% | 9.5 | 48/457 |
| Traitorous Blood | R | common | 14 | 100 | 53% | 9.4 | 53/497 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 7 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 6 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 27 | 27 |
| Angelic Overseer | W | mythic | 1.2 | 5 | 3 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 29 | 24 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Slayer of the Wicked | W | uncommon | 1.6 | 27 | 23 |
| Falkenrath Noble | B | uncommon | 1.6 | 28 | 21 |
| Victim of Night | B | common | 1.6 | 47 | 36 |
| Morkrut Banshee | B | uncommon | 1.6 | 25 | 21 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 53 | 38 |
| Claustrophobia | U | common | 1.8 | 50 | 36 |
| Brimstone Volley | R | common | 1.8 | 48 | 29 |
| Devil's Play | R | rare | 1.8 | 6 | 4 |
| Chapel Geist | W | common | 1.9 | 40 | 28 |
| Murder of Crows | U | uncommon | 1.9 | 28 | 24 |
| Dearly Departed | W | rare | 1.9 | 9 | 9 |
| Angel of Flight Alabaster | W | rare | 1.9 | 9 | 8 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Instigator Gang | R | rare | 2.1 | 7 | 6 |
| Rebuke | W | common | 2.2 | 55 | 37 |
| Tribute to Hunger | B | uncommon | 2.3 | 25 | 23 |
| Manor Gargoyle | C | rare | 2.3 | 11 | 11 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 11 | 6 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
