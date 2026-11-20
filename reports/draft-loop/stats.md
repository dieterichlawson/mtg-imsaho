# Draft loop: what the nights add up to

31 night(s), 248 decks, 919 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 92 | 134-142-0 | 49% | 328-349 | 48% |
| conventional | 24 | 39-33-0 | 54% | 93-80 | 54% |
| unguided | 132 | 199-197-0 | 50% | 498-490 | 50% |

### The nights with all three (12)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 24 | 33-39-0 | 46% | 78-95 | 45% |
| conventional | 24 | 39-33-0 | 54% | 93-80 | 54% |
| unguided | 48 | 72-72-0 | 50% | 182-178 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 32 | 38-58-0 | 46% (243) |
| BR | 31 | 48-45-0 | 49% (227) |
| WU | 30 | 46-44-0 | 52% (223) |
| BG | 29 | 48-39-0 | 54% (210) |
| WR | 28 | 44-40-0 | 52% (206) |
| WG | 27 | 44-37-0 | 52% (196) |
| UR | 22 | 29-37-0 | 45% (163) |
| WB | 21 | 31-32-0 | 47% (156) |
| RG | 13 | 23-16-0 | 56% (100) |
| UG | 8 | 10-14-0 | 48% (61) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 62 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 9 | 69 | 61% | 1.3 | 11/14 |
| Slayer of the Wicked | W | uncommon | 31 | 223 | 60% | 1.8 | 37/66 |
| Moldgraf Monstrosity | G | rare | 9 | 63 | 59% | 4.3 | 11/47 |
| Falkenrath Marauders | R | rare | 9 | 65 | 58% | 2.3 | 11/25 |
| Midnight Haunting | W | uncommon | 28 | 206 | 58% | 3.8 | 35/134 |
| Stromkirk Noble | R | rare | 12 | 88 | 58% | 4.9 | 13/64 |
| Disciple of Griselbrand | B | uncommon | 19 | 140 | 58% | 9.2 | 43/396 |
| Villagers of Estwald | G | common | 38 | 282 | 57% | 4.6 | 79/362 |
| Desperate Ravings | R | uncommon | 9 | 65 | 57% | 10.3 | 38/392 |
| Mentor of the Meek | W | rare | 13 | 95 | 57% | 3.9 | 14/55 |
| Woodland Sleuth | G | common | 36 | 262 | 56% | 6.2 | 66/411 |
| Furor of the Bitten | R | common | 23 | 168 | 57% | 9.7 | 72/686 |
| Elite Inquisitor | W | rare | 13 | 96 | 56% | 2.8 | 15/42 |
| Gatstaf Shepherd | G | uncommon | 32 | 237 | 56% | 5.0 | 45/227 |
| Grave Bramble | G | common | 29 | 214 | 56% | 8.9 | 65/579 |
| Manor Gargoyle | C | rare | 15 | 110 | 55% | 2.2 | 15/33 |
| Hollowhenge Scavenger | G | uncommon | 20 | 148 | 55% | 4.4 | 30/132 |
| Lumberknot | G | uncommon | 28 | 208 | 55% | 4.4 | 35/154 |
| Bonds of Faith | W | common | 48 | 356 | 55% | 3.9 | 64/251 |
| Avacynian Priest | W | common | 50 | 367 | 55% | 5.6 | 65/364 |
| Ashmouth Hound | R | common | 43 | 317 | 55% | 3.8 | 64/242 |
| Into the Maw of Hell | R | uncommon | 31 | 235 | 55% | 3.4 | 38/128 |
| Galvanic Juggernaut | C | uncommon | 39 | 295 | 55% | 1.2 | 39/47 |
| Spider Spawning | G | uncommon | 24 | 177 | 55% | 3.8 | 40/151 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.6 | 16/41 |
| Voiceless Spirit | W | common | 57 | 422 | 55% | 3.5 | 66/234 |
| Avacyn's Pilgrim | G | common | 21 | 154 | 54% | 9.0 | 69/622 |
| Festerhide Boar | G | common | 43 | 318 | 54% | 6.5 | 70/452 |
| Bloodgift Demon | B | rare | 9 | 65 | 54% | 1.1 | 9/10 |
| Rally the Peasants | W | uncommon | 29 | 209 | 54% | 6.7 | 49/330 |
| Typhoid Rats | B | common | 48 | 347 | 54% | 3.3 | 68/224 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Rage Thrower | R | uncommon | 22 | 162 | 53% | 9.3 | 48/448 |
| Kindercatch | G | common | 42 | 311 | 53% | 7.2 | 65/470 |
| Morkrut Banshee | B | uncommon | 30 | 226 | 53% | 1.7 | 35/60 |
| Unruly Mob | W | common | 43 | 313 | 53% | 9.1 | 64/580 |
| Prey Upon | G | common | 39 | 283 | 53% | 2.8 | 56/159 |
| Hamlet Captain | G | uncommon | 27 | 201 | 53% | 6.9 | 36/250 |
| Victim of Night | B | common | 45 | 330 | 53% | 1.6 | 63/99 |
| Hanweir Watchkeep | R | uncommon | 19 | 146 | 53% | 6.3 | 32/203 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 7 | 3 |
| Reaper from the Abyss | B | mythic | 1.0 | 3 | 3 |
| Bloodgift Demon | B | rare | 1.1 | 9 | 9 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 39 | 39 |
| Geist-Honored Monk | W | rare | 1.2 | 8 | 5 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Olivia Voldaren | BR | mythic | 1.2 | 4 | 0 |
| Sever the Bloodline | B | rare | 1.3 | 11 | 9 |
| Balefire Dragon | R | mythic | 1.4 | 5 | 4 |
| Fiend Hunter | W | uncommon | 1.5 | 38 | 30 |
| Victim of Night | B | common | 1.6 | 63 | 45 |
| Devil's Play | R | rare | 1.6 | 10 | 6 |
| Falkenrath Noble | B | uncommon | 1.7 | 36 | 28 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 70 | 48 |
| Morkrut Banshee | B | uncommon | 1.7 | 35 | 30 |
| Slayer of the Wicked | W | uncommon | 1.8 | 37 | 31 |
| Skaab Ruinator | U | mythic | 1.8 | 5 | 4 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 55 | 40 |
| Claustrophobia | U | common | 2.0 | 66 | 45 |
| Brimstone Volley | R | common | 2.0 | 64 | 41 |
| Murder of Crows | U | uncommon | 2.1 | 36 | 32 |
| Dearly Departed | W | rare | 2.1 | 14 | 10 |
| Manor Gargoyle | C | rare | 2.2 | 15 | 15 |
| Instigator Gang | R | rare | 2.2 | 8 | 6 |
| Rebuke | W | common | 2.3 | 72 | 48 |
| Falkenrath Marauders | R | rare | 2.3 | 11 | 9 |
