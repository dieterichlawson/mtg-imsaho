# Draft loop: what the nights add up to

26 night(s), 208 decks, 766 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 82 | 119-127-0 | 48% | 292-309 | 49% |
| conventional | 14 | 23-19-0 | 55% | 52-45 | 54% |
| unguided | 112 | 170-166-0 | 51% | 422-412 | 51% |

### The nights with all three (7)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 14 | 18-24-0 | 43% | 42-55 | 43% |
| conventional | 14 | 23-19-0 | 55% | 52-45 | 54% |
| unguided | 28 | 43-41-0 | 51% | 106-100 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 27 | 44-37-0 | 54% (196) |
| WU | 26 | 39-39-0 | 51% (192) |
| UB | 25 | 30-45-0 | 46% (189) |
| BR | 24 | 40-32-0 | 52% (174) |
| WR | 23 | 38-31-0 | 55% (166) |
| WG | 22 | 35-31-0 | 51% (156) |
| UR | 20 | 27-33-0 | 46% (149) |
| WB | 19 | 27-30-0 | 45% (143) |
| RG | 10 | 16-14-0 | 53% (76) |
| UG | 5 | 5-10-0 | 45% (38) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 52 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Moldgraf Monstrosity | G | rare | 8 | 57 | 61% | 3.9 | 10/39 |
| Furor of the Bitten | R | common | 18 | 129 | 60% | 10.0 | 60/601 |
| Slayer of the Wicked | W | uncommon | 25 | 178 | 60% | 1.7 | 29/48 |
| Mentor of the Meek | W | rare | 11 | 81 | 59% | 3.9 | 12/47 |
| Midnight Haunting | W | uncommon | 22 | 160 | 59% | 3.9 | 27/106 |
| Disciple of Griselbrand | B | uncommon | 17 | 126 | 58% | 9.1 | 38/347 |
| Into the Maw of Hell | R | uncommon | 27 | 205 | 58% | 3.5 | 33/114 |
| Ashmouth Hound | R | common | 35 | 255 | 57% | 3.9 | 55/214 |
| Stromkirk Noble | R | rare | 10 | 74 | 57% | 5.0 | 11/55 |
| Lumberknot | G | uncommon | 23 | 169 | 56% | 3.9 | 29/114 |
| Gatstaf Shepherd | G | uncommon | 25 | 184 | 56% | 4.6 | 34/156 |
| Bonds of Faith | W | common | 40 | 292 | 56% | 3.8 | 54/204 |
| Manor Gargoyle | C | rare | 13 | 95 | 56% | 2.2 | 13/28 |
| Villagers of Estwald | G | common | 29 | 212 | 56% | 4.2 | 61/257 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Elite Inquisitor | W | rare | 11 | 83 | 55% | 2.8 | 13/37 |
| Morkrut Banshee | B | uncommon | 25 | 190 | 55% | 1.7 | 29/48 |
| Rally the Peasants | W | uncommon | 22 | 156 | 55% | 6.8 | 40/273 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.1 | 15/167 |
| Avacynian Priest | W | common | 41 | 298 | 55% | 5.5 | 55/304 |
| Woodland Sleuth | G | common | 30 | 214 | 55% | 5.9 | 55/326 |
| Spider Spawning | G | uncommon | 20 | 147 | 54% | 3.5 | 35/123 |
| Typhoid Rats | B | common | 41 | 296 | 54% | 3.3 | 57/187 |
| Voiceless Spirit | W | common | 49 | 361 | 54% | 3.7 | 56/208 |
| Tormented Pariah | R | common | 37 | 271 | 54% | 7.0 | 65/452 |
| Grave Bramble | G | common | 24 | 177 | 54% | 8.9 | 54/478 |
| Thraben Sentry | W | common | 33 | 242 | 54% | 7.6 | 54/408 |
| Unruly Mob | W | common | 36 | 259 | 53% | 9.1 | 54/489 |
| Avacyn's Pilgrim | G | common | 17 | 122 | 53% | 9.2 | 59/542 |
| Bump in the Night | B | common | 29 | 213 | 53% | 8.6 | 59/509 |
| Rage Thrower | R | uncommon | 18 | 130 | 53% | 9.7 | 39/378 |
| Silver-Inlaid Dagger | C | uncommon | 20 | 145 | 53% | 6.8 | 28/189 |
| Pitchburn Devils | R | common | 41 | 305 | 53% | 5.0 | 52/261 |
| Galvanic Juggernaut | C | uncommon | 30 | 224 | 53% | 1.2 | 30/35 |
| Hollowhenge Scavenger | G | uncommon | 18 | 132 | 53% | 4.1 | 25/102 |
| Hamlet Captain | G | uncommon | 24 | 178 | 53% | 6.8 | 31/210 |
| Festerhide Boar | G | common | 35 | 256 | 53% | 6.4 | 57/367 |
| Battleground Geist | U | uncommon | 24 | 183 | 52% | 3.3 | 28/93 |
| Rakish Heir | R | uncommon | 16 | 118 | 53% | 6.2 | 22/136 |
| Victim of Night | B | common | 40 | 290 | 52% | 1.6 | 53/84 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 6 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 30 | 30 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Fiend Hunter | W | uncommon | 1.6 | 34 | 26 |
| Victim of Night | B | common | 1.6 | 53 | 40 |
| Slayer of the Wicked | W | uncommon | 1.7 | 29 | 25 |
| Falkenrath Noble | B | uncommon | 1.7 | 32 | 24 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 60 | 41 |
| Morkrut Banshee | B | uncommon | 1.7 | 29 | 25 |
| Devil's Play | R | rare | 1.7 | 7 | 5 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Claustrophobia | U | common | 1.9 | 56 | 38 |
| Murder of Crows | U | uncommon | 1.9 | 31 | 27 |
| Brimstone Volley | R | common | 1.9 | 53 | 34 |
| Dearly Departed | W | rare | 1.9 | 12 | 9 |
| Chapel Geist | W | common | 2.0 | 46 | 33 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Instigator Gang | R | rare | 2.1 | 7 | 6 |
| Rebuke | W | common | 2.2 | 61 | 42 |
| Manor Gargoyle | C | rare | 2.2 | 13 | 13 |
| Tribute to Hunger | B | uncommon | 2.3 | 27 | 25 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Falkenrath Marauders | R | rare | 2.4 | 8 | 7 |
