# Draft loop: what the nights add up to

32 night(s), 256 decks, 950 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 94 | 137-145-0 | 49% | 336-357 | 48% |
| conventional | 26 | 41-37-0 | 53% | 99-88 | 53% |
| unguided | 136 | 206-202-0 | 50% | 515-505 | 50% |

### The nights with all three (13)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 26 | 36-42-0 | 46% | 86-103 | 46% |
| conventional | 26 | 41-37-0 | 53% | 99-88 | 53% |
| unguided | 52 | 79-77-0 | 51% | 199-193 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 33 | 39-60-0 | 46% (252) |
| BR | 32 | 49-47-0 | 49% (234) |
| WU | 31 | 49-44-0 | 52% (232) |
| WR | 30 | 47-43-0 | 52% (221) |
| BG | 29 | 48-39-0 | 54% (210) |
| WG | 27 | 44-37-0 | 52% (196) |
| UR | 22 | 29-37-0 | 45% (163) |
| WB | 22 | 32-34-0 | 47% (163) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 9 | 12-15-0 | 49% (68) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 64 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 9 | 69 | 61% | 1.3 | 11/14 |
| Slayer of the Wicked | W | uncommon | 32 | 232 | 60% | 1.8 | 38/68 |
| Midnight Haunting | W | uncommon | 29 | 215 | 59% | 3.8 | 36/136 |
| Falkenrath Marauders | R | rare | 9 | 65 | 58% | 2.3 | 11/25 |
| Disciple of Griselbrand | B | uncommon | 19 | 140 | 58% | 9.2 | 43/396 |
| Mentor of the Meek | W | rare | 14 | 104 | 58% | 3.7 | 15/56 |
| Desperate Ravings | R | uncommon | 9 | 65 | 57% | 10.1 | 39/394 |
| Elite Inquisitor | W | rare | 14 | 104 | 57% | 2.7 | 16/43 |
| Villagers of Estwald | G | common | 40 | 297 | 57% | 4.5 | 83/377 |
| Stromkirk Noble | R | rare | 13 | 96 | 56% | 4.6 | 14/65 |
| Woodland Sleuth | G | common | 38 | 277 | 56% | 6.2 | 69/426 |
| Furor of the Bitten | R | common | 25 | 184 | 56% | 9.7 | 74/706 |
| Lumberknot | G | uncommon | 29 | 215 | 55% | 4.4 | 36/158 |
| Avacynian Priest | W | common | 52 | 383 | 55% | 5.6 | 67/372 |
| Gatstaf Shepherd | G | uncommon | 33 | 245 | 55% | 5.1 | 48/245 |
| Manor Gargoyle | C | rare | 17 | 127 | 55% | 2.2 | 17/38 |
| Bonds of Faith | W | common | 50 | 371 | 55% | 4.0 | 66/262 |
| Grave Bramble | G | common | 30 | 222 | 55% | 8.9 | 67/596 |
| Into the Maw of Hell | R | uncommon | 33 | 250 | 55% | 3.4 | 40/134 |
| Spider Spawning | G | uncommon | 24 | 177 | 55% | 3.8 | 40/151 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.6 | 16/41 |
| Galvanic Juggernaut | C | uncommon | 41 | 311 | 55% | 1.2 | 41/49 |
| Hollowhenge Scavenger | G | uncommon | 21 | 156 | 54% | 4.2 | 32/136 |
| Ashmouth Hound | R | common | 45 | 331 | 54% | 3.8 | 67/252 |
| Voiceless Spirit | W | common | 59 | 436 | 54% | 3.5 | 68/240 |
| Avacyn's Pilgrim | G | common | 21 | 154 | 54% | 9.1 | 71/644 |
| Festerhide Boar | G | common | 44 | 325 | 54% | 6.5 | 72/470 |
| Bloodgift Demon | B | rare | 9 | 65 | 54% | 1.1 | 9/10 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Rally the Peasants | W | uncommon | 30 | 216 | 53% | 6.8 | 50/339 |
| Typhoid Rats | B | common | 50 | 363 | 53% | 3.4 | 71/238 |
| Kindercatch | G | common | 43 | 318 | 53% | 7.3 | 67/487 |
| Morkrut Banshee | B | uncommon | 30 | 226 | 53% | 1.7 | 37/63 |
| Rage Thrower | R | uncommon | 23 | 169 | 53% | 9.3 | 49/457 |
| Doomed Traveler | W | common | 51 | 374 | 53% | 7.0 | 70/490 |
| Hanweir Watchkeep | R | uncommon | 19 | 146 | 53% | 6.3 | 32/203 |
| Unruly Mob | W | common | 45 | 327 | 53% | 9.1 | 67/607 |
| Prey Upon | G | common | 41 | 297 | 53% | 2.8 | 58/162 |
| Grizzled Outcasts | G | common | 24 | 168 | 52% | 7.5 | 54/406 |
| Hysterical Blindness | U | common | 15 | 109 | 52% | 10.3 | 72/739 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 7 | 3 |
| Reaper from the Abyss | B | mythic | 1.0 | 3 | 3 |
| Bloodgift Demon | B | rare | 1.1 | 9 | 9 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 41 | 41 |
| Geist-Honored Monk | W | rare | 1.2 | 8 | 5 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Olivia Voldaren | BR | mythic | 1.2 | 4 | 0 |
| Sever the Bloodline | B | rare | 1.3 | 11 | 9 |
| Balefire Dragon | R | mythic | 1.4 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 40 | 32 |
| Victim of Night | B | common | 1.5 | 66 | 47 |
| Devil's Play | R | rare | 1.6 | 10 | 6 |
| Falkenrath Noble | B | uncommon | 1.7 | 37 | 29 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 72 | 49 |
| Morkrut Banshee | B | uncommon | 1.7 | 37 | 30 |
| Slayer of the Wicked | W | uncommon | 1.8 | 38 | 32 |
| Skaab Ruinator | U | mythic | 1.8 | 5 | 4 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 55 | 40 |
| Brimstone Volley | R | common | 1.9 | 66 | 43 |
| Claustrophobia | U | common | 2.0 | 68 | 47 |
| Murder of Crows | U | uncommon | 2.1 | 37 | 32 |
| Dearly Departed | W | rare | 2.1 | 15 | 11 |
| Manor Gargoyle | C | rare | 2.2 | 17 | 17 |
| Kessig Cagebreakers | G | rare | 2.2 | 15 | 11 |
| Instigator Gang | R | rare | 2.2 | 8 | 6 |
| Daybreak Ranger | G | rare | 2.3 | 18 | 12 |
