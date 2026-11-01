# Draft loop: what the nights add up to

12 night(s), 96 decks, 350 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 40 | 54-66-0 | 45% | 135-154 | 47% |
| unguided | 56 | 90-78-0 | 54% | 215-196 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 19 | 33-24-0 | 55% (137) |
| WU | 14 | 24-18-0 | 55% (102) |
| WR | 11 | 18-15-0 | 54% (78) |
| BR | 10 | 18-12-0 | 55% (69) |
| UB | 10 | 10-20-0 | 44% (78) |
| UR | 10 | 13-17-0 | 45% (74) |
| WG | 8 | 12-12-0 | 48% (58) |
| WB | 5 | 5-10-0 | 36% (36) |
| RG | 3 | 4-5-0 | 45% (22) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 24 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Spider Spawning | G | uncommon | 10 | 74 | 65% | 3.2 | 17/54 |
| Morkrut Banshee | B | uncommon | 7 | 52 | 63% | 1.5 | 10/15 |
| Hysterical Blindness | U | common | 6 | 42 | 62% | 10.5 | 29/304 |
| Falkenrath Noble | B | uncommon | 9 | 62 | 61% | 1.4 | 14/20 |
| Thraben Sentry | W | common | 11 | 80 | 61% | 8.8 | 17/149 |
| Woodland Sleuth | G | common | 16 | 116 | 60% | 5.0 | 25/126 |
| Gatstaf Shepherd | G | uncommon | 13 | 98 | 60% | 4.4 | 19/83 |
| Slayer of the Wicked | W | uncommon | 11 | 77 | 60% | 1.6 | 14/22 |
| Villagers of Estwald | G | common | 13 | 97 | 59% | 3.1 | 33/101 |
| Unruly Mob | W | common | 12 | 87 | 59% | 11.0 | 24/265 |
| Cackling Counterpart | U | rare | 4 | 29 | 59% | 9.2 | 9/83 |
| Galvanic Juggernaut | C | uncommon | 14 | 103 | 58% | 1.3 | 14/18 |
| Typhoid Rats | B | common | 19 | 142 | 58% | 3.4 | 25/84 |
| Woodland Cemetery | C | rare | 4 | 26 | 58% | 7.9 | 7/55 |
| Into the Maw of Hell | R | uncommon | 15 | 110 | 57% | 3.7 | 16/59 |
| Hamlet Captain | G | uncommon | 11 | 82 | 57% | 5.9 | 15/88 |
| Furor of the Bitten | R | common | 5 | 35 | 57% | 11.9 | 27/320 |
| Dearly Departed | W | rare | 4 | 28 | 57% | 2.5 | 4/10 |
| Ashmouth Hound | R | common | 14 | 101 | 56% | 3.8 | 24/90 |
| Prey Upon | G | common | 16 | 114 | 56% | 2.3 | 23/54 |
| Midnight Haunting | W | uncommon | 11 | 77 | 56% | 4.2 | 13/55 |
| Lumberknot | G | uncommon | 12 | 88 | 56% | 3.9 | 14/54 |
| Skirsdag Cultist | R | uncommon | 6 | 45 | 56% | 8.2 | 12/98 |
| Selfless Cathar | W | common | 10 | 74 | 55% | 10.5 | 24/252 |
| Avacynian Priest | W | common | 18 | 130 | 55% | 5.2 | 25/129 |
| Elite Inquisitor | W | rare | 4 | 29 | 55% | 3.4 | 5/17 |
| Curse of the Pierced Heart | R | common | 7 | 49 | 55% | 9.2 | 25/231 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.7 | 18/139 |
| Silver-Inlaid Dagger | C | uncommon | 9 | 64 | 55% | 7.8 | 15/117 |
| Ghoulcaller's Chant | B | common | 16 | 117 | 55% | 7.8 | 25/194 |
| Avacyn's Pilgrim | G | common | 6 | 44 | 55% | 10.3 | 27/277 |
| Silverchase Fox | W | common | 16 | 116 | 54% | 7.4 | 26/193 |
| Invisible Stalker | U | uncommon | 13 | 94 | 54% | 5.6 | 16/90 |
| Bonds of Faith | W | common | 16 | 118 | 54% | 4.3 | 26/111 |
| Manor Gargoyle | C | rare | 5 | 37 | 54% | 2.4 | 5/12 |
| Kruin Outlaw | R | rare | 7 | 50 | 54% | 2.1 | 7/15 |
| Somberwald Spider | G | common | 19 | 143 | 54% | 4.2 | 25/104 |
| Dissipate | U | uncommon | 4 | 26 | 54% | 9.0 | 13/117 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 4 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 14 | 14 |
| Bloodgift Demon | B | rare | 1.3 | 3 | 3 |
| Angel of Flight Alabaster | W | rare | 1.3 | 3 | 2 |
| Devil's Play | R | rare | 1.3 | 3 | 2 |
| Falkenrath Noble | B | uncommon | 1.4 | 14 | 9 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 10 | 7 |
| Slayer of the Wicked | W | uncommon | 1.6 | 14 | 11 |
| Daybreak Ranger | G | rare | 1.6 | 5 | 3 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Murder of Crows | U | uncommon | 1.6 | 14 | 12 |
| Fiend Hunter | W | uncommon | 1.6 | 16 | 13 |
| Claustrophobia | U | common | 1.7 | 26 | 18 |
| Chapel Geist | W | common | 1.7 | 22 | 16 |
| Victim of Night | B | common | 1.7 | 23 | 19 |
| Dead Weight | B | common | 1.8 | 25 | 18 |
| Brimstone Volley | R | common | 1.9 | 25 | 15 |
| Kruin Outlaw | R | rare | 2.1 | 7 | 7 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Blasphemous Act | R | rare | 2.2 | 4 | 1 |
| Tribute to Hunger | B | uncommon | 2.2 | 14 | 14 |
| Prey Upon | G | common | 2.3 | 23 | 16 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Manor Gargoyle | C | rare | 2.4 | 5 | 5 |
