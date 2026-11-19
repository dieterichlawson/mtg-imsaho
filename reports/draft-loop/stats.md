# Draft loop: what the nights add up to

30 night(s), 240 decks, 887 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 90 | 132-138-0 | 49% | 322-340 | 49% |
| conventional | 22 | 34-32-0 | 52% | 82-74 | 53% |
| unguided | 128 | 194-190-0 | 51% | 483-473 | 51% |

### The nights with all three (11)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 22 | 31-35-0 | 47% | 72-86 | 46% |
| conventional | 22 | 34-32-0 | 52% | 82-74 | 53% |
| unguided | 44 | 67-65-0 | 51% | 167-161 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BR | 30 | 47-43-0 | 50% (219) |
| BG | 29 | 48-39-0 | 54% (210) |
| UB | 29 | 34-53-0 | 45% (220) |
| WU | 29 | 44-43-0 | 51% (214) |
| WR | 28 | 44-40-0 | 52% (206) |
| WG | 25 | 42-33-0 | 53% (180) |
| UR | 22 | 29-37-0 | 45% (163) |
| WB | 21 | 31-32-0 | 47% (156) |
| RG | 12 | 20-16-0 | 54% (92) |
| UG | 8 | 10-14-0 | 48% (61) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 60 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Slayer of the Wicked | W | uncommon | 30 | 214 | 60% | 1.8 | 36/65 |
| Moldgraf Monstrosity | G | rare | 9 | 63 | 59% | 4.3 | 11/47 |
| Midnight Haunting | W | uncommon | 27 | 197 | 58% | 3.8 | 34/130 |
| Stromkirk Noble | R | rare | 12 | 88 | 58% | 4.9 | 13/64 |
| Disciple of Griselbrand | B | uncommon | 19 | 140 | 58% | 9.2 | 43/396 |
| Desperate Ravings | R | uncommon | 9 | 65 | 57% | 10.3 | 37/382 |
| Villagers of Estwald | G | common | 36 | 266 | 57% | 4.6 | 74/341 |
| Mentor of the Meek | W | rare | 13 | 95 | 57% | 3.9 | 14/55 |
| Gatstaf Shepherd | G | uncommon | 30 | 221 | 57% | 4.9 | 43/210 |
| Elite Inquisitor | W | rare | 13 | 96 | 56% | 2.8 | 15/42 |
| Woodland Sleuth | G | common | 34 | 246 | 56% | 6.3 | 64/403 |
| Lumberknot | G | uncommon | 27 | 200 | 56% | 4.4 | 34/148 |
| Grave Bramble | G | common | 28 | 206 | 56% | 9.0 | 63/565 |
| Furor of the Bitten | R | common | 22 | 160 | 56% | 9.7 | 70/667 |
| Into the Maw of Hell | R | uncommon | 30 | 227 | 56% | 3.4 | 37/124 |
| Manor Gargoyle | C | rare | 15 | 110 | 55% | 2.2 | 15/33 |
| Galvanic Juggernaut | C | uncommon | 38 | 287 | 55% | 1.2 | 38/46 |
| Hollowhenge Scavenger | G | uncommon | 20 | 148 | 55% | 4.4 | 29/127 |
| Avacyn's Pilgrim | G | common | 19 | 138 | 55% | 9.2 | 66/604 |
| Avacynian Priest | W | common | 49 | 358 | 55% | 5.5 | 63/347 |
| Bonds of Faith | W | common | 47 | 347 | 55% | 3.9 | 62/240 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.2 | 16/180 |
| Voiceless Spirit | W | common | 56 | 414 | 55% | 3.6 | 64/230 |
| Spider Spawning | G | uncommon | 24 | 177 | 55% | 3.8 | 40/151 |
| Typhoid Rats | B | common | 46 | 332 | 55% | 3.3 | 65/217 |
| Ashmouth Hound | R | common | 42 | 309 | 54% | 3.8 | 62/233 |
| Festerhide Boar | G | common | 41 | 302 | 54% | 6.5 | 68/441 |
| Rally the Peasants | W | uncommon | 28 | 200 | 54% | 6.8 | 47/321 |
| Prey Upon | G | common | 38 | 275 | 53% | 2.8 | 54/153 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Hamlet Captain | G | uncommon | 26 | 193 | 53% | 6.9 | 34/235 |
| Unruly Mob | W | common | 42 | 305 | 53% | 9.0 | 62/557 |
| Kruin Outlaw | R | rare | 13 | 96 | 53% | 2.5 | 15/38 |
| Kindercatch | G | common | 41 | 303 | 53% | 7.2 | 63/455 |
| Morkrut Banshee | B | uncommon | 30 | 226 | 53% | 1.7 | 35/60 |
| Victim of Night | B | common | 44 | 322 | 53% | 1.6 | 61/96 |
| Silver-Inlaid Dagger | C | uncommon | 23 | 169 | 53% | 6.8 | 33/226 |
| Hanweir Watchkeep | R | uncommon | 19 | 146 | 53% | 6.3 | 32/203 |
| Dearly Departed | W | rare | 10 | 74 | 53% | 2.1 | 14/29 |
| Ambush Viper | G | common | 35 | 255 | 53% | 6.1 | 63/384 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 7 | 3 |
| Reaper from the Abyss | B | mythic | 1.0 | 3 | 3 |
| Balefire Dragon | R | mythic | 1.0 | 4 | 3 |
| Olivia Voldaren | BR | mythic | 1.0 | 3 | 0 |
| Bloodgift Demon | B | rare | 1.1 | 8 | 8 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 38 | 38 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Skaab Ruinator | U | mythic | 1.2 | 4 | 3 |
| Geist-Honored Monk | W | rare | 1.3 | 7 | 5 |
| Sever the Bloodline | B | rare | 1.3 | 9 | 7 |
| Fiend Hunter | W | uncommon | 1.6 | 36 | 28 |
| Victim of Night | B | common | 1.6 | 61 | 44 |
| Devil's Play | R | rare | 1.6 | 10 | 6 |
| Falkenrath Noble | B | uncommon | 1.7 | 35 | 27 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Morkrut Banshee | B | uncommon | 1.7 | 35 | 30 |
| Slayer of the Wicked | W | uncommon | 1.8 | 36 | 30 |
| Dead Weight | B | common | 1.8 | 68 | 47 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 54 | 39 |
| Brimstone Volley | R | common | 1.9 | 62 | 41 |
| Claustrophobia | U | common | 2.0 | 64 | 43 |
| Murder of Crows | U | uncommon | 2.0 | 35 | 31 |
| Dearly Departed | W | rare | 2.1 | 14 | 10 |
| Rebuke | W | common | 2.2 | 69 | 46 |
| Falkenrath Marauders | R | rare | 2.2 | 10 | 8 |
| Tribute to Hunger | B | uncommon | 2.2 | 33 | 29 |
| Manor Gargoyle | C | rare | 2.2 | 15 | 15 |
