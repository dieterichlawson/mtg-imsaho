# Draft loop: what the nights add up to

18 night(s), 144 decks, 535 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 64 | 92-100-0 | 48% | 231-243 | 49% |
| unguided | 80 | 124-116-0 | 52% | 304-292 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 21 | 35-28-0 | 54% (152) |
| WU | 20 | 30-30-0 | 51% (150) |
| UB | 18 | 22-32-0 | 46% (139) |
| UR | 15 | 19-26-0 | 46% (112) |
| BR | 14 | 27-15-0 | 57% (102) |
| WG | 14 | 22-20-0 | 50% (103) |
| WR | 14 | 23-19-0 | 54% (101) |
| WB | 13 | 17-22-0 | 44% (97) |
| RG | 8 | 12-12-0 | 50% (60) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 36 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Furor of the Bitten | R | common | 9 | 65 | 62% | 10.7 | 39/416 |
| Skirsdag Cultist | R | uncommon | 11 | 84 | 61% | 8.1 | 18/145 |
| Spider Spawning | G | uncommon | 15 | 111 | 60% | 3.1 | 25/77 |
| Morkrut Banshee | B | uncommon | 14 | 108 | 60% | 1.5 | 17/26 |
| Elite Inquisitor | W | rare | 8 | 60 | 60% | 2.8 | 10/28 |
| Slayer of the Wicked | W | uncommon | 18 | 131 | 60% | 1.7 | 22/37 |
| Stromkirk Noble | R | rare | 7 | 51 | 59% | 5.6 | 7/39 |
| Mentor of the Meek | W | rare | 7 | 53 | 58% | 4.9 | 8/39 |
| Moldgraf Monstrosity | G | rare | 5 | 36 | 58% | 4.4 | 7/31 |
| Thraben Sentry | W | common | 22 | 165 | 58% | 7.8 | 31/242 |
| Gatstaf Shepherd | G | uncommon | 18 | 134 | 57% | 4.5 | 26/116 |
| Typhoid Rats | B | common | 27 | 200 | 58% | 3.3 | 39/130 |
| Curse of the Pierced Heart | R | common | 12 | 87 | 57% | 8.5 | 39/333 |
| Midnight Haunting | W | uncommon | 16 | 117 | 57% | 4.4 | 20/88 |
| Traitorous Blood | R | common | 9 | 65 | 57% | 9.9 | 43/427 |
| Ashmouth Hound | R | common | 24 | 176 | 57% | 3.9 | 37/144 |
| Rally the Peasants | W | uncommon | 11 | 78 | 56% | 7.5 | 24/181 |
| Lumberknot | G | uncommon | 16 | 117 | 56% | 4.0 | 20/79 |
| Unruly Mob | W | common | 23 | 169 | 56% | 9.5 | 37/351 |
| Desperate Ravings | R | uncommon | 6 | 43 | 56% | 10.4 | 21/218 |
| Woodland Sleuth | G | common | 22 | 160 | 56% | 5.4 | 39/209 |
| Galvanic Juggernaut | C | uncommon | 21 | 155 | 55% | 1.2 | 21/25 |
| Silver-Inlaid Dagger | C | uncommon | 17 | 123 | 55% | 6.8 | 24/164 |
| Into the Maw of Hell | R | uncommon | 20 | 151 | 55% | 3.9 | 23/89 |
| Avacyn's Pilgrim | G | common | 11 | 82 | 55% | 9.2 | 43/394 |
| Invisible Stalker | U | uncommon | 19 | 142 | 55% | 4.9 | 23/112 |
| Villagers of Estwald | G | common | 21 | 156 | 54% | 3.6 | 48/174 |
| Disciple of Griselbrand | B | uncommon | 13 | 96 | 54% | 8.7 | 26/227 |
| Bonds of Faith | W | common | 26 | 196 | 54% | 4.1 | 37/152 |
| Kruin Outlaw | R | rare | 10 | 74 | 54% | 2.6 | 11/29 |
| Prey Upon | G | common | 22 | 159 | 54% | 2.5 | 31/79 |
| Night Revelers | R | common | 24 | 177 | 54% | 6.9 | 31/213 |
| Rakish Heir | R | uncommon | 11 | 82 | 54% | 6.4 | 17/109 |
| Woodland Cemetery | C | rare | 6 | 41 | 54% | 9.2 | 14/129 |
| Manor Gargoyle | C | rare | 9 | 69 | 54% | 2.6 | 9/23 |
| Ghoulcaller's Chant | B | common | 25 | 185 | 54% | 7.9 | 39/307 |
| Avacynian Priest | W | common | 28 | 206 | 53% | 5.6 | 38/212 |
| Spectral Flight | U | common | 21 | 161 | 53% | 5.4 | 34/183 |
| Falkenrath Noble | B | uncommon | 15 | 109 | 53% | 1.6 | 22/36 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 5 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 21 | 21 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 5 | 5 |
| Devil's Play | R | rare | 1.2 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 22 | 19 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 17 | 14 |
| Falkenrath Noble | B | uncommon | 1.6 | 22 | 15 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Victim of Night | B | common | 1.6 | 37 | 29 |
| Slayer of the Wicked | W | uncommon | 1.7 | 22 | 18 |
| Claustrophobia | U | common | 1.8 | 39 | 30 |
| Dead Weight | B | common | 1.8 | 40 | 30 |
| Murder of Crows | U | uncommon | 1.8 | 22 | 19 |
| Brimstone Volley | R | common | 1.8 | 38 | 23 |
| Angel of Flight Alabaster | W | rare | 1.8 | 6 | 5 |
| Chapel Geist | W | common | 1.9 | 33 | 24 |
| Daybreak Ranger | G | rare | 2.0 | 9 | 5 |
| Dearly Departed | W | rare | 2.0 | 8 | 8 |
| Rebuke | W | common | 2.2 | 42 | 30 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Tribute to Hunger | B | uncommon | 2.3 | 23 | 21 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Divine Reckoning | W | rare | 2.4 | 5 | 0 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Geistflame | R | common | 2.5 | 42 | 29 |
