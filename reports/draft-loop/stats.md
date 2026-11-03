# Draft loop: what the nights add up to

14 night(s), 112 decks, 411 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 48 | 70-74-0 | 49% | 172-181 | 49% |
| unguided | 64 | 98-94-0 | 51% | 239-230 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 19 | 33-24-0 | 55% (137) |
| WU | 16 | 27-21-0 | 54% (117) |
| UR | 14 | 17-25-0 | 45% (103) |
| BR | 12 | 22-14-0 | 56% (84) |
| UB | 11 | 11-22-0 | 44% (85) |
| WR | 11 | 18-15-0 | 54% (78) |
| WG | 10 | 17-13-0 | 53% (74) |
| WB | 8 | 9-15-0 | 40% (60) |
| RG | 4 | 5-7-0 | 43% (30) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 28 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Spider Spawning | G | uncommon | 12 | 90 | 66% | 3.3 | 20/66 |
| Morkrut Banshee | B | uncommon | 9 | 68 | 65% | 1.5 | 12/18 |
| Gatstaf Shepherd | G | uncommon | 14 | 105 | 62% | 4.5 | 21/94 |
| Furor of the Bitten | R | common | 6 | 42 | 62% | 11.5 | 32/369 |
| Mentor of the Meek | W | rare | 4 | 31 | 61% | 5.8 | 5/29 |
| Woodland Sleuth | G | common | 19 | 139 | 60% | 4.7 | 30/141 |
| Skirsdag Cultist | R | uncommon | 7 | 52 | 60% | 8.0 | 13/104 |
| Typhoid Rats | B | common | 21 | 157 | 59% | 3.3 | 30/98 |
| Slayer of the Wicked | W | uncommon | 13 | 95 | 59% | 1.7 | 16/27 |
| Silver-Inlaid Dagger | C | uncommon | 12 | 85 | 59% | 7.4 | 19/140 |
| Thraben Sentry | W | common | 14 | 104 | 59% | 8.5 | 22/187 |
| Hysterical Blindness | U | common | 8 | 58 | 59% | 9.8 | 34/332 |
| Cackling Counterpart | U | rare | 4 | 29 | 59% | 9.7 | 11/107 |
| Villagers of Estwald | G | common | 16 | 120 | 58% | 3.2 | 37/117 |
| Avacyn's Pilgrim | G | common | 8 | 60 | 58% | 9.6 | 32/308 |
| Selfless Cathar | W | common | 12 | 90 | 58% | 10.4 | 28/291 |
| Prey Upon | G | common | 18 | 130 | 58% | 2.5 | 25/63 |
| Unruly Mob | W | common | 15 | 111 | 57% | 10.6 | 27/286 |
| Falkenrath Noble | B | uncommon | 11 | 78 | 56% | 1.6 | 16/25 |
| Kruin Outlaw | R | rare | 8 | 57 | 56% | 2.1 | 8/17 |
| Into the Maw of Hell | R | uncommon | 17 | 125 | 56% | 3.6 | 19/68 |
| Bonds of Faith | W | common | 19 | 143 | 56% | 4.3 | 30/130 |
| Woodland Cemetery | C | rare | 5 | 34 | 56% | 8.5 | 10/85 |
| Midnight Haunting | W | uncommon | 13 | 95 | 56% | 4.3 | 15/64 |
| Lumberknot | G | uncommon | 12 | 88 | 56% | 3.9 | 16/63 |
| Frightful Delusion | U | common | 9 | 63 | 56% | 9.7 | 30/290 |
| Invisible Stalker | U | uncommon | 15 | 110 | 55% | 5.4 | 18/98 |
| Ashmouth Hound | R | common | 18 | 130 | 55% | 3.7 | 28/104 |
| Elite Inquisitor | W | rare | 4 | 29 | 55% | 3.3 | 6/20 |
| Hamlet Captain | G | uncommon | 14 | 107 | 55% | 5.8 | 18/104 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.6 | 20/152 |
| Curse of the Pierced Heart | R | common | 10 | 71 | 55% | 8.8 | 30/264 |
| Galvanic Juggernaut | C | uncommon | 17 | 126 | 55% | 1.2 | 17/21 |
| Bump in the Night | B | common | 17 | 123 | 54% | 8.5 | 32/273 |
| Hanweir Watchkeep | R | uncommon | 9 | 66 | 55% | 6.9 | 14/96 |
| Manor Gargoyle | C | rare | 6 | 46 | 54% | 2.5 | 6/15 |
| Ghoulcaller's Chant | B | common | 18 | 133 | 54% | 8.2 | 29/237 |
| Kindercatch | G | common | 20 | 148 | 54% | 6.5 | 29/189 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 4 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 17 | 17 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 5 | 5 |
| Devil's Play | R | rare | 1.2 | 4 | 3 |
| Daybreak Ranger | G | rare | 1.5 | 6 | 3 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 12 | 9 |
| Falkenrath Noble | B | uncommon | 1.6 | 16 | 11 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Murder of Crows | U | uncommon | 1.6 | 17 | 15 |
| Fiend Hunter | W | uncommon | 1.6 | 18 | 15 |
| Slayer of the Wicked | W | uncommon | 1.7 | 16 | 13 |
| Chapel Geist | W | common | 1.7 | 25 | 19 |
| Claustrophobia | U | common | 1.8 | 30 | 21 |
| Dead Weight | B | common | 1.8 | 30 | 23 |
| Brimstone Volley | R | common | 1.8 | 29 | 18 |
| Victim of Night | B | common | 1.8 | 28 | 22 |
| Angel of Flight Alabaster | W | rare | 1.8 | 4 | 3 |
| Kruin Outlaw | R | rare | 2.1 | 8 | 8 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Rebuke | W | common | 2.3 | 31 | 22 |
| Tribute to Hunger | B | uncommon | 2.3 | 18 | 17 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Instigator Gang | R | rare | 2.3 | 3 | 3 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Dearly Departed | W | rare | 2.4 | 5 | 5 |
