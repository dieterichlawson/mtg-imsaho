# Draft loop: what the nights add up to

19 night(s), 152 decks, 566 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 68 | 101-103-0 | 50% | 250-254 | 50% |
| unguided | 84 | 127-125-0 | 50% | 316-312 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 23 | 39-30-0 | 54% (167) |
| WU | 20 | 30-30-0 | 51% (150) |
| UB | 19 | 22-35-0 | 45% (148) |
| UR | 16 | 20-28-0 | 45% (119) |
| BR | 15 | 30-15-0 | 59% (109) |
| WR | 15 | 24-21-0 | 54% (109) |
| WB | 14 | 19-23-0 | 45% (105) |
| WG | 14 | 22-20-0 | 50% (103) |
| RG | 8 | 12-12-0 | 50% (60) |
| R | 2 | 3-3-0 | 50% (16) |
| U | 2 | 2-4-0 | 36% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 38 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 6 | 43 | 77% | 2.6 | 7/18 |
| Sever the Bloodline | B | rare | 5 | 38 | 74% | 1.6 | 5/8 |
| Furor of the Bitten | R | common | 10 | 72 | 64% | 10.6 | 41/434 |
| Skirsdag Cultist | R | uncommon | 12 | 91 | 63% | 8.0 | 20/160 |
| Morkrut Banshee | B | uncommon | 16 | 123 | 60% | 1.6 | 19/30 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.5 | 22/231 |
| Elite Inquisitor | W | rare | 8 | 60 | 60% | 2.8 | 10/28 |
| Traitorous Blood | R | common | 10 | 72 | 60% | 9.7 | 45/438 |
| Spider Spawning | G | uncommon | 16 | 119 | 60% | 3.0 | 26/79 |
| Slayer of the Wicked | W | uncommon | 18 | 131 | 60% | 1.7 | 22/37 |
| Typhoid Rats | B | common | 29 | 214 | 59% | 3.3 | 42/138 |
| Stromkirk Noble | R | rare | 7 | 51 | 59% | 5.6 | 7/39 |
| Mentor of the Meek | W | rare | 7 | 53 | 58% | 4.9 | 8/39 |
| Curse of the Pierced Heart | R | common | 14 | 101 | 57% | 8.5 | 41/349 |
| Thraben Sentry | W | common | 23 | 173 | 57% | 7.8 | 35/273 |
| Gatstaf Shepherd | G | uncommon | 19 | 142 | 57% | 4.4 | 27/120 |
| Ashmouth Hound | R | common | 26 | 190 | 57% | 3.8 | 39/149 |
| Midnight Haunting | W | uncommon | 17 | 125 | 57% | 4.4 | 21/93 |
| Bloodgift Demon | B | rare | 6 | 44 | 57% | 1.2 | 6/7 |
| Woodland Sleuth | G | common | 23 | 167 | 56% | 5.3 | 41/219 |
| Into the Maw of Hell | R | uncommon | 22 | 166 | 56% | 3.8 | 25/96 |
| Lumberknot | G | uncommon | 17 | 125 | 56% | 4.0 | 21/84 |
| Unruly Mob | W | common | 24 | 177 | 56% | 9.4 | 39/366 |
| Rally the Peasants | W | uncommon | 12 | 86 | 56% | 7.6 | 26/197 |
| Silver-Inlaid Dagger | C | uncommon | 17 | 123 | 55% | 6.8 | 24/164 |
| Disciple of Griselbrand | B | uncommon | 14 | 103 | 55% | 8.7 | 27/234 |
| Avacyn's Pilgrim | G | common | 11 | 82 | 55% | 9.2 | 45/414 |
| Galvanic Juggernaut | C | uncommon | 22 | 163 | 55% | 1.2 | 22/26 |
| Ghoulcaller's Chant | B | common | 27 | 200 | 54% | 7.7 | 41/316 |
| Bump in the Night | B | common | 22 | 160 | 54% | 8.5 | 42/355 |
| Villagers of Estwald | G | common | 22 | 164 | 54% | 3.7 | 50/187 |
| Falkenrath Noble | B | uncommon | 16 | 116 | 54% | 1.6 | 23/37 |
| Kruin Outlaw | R | rare | 10 | 74 | 54% | 2.6 | 11/29 |
| Bonds of Faith | W | common | 27 | 204 | 54% | 4.1 | 39/158 |
| Prey Upon | G | common | 23 | 167 | 54% | 2.6 | 33/85 |
| Rakish Heir | R | uncommon | 11 | 82 | 54% | 6.4 | 17/109 |
| Woodland Cemetery | C | rare | 6 | 41 | 54% | 9.2 | 14/129 |
| Invisible Stalker | U | uncommon | 20 | 151 | 54% | 4.8 | 25/120 |
| Manor Gargoyle | C | rare | 9 | 69 | 54% | 2.6 | 9/23 |
| Night Revelers | R | common | 25 | 185 | 54% | 7.1 | 33/234 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 6 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 4 | 1 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 22 | 22 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Bloodgift Demon | B | rare | 1.2 | 6 | 6 |
| Devil's Play | R | rare | 1.2 | 5 | 4 |
| Geist-Honored Monk | W | rare | 1.4 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 24 | 20 |
| Falkenrath Noble | B | uncommon | 1.6 | 23 | 16 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Victim of Night | B | common | 1.6 | 39 | 31 |
| Sever the Bloodline | B | rare | 1.6 | 5 | 5 |
| Morkrut Banshee | B | uncommon | 1.6 | 19 | 16 |
| Slayer of the Wicked | W | uncommon | 1.7 | 22 | 18 |
| Dead Weight | B | common | 1.7 | 43 | 31 |
| Claustrophobia | U | common | 1.8 | 41 | 32 |
| Brimstone Volley | R | common | 1.8 | 40 | 24 |
| Angel of Flight Alabaster | W | rare | 1.8 | 6 | 5 |
| Murder of Crows | U | uncommon | 1.9 | 23 | 20 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Dearly Departed | W | rare | 2.0 | 8 | 8 |
| Chapel Geist | W | common | 2.1 | 34 | 24 |
| Instigator Gang | R | rare | 2.2 | 4 | 4 |
| Rebuke | W | common | 2.3 | 44 | 30 |
| Tribute to Hunger | B | uncommon | 2.3 | 23 | 21 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 10 | 5 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Geistflame | R | common | 2.5 | 44 | 31 |
