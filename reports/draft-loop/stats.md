# Draft loop: what the nights add up to

28 night(s), 224 decks, 828 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 86 | 125-133-0 | 48% | 308-327 | 49% |
| conventional | 18 | 26-28-0 | 48% | 62-64 | 49% |
| unguided | 120 | 185-175-0 | 51% | 458-437 | 51% |

### The nights with all three (9)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 18 | 24-30-0 | 44% | 58-73 | 44% |
| conventional | 18 | 26-28-0 | 48% | 62-64 | 49% |
| unguided | 36 | 58-50-0 | 54% | 142-125 | 53% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 29 | 48-39-0 | 54% (210) |
| UB | 28 | 33-51-0 | 45% (212) |
| WU | 28 | 43-41-0 | 52% (207) |
| WR | 27 | 44-37-0 | 54% (200) |
| BR | 26 | 41-37-0 | 49% (188) |
| WG | 22 | 35-31-0 | 51% (156) |
| UR | 20 | 27-33-0 | 46% (149) |
| WB | 19 | 27-30-0 | 45% (143) |
| RG | 12 | 20-16-0 | 54% (92) |
| UG | 6 | 7-11-0 | 46% (46) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 56 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 8 | 57 | 61% | 2.2 | 10/22 |
| Slayer of the Wicked | W | uncommon | 28 | 201 | 60% | 1.7 | 32/55 |
| Midnight Haunting | W | uncommon | 25 | 183 | 60% | 3.9 | 31/122 |
| Mentor of the Meek | W | rare | 11 | 81 | 59% | 3.9 | 12/47 |
| Moldgraf Monstrosity | G | rare | 9 | 63 | 59% | 4.3 | 11/47 |
| Furor of the Bitten | R | common | 21 | 154 | 58% | 9.7 | 65/631 |
| Into the Maw of Hell | R | uncommon | 29 | 221 | 57% | 3.4 | 35/118 |
| Stromkirk Noble | R | rare | 10 | 74 | 57% | 5.0 | 11/55 |
| Lumberknot | G | uncommon | 25 | 184 | 57% | 4.2 | 32/135 |
| Disciple of Griselbrand | B | uncommon | 18 | 133 | 56% | 9.1 | 40/365 |
| Manor Gargoyle | C | rare | 13 | 95 | 56% | 2.2 | 13/28 |
| Ashmouth Hound | R | common | 38 | 280 | 56% | 3.8 | 58/221 |
| Gatstaf Shepherd | G | uncommon | 26 | 192 | 56% | 4.9 | 37/180 |
| Elite Inquisitor | W | rare | 11 | 83 | 55% | 2.8 | 13/37 |
| Villagers of Estwald | G | common | 32 | 235 | 55% | 4.3 | 65/281 |
| Avacynian Priest | W | common | 45 | 330 | 55% | 5.5 | 59/327 |
| Woodland Sleuth | G | common | 32 | 230 | 55% | 6.2 | 60/372 |
| Kruin Outlaw | R | rare | 12 | 89 | 55% | 2.6 | 13/34 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.1 | 15/167 |
| Spider Spawning | G | uncommon | 22 | 162 | 55% | 3.6 | 37/134 |
| Bonds of Faith | W | common | 44 | 326 | 55% | 3.9 | 58/225 |
| Typhoid Rats | B | common | 42 | 303 | 55% | 3.3 | 61/203 |
| Grave Bramble | G | common | 26 | 191 | 54% | 8.9 | 58/517 |
| Hollowhenge Scavenger | G | uncommon | 19 | 140 | 54% | 4.2 | 27/114 |
| Voiceless Spirit | W | common | 53 | 393 | 54% | 3.6 | 60/217 |
| Galvanic Juggernaut | C | uncommon | 34 | 255 | 54% | 1.2 | 34/42 |
| Rally the Peasants | W | uncommon | 24 | 173 | 54% | 6.7 | 43/290 |
| Spectral Flight | U | common | 34 | 260 | 53% | 5.5 | 51/283 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Festerhide Boar | G | common | 38 | 279 | 53% | 6.5 | 63/412 |
| Avacyn's Pilgrim | G | common | 17 | 122 | 53% | 9.2 | 61/560 |
| Morkrut Banshee | B | uncommon | 29 | 218 | 53% | 1.7 | 33/56 |
| Hamlet Captain | G | uncommon | 25 | 185 | 53% | 6.9 | 33/229 |
| Battleground Geist | U | uncommon | 27 | 204 | 53% | 3.4 | 31/104 |
| Kindercatch | G | common | 38 | 280 | 53% | 7.1 | 58/413 |
| Mindshrieker | U | rare | 10 | 72 | 53% | 6.2 | 13/80 |
| Unruly Mob | W | common | 39 | 283 | 53% | 9.0 | 58/520 |
| Prey Upon | G | common | 36 | 260 | 53% | 2.8 | 51/144 |
| Hanweir Watchkeep | R | uncommon | 19 | 146 | 53% | 6.3 | 32/203 |
| Spidery Grasp | G | common | 37 | 270 | 53% | 8.4 | 58/488 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 7 | 3 |
| Reaper from the Abyss | B | mythic | 1.0 | 3 | 3 |
| Balefire Dragon | R | mythic | 1.0 | 4 | 3 |
| Olivia Voldaren | BR | mythic | 1.0 | 3 | 0 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 34 | 34 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Skaab Ruinator | U | mythic | 1.2 | 4 | 3 |
| Geist-Honored Monk | W | rare | 1.3 | 7 | 5 |
| Sever the Bloodline | B | rare | 1.3 | 9 | 7 |
| Fiend Hunter | W | uncommon | 1.6 | 34 | 26 |
| Victim of Night | B | common | 1.6 | 57 | 42 |
| Slayer of the Wicked | W | uncommon | 1.7 | 32 | 28 |
| Falkenrath Noble | B | uncommon | 1.7 | 33 | 25 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Morkrut Banshee | B | uncommon | 1.7 | 33 | 29 |
| Devil's Play | R | rare | 1.7 | 9 | 6 |
| Dead Weight | B | common | 1.8 | 64 | 45 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Claustrophobia | U | common | 2.0 | 60 | 40 |
| Chapel Geist | W | common | 2.0 | 50 | 35 |
| Murder of Crows | U | uncommon | 2.0 | 33 | 29 |
| Brimstone Volley | R | common | 2.0 | 58 | 38 |
| Dearly Departed | W | rare | 2.0 | 13 | 9 |
| Rebuke | W | common | 2.2 | 65 | 44 |
| Falkenrath Marauders | R | rare | 2.2 | 10 | 8 |
| Manor Gargoyle | C | rare | 2.2 | 13 | 13 |
| Instigator Gang | R | rare | 2.2 | 8 | 6 |
