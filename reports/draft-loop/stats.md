# Draft loop: what the nights add up to

13 night(s), 104 decks, 379 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 44 | 62-70-0 | 47% | 153-167 | 48% |
| unguided | 60 | 94-86-0 | 52% | 226-212 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 19 | 33-24-0 | 55% (137) |
| WU | 15 | 25-20-0 | 54% (108) |
| UR | 12 | 16-20-0 | 47% (88) |
| BR | 11 | 21-12-0 | 58% (76) |
| UB | 11 | 11-22-0 | 44% (85) |
| WR | 11 | 18-15-0 | 54% (78) |
| WG | 9 | 14-13-0 | 49% (67) |
| WB | 6 | 5-13-0 | 33% (43) |
| RG | 3 | 4-5-0 | 45% (22) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 26 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Morkrut Banshee | B | uncommon | 8 | 59 | 66% | 1.5 | 11/16 |
| Spider Spawning | G | uncommon | 11 | 83 | 64% | 3.2 | 18/57 |
| Furor of the Bitten | R | common | 6 | 42 | 62% | 11.6 | 30/348 |
| Mentor of the Meek | W | rare | 4 | 31 | 61% | 5.8 | 5/29 |
| Gatstaf Shepherd | G | uncommon | 13 | 98 | 60% | 4.4 | 19/83 |
| Woodland Sleuth | G | common | 17 | 124 | 60% | 4.9 | 27/131 |
| Skirsdag Cultist | R | uncommon | 7 | 52 | 60% | 8.0 | 13/104 |
| Slayer of the Wicked | W | uncommon | 12 | 86 | 59% | 1.7 | 15/26 |
| Hysterical Blindness | U | common | 7 | 49 | 59% | 10.0 | 32/319 |
| Ashmouth Hound | R | common | 16 | 115 | 59% | 3.8 | 26/99 |
| Typhoid Rats | B | common | 20 | 149 | 59% | 3.2 | 28/91 |
| Thraben Sentry | W | common | 13 | 95 | 59% | 8.8 | 20/176 |
| Cackling Counterpart | U | rare | 4 | 29 | 59% | 9.4 | 10/94 |
| Villagers of Estwald | G | common | 14 | 105 | 58% | 3.1 | 34/105 |
| Galvanic Juggernaut | C | uncommon | 15 | 111 | 58% | 1.3 | 15/19 |
| Curse of the Pierced Heart | R | common | 9 | 63 | 57% | 9.1 | 28/256 |
| Stromkirk Noble | R | rare | 4 | 28 | 57% | 6.5 | 4/26 |
| Unruly Mob | W | common | 14 | 102 | 57% | 10.7 | 26/279 |
| Hanweir Watchkeep | R | uncommon | 8 | 58 | 57% | 6.6 | 13/86 |
| Hamlet Captain | G | uncommon | 13 | 99 | 57% | 5.8 | 17/98 |
| Falkenrath Noble | B | uncommon | 10 | 69 | 57% | 1.6 | 15/24 |
| Into the Maw of Hell | R | uncommon | 16 | 117 | 56% | 3.6 | 18/64 |
| Silver-Inlaid Dagger | C | uncommon | 11 | 78 | 56% | 7.6 | 17/130 |
| Kruin Outlaw | R | rare | 8 | 57 | 56% | 2.1 | 8/17 |
| Prey Upon | G | common | 17 | 123 | 56% | 2.5 | 24/60 |
| Woodland Cemetery | C | rare | 5 | 34 | 56% | 8.4 | 9/76 |
| Midnight Haunting | W | uncommon | 12 | 86 | 56% | 4.4 | 14/62 |
| Bump in the Night | B | common | 15 | 106 | 56% | 8.8 | 29/255 |
| Lumberknot | G | uncommon | 12 | 88 | 56% | 3.9 | 15/59 |
| Frightful Delusion | U | common | 9 | 63 | 56% | 9.8 | 28/273 |
| Selfless Cathar | W | common | 11 | 83 | 55% | 10.4 | 26/271 |
| Invisible Stalker | U | uncommon | 14 | 101 | 55% | 5.5 | 17/94 |
| Elite Inquisitor | W | rare | 4 | 29 | 55% | 3.3 | 6/20 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.7 | 18/139 |
| Avacyn's Pilgrim | G | common | 7 | 53 | 55% | 10.1 | 29/293 |
| Bonds of Faith | W | common | 17 | 127 | 54% | 4.3 | 28/121 |
| Manor Gargoyle | C | rare | 6 | 46 | 54% | 2.5 | 6/15 |
| Ghoulcaller's Chant | B | common | 17 | 124 | 54% | 8.0 | 27/216 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 4 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Devil's Play | R | rare | 1.2 | 4 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 15 | 15 |
| Bloodgift Demon | B | rare | 1.3 | 3 | 3 |
| Angel of Flight Alabaster | W | rare | 1.3 | 3 | 2 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 11 | 8 |
| Daybreak Ranger | G | rare | 1.6 | 5 | 3 |
| Falkenrath Noble | B | uncommon | 1.6 | 15 | 10 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Murder of Crows | U | uncommon | 1.6 | 16 | 14 |
| Fiend Hunter | W | uncommon | 1.6 | 18 | 15 |
| Claustrophobia | U | common | 1.7 | 28 | 19 |
| Slayer of the Wicked | W | uncommon | 1.7 | 15 | 12 |
| Chapel Geist | W | common | 1.7 | 23 | 17 |
| Dead Weight | B | common | 1.8 | 28 | 21 |
| Victim of Night | B | common | 1.8 | 26 | 21 |
| Brimstone Volley | R | common | 1.9 | 27 | 16 |
| Kruin Outlaw | R | rare | 2.1 | 8 | 8 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Blasphemous Act | R | rare | 2.2 | 4 | 1 |
| Tribute to Hunger | B | uncommon | 2.3 | 16 | 16 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Instigator Gang | R | rare | 2.3 | 3 | 3 |
| Rebuke | W | common | 2.4 | 28 | 20 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
