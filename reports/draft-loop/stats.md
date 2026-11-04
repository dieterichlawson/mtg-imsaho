# Draft loop: what the nights add up to

15 night(s), 120 decks, 444 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 52 | 76-80-0 | 49% | 190-196 | 49% |
| unguided | 68 | 104-100-0 | 51% | 254-248 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 20 | 34-26-0 | 54% (145) |
| WU | 18 | 28-26-0 | 52% (135) |
| UR | 14 | 17-25-0 | 45% (103) |
| BR | 13 | 24-15-0 | 56% (93) |
| UB | 12 | 13-23-0 | 44% (93) |
| WR | 11 | 18-15-0 | 54% (78) |
| WB | 10 | 14-16-0 | 47% (75) |
| WG | 10 | 17-13-0 | 53% (74) |
| RG | 5 | 6-9-0 | 42% (38) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 30 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Spider Spawning | G | uncommon | 12 | 90 | 66% | 3.2 | 21/67 |
| Morkrut Banshee | B | uncommon | 9 | 68 | 65% | 1.5 | 12/18 |
| Moorland Haunt | C | rare | 4 | 30 | 63% | 11.2 | 6/67 |
| Gatstaf Shepherd | G | uncommon | 14 | 105 | 62% | 4.5 | 21/94 |
| Furor of the Bitten | R | common | 6 | 42 | 62% | 11.3 | 34/383 |
| Mentor of the Meek | W | rare | 5 | 39 | 62% | 5.3 | 6/32 |
| Slayer of the Wicked | W | uncommon | 14 | 102 | 61% | 1.6 | 17/28 |
| Typhoid Rats | B | common | 22 | 164 | 60% | 3.2 | 32/104 |
| Instigator Gang | R | rare | 4 | 30 | 60% | 2.2 | 4/9 |
| Woodland Sleuth | G | common | 19 | 139 | 60% | 5.1 | 33/169 |
| Skirsdag Cultist | R | uncommon | 7 | 52 | 60% | 8.0 | 13/104 |
| Silver-Inlaid Dagger | C | uncommon | 14 | 101 | 59% | 7.0 | 21/147 |
| Thraben Sentry | W | common | 17 | 128 | 59% | 8.3 | 26/217 |
| Unruly Mob | W | common | 17 | 126 | 59% | 10.3 | 30/309 |
| Selfless Cathar | W | common | 14 | 106 | 58% | 10.2 | 30/305 |
| Manor Gargoyle | C | rare | 7 | 53 | 58% | 2.6 | 7/18 |
| Villagers of Estwald | G | common | 16 | 120 | 58% | 3.2 | 38/122 |
| Avacyn's Pilgrim | G | common | 8 | 60 | 58% | 9.5 | 35/333 |
| Midnight Haunting | W | uncommon | 14 | 102 | 58% | 4.5 | 16/72 |
| Prey Upon | G | common | 18 | 130 | 58% | 2.5 | 26/65 |
| Elite Inquisitor | W | rare | 5 | 37 | 57% | 3.0 | 7/21 |
| Bonds of Faith | W | common | 21 | 159 | 57% | 4.2 | 32/136 |
| Into the Maw of Hell | R | uncommon | 17 | 125 | 56% | 3.6 | 20/72 |
| Woodland Cemetery | C | rare | 5 | 34 | 56% | 8.7 | 11/96 |
| Ghoulcaller's Chant | B | common | 21 | 156 | 55% | 8.0 | 32/257 |
| Bump in the Night | B | common | 18 | 131 | 55% | 8.4 | 34/286 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.6 | 21/159 |
| Curse of the Pierced Heart | R | common | 10 | 71 | 55% | 8.8 | 32/283 |
| Invisible Stalker | U | uncommon | 16 | 119 | 55% | 5.3 | 19/100 |
| Ashmouth Hound | R | common | 20 | 147 | 54% | 3.9 | 31/120 |
| Silverchase Fox | W | common | 20 | 147 | 54% | 7.3 | 32/234 |
| Falkenrath Noble | B | uncommon | 13 | 94 | 54% | 1.6 | 18/28 |
| Lumberknot | G | uncommon | 13 | 96 | 54% | 3.9 | 17/66 |
| Kruin Outlaw | R | rare | 10 | 74 | 54% | 2.5 | 10/25 |
| Victim of Night | B | common | 23 | 159 | 54% | 1.7 | 30/51 |
| Hysterical Blindness | U | common | 10 | 76 | 54% | 9.6 | 36/344 |
| Hamlet Captain | G | uncommon | 15 | 115 | 54% | 5.9 | 19/113 |
| Traitorous Blood | R | common | 7 | 52 | 54% | 10.0 | 35/349 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 4 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 18 | 18 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 5 | 5 |
| Devil's Play | R | rare | 1.2 | 4 | 3 |
| Daybreak Ranger | G | rare | 1.5 | 6 | 3 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 12 | 9 |
| Slayer of the Wicked | W | uncommon | 1.6 | 17 | 14 |
| Falkenrath Noble | B | uncommon | 1.6 | 18 | 13 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Murder of Crows | U | uncommon | 1.6 | 18 | 16 |
| Fiend Hunter | W | uncommon | 1.6 | 19 | 16 |
| Victim of Night | B | common | 1.7 | 30 | 23 |
| Claustrophobia | U | common | 1.8 | 32 | 23 |
| Chapel Geist | W | common | 1.8 | 27 | 20 |
| Dead Weight | B | common | 1.8 | 33 | 25 |
| Brimstone Volley | R | common | 1.8 | 31 | 18 |
| Angel of Flight Alabaster | W | rare | 1.8 | 4 | 3 |
| Rebuke | W | common | 2.2 | 34 | 25 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Tribute to Hunger | B | uncommon | 2.2 | 20 | 19 |
| Dearly Departed | W | rare | 2.2 | 6 | 6 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Geistcatcher's Rig | C | uncommon | 2.4 | 18 | 16 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
