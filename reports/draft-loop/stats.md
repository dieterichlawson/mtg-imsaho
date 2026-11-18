# Draft loop: what the nights add up to

29 night(s), 232 decks, 858 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 88 | 128-136-0 | 48% | 314-334 | 48% |
| conventional | 20 | 30-30-0 | 50% | 72-69 | 51% |
| unguided | 124 | 190-182-0 | 51% | 472-455 | 51% |

### The nights with all three (10)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 20 | 27-33-0 | 45% | 64-80 | 44% |
| conventional | 20 | 30-30-0 | 50% | 72-69 | 51% |
| unguided | 40 | 63-57-0 | 52% | 156-143 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 29 | 48-39-0 | 54% (210) |
| UB | 29 | 34-53-0 | 45% (220) |
| BR | 28 | 45-39-0 | 50% (204) |
| WR | 28 | 44-40-0 | 52% (206) |
| WU | 28 | 43-41-0 | 52% (207) |
| WG | 23 | 37-32-0 | 51% (164) |
| UR | 21 | 28-35-0 | 46% (156) |
| WB | 20 | 30-30-0 | 47% (150) |
| RG | 12 | 20-16-0 | 54% (92) |
| UG | 7 | 8-13-0 | 44% (54) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 58 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Slayer of the Wicked | W | uncommon | 29 | 208 | 61% | 1.8 | 34/60 |
| Moldgraf Monstrosity | G | rare | 9 | 63 | 59% | 4.3 | 11/47 |
| Stromkirk Noble | R | rare | 11 | 81 | 58% | 4.9 | 12/59 |
| Disciple of Griselbrand | B | uncommon | 19 | 140 | 58% | 9.1 | 41/375 |
| Elite Inquisitor | W | rare | 12 | 90 | 58% | 2.8 | 14/39 |
| Midnight Haunting | W | uncommon | 26 | 189 | 58% | 3.8 | 33/127 |
| Avacynian Priest | W | common | 47 | 345 | 56% | 5.5 | 61/337 |
| Manor Gargoyle | C | rare | 14 | 104 | 56% | 2.2 | 14/31 |
| Lumberknot | G | uncommon | 26 | 192 | 56% | 4.2 | 33/140 |
| Villagers of Estwald | G | common | 33 | 243 | 56% | 4.5 | 69/308 |
| Furor of the Bitten | R | common | 22 | 160 | 56% | 9.6 | 67/646 |
| Into the Maw of Hell | R | uncommon | 30 | 227 | 56% | 3.4 | 36/121 |
| Gatstaf Shepherd | G | uncommon | 28 | 206 | 55% | 4.8 | 40/193 |
| Woodland Sleuth | G | common | 32 | 230 | 55% | 6.3 | 62/389 |
| Typhoid Rats | B | common | 44 | 317 | 55% | 3.3 | 63/208 |
| Mentor of the Meek | W | rare | 12 | 87 | 55% | 4.2 | 13/54 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.1 | 15/167 |
| Voiceless Spirit | W | common | 54 | 400 | 55% | 3.6 | 62/221 |
| Ashmouth Hound | R | common | 40 | 295 | 55% | 3.8 | 60/225 |
| Bonds of Faith | W | common | 46 | 339 | 55% | 3.9 | 60/231 |
| Grave Bramble | G | common | 26 | 191 | 54% | 9.0 | 60/538 |
| Galvanic Juggernaut | C | uncommon | 36 | 272 | 54% | 1.2 | 36/44 |
| Hollowhenge Scavenger | G | uncommon | 19 | 140 | 54% | 4.2 | 28/117 |
| Spider Spawning | G | uncommon | 23 | 170 | 54% | 3.7 | 38/141 |
| Avacyn's Pilgrim | G | common | 18 | 130 | 54% | 9.1 | 64/585 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Hamlet Captain | G | uncommon | 26 | 193 | 53% | 6.9 | 34/235 |
| Rally the Peasants | W | uncommon | 26 | 186 | 53% | 6.8 | 45/305 |
| Kruin Outlaw | R | rare | 13 | 96 | 53% | 2.6 | 14/36 |
| Morkrut Banshee | B | uncommon | 30 | 226 | 53% | 1.7 | 34/58 |
| Festerhide Boar | G | common | 39 | 287 | 53% | 6.5 | 66/428 |
| Prey Upon | G | common | 37 | 268 | 53% | 2.8 | 53/151 |
| Unruly Mob | W | common | 40 | 291 | 53% | 8.9 | 60/537 |
| Mindshrieker | U | rare | 10 | 72 | 53% | 6.2 | 13/80 |
| Kindercatch | G | common | 40 | 296 | 53% | 7.2 | 61/441 |
| Victim of Night | B | common | 43 | 315 | 53% | 1.6 | 59/94 |
| Spectral Flight | U | common | 36 | 275 | 53% | 5.6 | 54/302 |
| Hanweir Watchkeep | R | uncommon | 19 | 146 | 53% | 6.3 | 32/203 |
| Battleground Geist | U | uncommon | 28 | 212 | 52% | 3.4 | 32/108 |
| Grizzled Outcasts | G | common | 24 | 168 | 52% | 7.5 | 54/406 |

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
| Galvanic Juggernaut | C | uncommon | 1.2 | 36 | 36 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Skaab Ruinator | U | mythic | 1.2 | 4 | 3 |
| Geist-Honored Monk | W | rare | 1.3 | 7 | 5 |
| Sever the Bloodline | B | rare | 1.3 | 9 | 7 |
| Fiend Hunter | W | uncommon | 1.6 | 36 | 28 |
| Victim of Night | B | common | 1.6 | 59 | 43 |
| Falkenrath Noble | B | uncommon | 1.7 | 34 | 26 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 66 | 46 |
| Morkrut Banshee | B | uncommon | 1.7 | 34 | 30 |
| Devil's Play | R | rare | 1.7 | 9 | 6 |
| Slayer of the Wicked | W | uncommon | 1.8 | 34 | 29 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 52 | 37 |
| Claustrophobia | U | common | 2.0 | 62 | 41 |
| Murder of Crows | U | uncommon | 2.0 | 34 | 30 |
| Brimstone Volley | R | common | 2.0 | 60 | 39 |
| Dearly Departed | W | rare | 2.0 | 13 | 9 |
| Rebuke | W | common | 2.2 | 67 | 45 |
| Falkenrath Marauders | R | rare | 2.2 | 10 | 8 |
| Manor Gargoyle | C | rare | 2.2 | 14 | 14 |
| Instigator Gang | R | rare | 2.2 | 8 | 6 |
