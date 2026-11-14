# Draft loop: what the nights add up to

25 night(s), 200 decks, 740 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 80 | 116-124-0 | 48% | 285-303 | 48% |
| conventional | 12 | 20-16-0 | 56% | 46-38 | 55% |
| unguided | 108 | 164-160-0 | 51% | 409-399 | 51% |

### The nights with all three (6)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 12 | 15-21-0 | 42% | 35-49 | 42% |
| conventional | 12 | 20-16-0 | 56% | 46-38 | 55% |
| unguided | 24 | 37-35-0 | 51% | 93-87 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 26 | 41-37-0 | 52% (190) |
| WU | 26 | 39-39-0 | 51% (192) |
| UB | 24 | 29-43-0 | 46% (183) |
| BR | 22 | 38-28-0 | 53% (160) |
| WG | 22 | 35-31-0 | 51% (156) |
| WR | 22 | 36-30-0 | 54% (159) |
| UR | 20 | 27-33-0 | 46% (149) |
| WB | 17 | 25-26-0 | 47% (131) |
| RG | 10 | 16-14-0 | 53% (76) |
| UG | 4 | 3-9-0 | 42% (31) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 50 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Moorland Haunt | C | rare | 7 | 51 | 63% | 10.7 | 11/118 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.3 | 30/308 |
| Furor of the Bitten | R | common | 17 | 122 | 60% | 10.1 | 58/584 |
| Slayer of the Wicked | W | uncommon | 25 | 178 | 60% | 1.7 | 29/48 |
| Midnight Haunting | W | uncommon | 22 | 160 | 59% | 3.9 | 27/106 |
| Mentor of the Meek | W | rare | 10 | 74 | 58% | 4.2 | 11/46 |
| Disciple of Griselbrand | B | uncommon | 17 | 126 | 58% | 9.2 | 37/341 |
| Manor Gargoyle | C | rare | 12 | 89 | 57% | 2.2 | 12/26 |
| Into the Maw of Hell | R | uncommon | 26 | 198 | 57% | 3.5 | 31/108 |
| Ashmouth Hound | R | common | 34 | 248 | 57% | 4.0 | 53/212 |
| Moldgraf Monstrosity | G | rare | 7 | 51 | 57% | 3.9 | 9/35 |
| Stromkirk Noble | R | rare | 10 | 74 | 57% | 5.0 | 11/55 |
| Bonds of Faith | W | common | 38 | 279 | 56% | 3.9 | 51/200 |
| Morkrut Banshee | B | uncommon | 23 | 177 | 56% | 1.7 | 27/45 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Elite Inquisitor | W | rare | 11 | 83 | 55% | 2.8 | 13/37 |
| Rally the Peasants | W | uncommon | 20 | 143 | 55% | 6.8 | 38/259 |
| Typhoid Rats | B | common | 38 | 277 | 55% | 3.3 | 54/179 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.0 | 14/154 |
| Avacynian Priest | W | common | 39 | 285 | 55% | 5.5 | 53/292 |
| Lumberknot | G | uncommon | 22 | 163 | 55% | 3.7 | 27/101 |
| Gatstaf Shepherd | G | uncommon | 24 | 178 | 54% | 4.4 | 32/141 |
| Tormented Pariah | R | common | 34 | 250 | 54% | 7.0 | 62/436 |
| Spider Spawning | G | uncommon | 20 | 147 | 54% | 3.4 | 34/116 |
| Villagers of Estwald | G | common | 27 | 199 | 54% | 4.1 | 58/236 |
| Voiceless Spirit | W | common | 47 | 348 | 54% | 3.7 | 54/202 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Grave Bramble | G | common | 24 | 177 | 54% | 8.9 | 52/461 |
| Thraben Sentry | W | common | 31 | 229 | 54% | 7.5 | 52/389 |
| Skirsdag Cultist | R | uncommon | 19 | 140 | 54% | 8.2 | 29/237 |
| Woodland Sleuth | G | common | 29 | 208 | 53% | 5.8 | 53/310 |
| Hanweir Watchkeep | R | uncommon | 17 | 131 | 53% | 6.4 | 29/186 |
| Woodland Cemetery | C | rare | 8 | 58 | 53% | 9.8 | 16/156 |
| Mindshrieker | U | rare | 8 | 58 | 53% | 6.8 | 10/68 |
| Unruly Mob | W | common | 34 | 246 | 53% | 9.2 | 52/476 |
| Avacyn's Pilgrim | G | common | 17 | 122 | 53% | 9.1 | 56/510 |
| Cloistered Youth | W | uncommon | 17 | 122 | 53% | 5.6 | 31/174 |
| Night Revelers | R | common | 32 | 235 | 53% | 7.1 | 44/313 |
| Silver-Inlaid Dagger | C | uncommon | 20 | 145 | 53% | 6.8 | 28/189 |
| Pitchburn Devils | R | common | 40 | 298 | 53% | 5.1 | 51/259 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 9 | 6 |
| Garruk Relentless | G | mythic | 1.0 | 6 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 29 | 29 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Falkenrath Noble | B | uncommon | 1.6 | 31 | 23 |
| Fiend Hunter | W | uncommon | 1.6 | 32 | 25 |
| Victim of Night | B | common | 1.6 | 51 | 39 |
| Slayer of the Wicked | W | uncommon | 1.7 | 29 | 25 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 57 | 39 |
| Morkrut Banshee | B | uncommon | 1.7 | 27 | 23 |
| Devil's Play | R | rare | 1.7 | 7 | 5 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Claustrophobia | U | common | 1.9 | 54 | 37 |
| Brimstone Volley | R | common | 1.9 | 51 | 32 |
| Chapel Geist | W | common | 2.0 | 45 | 32 |
| Murder of Crows | U | uncommon | 2.0 | 30 | 26 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Dearly Departed | W | rare | 2.0 | 11 | 9 |
| Instigator Gang | R | rare | 2.1 | 7 | 6 |
| Rebuke | W | common | 2.2 | 59 | 41 |
| Manor Gargoyle | C | rare | 2.2 | 12 | 12 |
| Tribute to Hunger | B | uncommon | 2.3 | 27 | 25 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Falkenrath Marauders | R | rare | 2.4 | 8 | 7 |
