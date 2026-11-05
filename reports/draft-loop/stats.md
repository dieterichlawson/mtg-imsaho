# Draft loop: what the nights add up to

16 night(s), 128 decks, 475 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 56 | 83-85-0 | 49% | 207-209 | 50% |
| unguided | 72 | 109-107-0 | 50% | 268-266 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 21 | 35-28-0 | 54% (152) |
| WU | 18 | 28-26-0 | 52% (135) |
| UB | 15 | 17-28-0 | 44% (117) |
| UR | 14 | 17-25-0 | 45% (103) |
| BR | 13 | 24-15-0 | 56% (93) |
| WR | 12 | 21-15-0 | 56% (85) |
| WB | 11 | 15-18-0 | 46% (83) |
| WG | 11 | 19-14-0 | 54% (82) |
| RG | 6 | 7-11-0 | 41% (46) |
| R | 2 | 3-3-0 | 50% (16) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 32 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Furor of the Bitten | R | common | 7 | 49 | 65% | 11.1 | 36/400 |
| Mentor of the Meek | W | rare | 6 | 46 | 65% | 5.0 | 7/35 |
| Spider Spawning | G | uncommon | 13 | 97 | 64% | 3.2 | 22/70 |
| Skirsdag Cultist | R | uncommon | 8 | 59 | 63% | 8.1 | 15/122 |
| Gatstaf Shepherd | G | uncommon | 15 | 113 | 62% | 4.5 | 22/98 |
| Elite Inquisitor | W | rare | 6 | 44 | 61% | 2.9 | 8/23 |
| Slayer of the Wicked | W | uncommon | 16 | 117 | 61% | 1.7 | 19/32 |
| Morkrut Banshee | B | uncommon | 11 | 84 | 61% | 1.5 | 14/21 |
| Thraben Sentry | W | common | 20 | 151 | 60% | 8.1 | 29/234 |
| Typhoid Rats | B | common | 24 | 179 | 59% | 3.2 | 34/109 |
| Woodland Sleuth | G | common | 20 | 146 | 59% | 5.2 | 35/181 |
| Avacyn's Pilgrim | G | common | 9 | 68 | 59% | 9.5 | 37/351 |
| Silver-Inlaid Dagger | C | uncommon | 15 | 108 | 58% | 7.0 | 22/153 |
| Unruly Mob | W | common | 19 | 142 | 58% | 10.1 | 32/322 |
| Curse of the Pierced Heart | R | common | 11 | 78 | 58% | 8.9 | 34/303 |
| Villagers of Estwald | G | common | 17 | 127 | 57% | 3.3 | 40/133 |
| Prey Upon | G | common | 20 | 145 | 57% | 2.5 | 28/71 |
| Selfless Cathar | W | common | 15 | 114 | 57% | 10.0 | 32/321 |
| Midnight Haunting | W | uncommon | 15 | 110 | 56% | 4.4 | 19/84 |
| Bonds of Faith | W | common | 23 | 175 | 56% | 4.2 | 34/142 |
| Ashmouth Hound | R | common | 21 | 154 | 56% | 3.9 | 33/129 |
| Manor Gargoyle | C | rare | 8 | 61 | 56% | 2.4 | 8/19 |
| Into the Maw of Hell | R | uncommon | 18 | 133 | 55% | 3.7 | 21/78 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 7.6 | 22/168 |
| Falkenrath Noble | B | uncommon | 13 | 94 | 54% | 1.6 | 19/31 |
| Invisible Stalker | U | uncommon | 17 | 127 | 54% | 5.1 | 21/107 |
| Ghoulcaller's Chant | B | common | 23 | 172 | 54% | 7.9 | 34/269 |
| Kruin Outlaw | R | rare | 10 | 74 | 54% | 2.5 | 10/25 |
| Bump in the Night | B | common | 19 | 139 | 54% | 8.3 | 36/299 |
| Hysterical Blindness | U | common | 10 | 76 | 54% | 9.7 | 38/367 |
| Chapel Geist | W | common | 21 | 152 | 54% | 1.8 | 28/50 |
| Traitorous Blood | R | common | 7 | 52 | 54% | 10.0 | 38/380 |
| Avacynian Priest | W | common | 24 | 177 | 54% | 5.6 | 34/191 |
| Woodland Cemetery | C | rare | 6 | 41 | 54% | 8.7 | 12/104 |
| Silverchase Fox | W | common | 21 | 155 | 54% | 7.5 | 34/255 |
| Lumberknot | G | uncommon | 14 | 103 | 53% | 3.8 | 18/68 |
| Stromkirk Noble | R | rare | 6 | 45 | 53% | 5.0 | 6/30 |
| Galvanic Juggernaut | C | uncommon | 19 | 141 | 53% | 1.2 | 19/23 |
| Kindercatch | G | common | 23 | 171 | 53% | 6.6 | 34/226 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 5 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 19 | 19 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 5 | 5 |
| Devil's Play | R | rare | 1.2 | 4 | 3 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 14 | 11 |
| Daybreak Ranger | G | rare | 1.6 | 7 | 4 |
| Falkenrath Noble | B | uncommon | 1.6 | 19 | 13 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Murder of Crows | U | uncommon | 1.6 | 19 | 16 |
| Fiend Hunter | W | uncommon | 1.6 | 19 | 16 |
| Victim of Night | B | common | 1.6 | 33 | 26 |
| Slayer of the Wicked | W | uncommon | 1.7 | 19 | 16 |
| Dead Weight | B | common | 1.7 | 35 | 27 |
| Claustrophobia | U | common | 1.8 | 35 | 26 |
| Chapel Geist | W | common | 1.8 | 28 | 21 |
| Brimstone Volley | R | common | 1.8 | 33 | 19 |
| Angel of Flight Alabaster | W | rare | 1.8 | 4 | 3 |
| Dearly Departed | W | rare | 2.0 | 7 | 7 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Rebuke | W | common | 2.3 | 36 | 26 |
| Tribute to Hunger | B | uncommon | 2.3 | 21 | 20 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Geistcatcher's Rig | C | uncommon | 2.4 | 19 | 17 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
