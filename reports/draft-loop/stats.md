# Draft loop: what the nights add up to

17 night(s), 136 decks, 504 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 60 | 87-93-0 | 48% | 218-227 | 49% |
| unguided | 76 | 117-111-0 | 51% | 286-277 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 21 | 35-28-0 | 54% (152) |
| WU | 19 | 28-29-0 | 50% (143) |
| UB | 17 | 21-30-0 | 46% (131) |
| UR | 14 | 17-25-0 | 45% (103) |
| WR | 14 | 23-19-0 | 54% (101) |
| BR | 13 | 24-15-0 | 56% (93) |
| WG | 13 | 22-17-0 | 52% (96) |
| WB | 11 | 15-18-0 | 46% (83) |
| RG | 7 | 10-11-0 | 48% (52) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 34 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Mentor of the Meek | W | rare | 6 | 46 | 65% | 5.0 | 7/35 |
| Spider Spawning | G | uncommon | 14 | 104 | 63% | 3.1 | 23/71 |
| Furor of the Bitten | R | common | 8 | 56 | 62% | 11.0 | 37/408 |
| Skirsdag Cultist | R | uncommon | 9 | 66 | 61% | 8.1 | 16/129 |
| Morkrut Banshee | B | uncommon | 12 | 91 | 60% | 1.5 | 15/22 |
| Gatstaf Shepherd | G | uncommon | 17 | 127 | 60% | 4.2 | 24/102 |
| Typhoid Rats | B | common | 25 | 186 | 59% | 3.4 | 37/127 |
| Slayer of the Wicked | W | uncommon | 17 | 124 | 59% | 1.6 | 20/33 |
| Stromkirk Noble | R | rare | 7 | 51 | 59% | 5.6 | 7/39 |
| Avacyn's Pilgrim | G | common | 10 | 75 | 59% | 9.2 | 40/367 |
| Elite Inquisitor | W | rare | 7 | 53 | 58% | 3.0 | 9/27 |
| Curse of the Pierced Heart | R | common | 11 | 78 | 58% | 8.7 | 36/313 |
| Thraben Sentry | W | common | 22 | 165 | 58% | 7.8 | 31/242 |
| Woodland Sleuth | G | common | 21 | 153 | 58% | 5.2 | 37/192 |
| Ashmouth Hound | R | common | 23 | 167 | 57% | 3.9 | 35/138 |
| Traitorous Blood | R | common | 9 | 65 | 57% | 10.0 | 41/409 |
| Midnight Haunting | W | uncommon | 15 | 110 | 56% | 4.4 | 19/84 |
| Unruly Mob | W | common | 21 | 156 | 56% | 9.8 | 34/333 |
| Villagers of Estwald | G | common | 19 | 141 | 56% | 3.5 | 43/151 |
| Lumberknot | G | uncommon | 15 | 109 | 56% | 4.1 | 19/78 |
| Prey Upon | G | common | 21 | 152 | 56% | 2.6 | 30/78 |
| Desperate Ravings | R | uncommon | 6 | 43 | 56% | 10.4 | 20/209 |
| Manor Gargoyle | C | rare | 8 | 61 | 56% | 2.4 | 8/19 |
| Silver-Inlaid Dagger | C | uncommon | 17 | 123 | 55% | 6.8 | 24/164 |
| Galvanic Juggernaut | C | uncommon | 20 | 147 | 55% | 1.2 | 20/24 |
| Selfless Cathar | W | common | 17 | 129 | 55% | 9.8 | 34/332 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.6 | 22/168 |
| Bonds of Faith | W | common | 25 | 190 | 55% | 4.1 | 36/149 |
| Falkenrath Noble | B | uncommon | 14 | 101 | 54% | 1.6 | 21/34 |
| Invisible Stalker | U | uncommon | 17 | 127 | 54% | 5.1 | 21/107 |
| Into the Maw of Hell | R | uncommon | 19 | 142 | 54% | 3.8 | 22/84 |
| Ghoulcaller's Chant | B | common | 24 | 179 | 54% | 7.8 | 36/282 |
| Hollowhenge Scavenger | G | uncommon | 13 | 96 | 54% | 3.6 | 16/58 |
| Kruin Outlaw | R | rare | 10 | 74 | 54% | 2.5 | 10/25 |
| Bump in the Night | B | common | 19 | 139 | 54% | 8.4 | 38/320 |
| Rakish Heir | R | uncommon | 11 | 82 | 54% | 6.4 | 17/109 |
| Woodland Cemetery | C | rare | 6 | 41 | 54% | 9.0 | 13/117 |
| Night Revelers | R | common | 23 | 168 | 54% | 6.7 | 29/194 |
| Avacynian Priest | W | common | 26 | 191 | 53% | 5.5 | 36/198 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 5 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 20 | 20 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 5 | 5 |
| Devil's Play | R | rare | 1.2 | 4 | 3 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 15 | 12 |
| Slayer of the Wicked | W | uncommon | 1.6 | 20 | 17 |
| Falkenrath Noble | B | uncommon | 1.6 | 21 | 14 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.6 | 21 | 18 |
| Angel of Flight Alabaster | W | rare | 1.6 | 5 | 4 |
| Claustrophobia | U | common | 1.7 | 37 | 28 |
| Dead Weight | B | common | 1.7 | 37 | 29 |
| Murder of Crows | U | uncommon | 1.7 | 21 | 18 |
| Victim of Night | B | common | 1.7 | 35 | 27 |
| Chapel Geist | W | common | 1.8 | 30 | 23 |
| Brimstone Volley | R | common | 1.8 | 35 | 21 |
| Daybreak Ranger | G | rare | 1.9 | 8 | 5 |
| Dearly Departed | W | rare | 2.0 | 7 | 7 |
| Rebuke | W | common | 2.2 | 39 | 28 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Tribute to Hunger | B | uncommon | 2.3 | 23 | 21 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Divine Reckoning | W | rare | 2.4 | 5 | 0 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Manor Gargoyle | C | rare | 2.4 | 8 | 8 |
