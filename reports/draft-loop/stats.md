# Draft loop: what the nights add up to

34 night(s), 272 decks, 1007 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 98 | 142-152-0 | 48% | 349-371 | 48% |
| conventional | 30 | 45-45-0 | 50% | 111-105 | 51% |
| unguided | 144 | 221-211-0 | 51% | 547-531 | 51% |

### The nights with all three (15)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 30 | 41-49-0 | 46% | 99-117 | 46% |
| conventional | 30 | 45-45-0 | 50% | 111-105 | 51% |
| unguided | 60 | 94-86-0 | 52% | 231-219 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 35 | 41-64-0 | 45% (266) |
| BR | 34 | 53-49-0 | 50% (248) |
| WR | 34 | 52-50-0 | 51% (247) |
| WU | 31 | 49-44-0 | 52% (232) |
| BG | 30 | 50-40-0 | 54% (217) |
| WG | 30 | 49-41-0 | 52% (220) |
| UR | 23 | 32-37-0 | 47% (171) |
| WB | 23 | 33-36-0 | 46% (170) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 10 | 14-16-0 | 49% (75) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 68 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 11 | 77 | 60% | 1.2 | 13/16 |
| Disciple of Griselbrand | B | uncommon | 20 | 146 | 60% | 9.2 | 46/423 |
| Desperate Ravings | R | uncommon | 10 | 73 | 59% | 9.8 | 43/423 |
| Slayer of the Wicked | W | uncommon | 35 | 252 | 59% | 1.8 | 41/73 |
| Moldgraf Monstrosity | G | rare | 10 | 70 | 59% | 4.3 | 13/56 |
| Mentor of the Meek | W | rare | 14 | 104 | 58% | 3.7 | 15/56 |
| Falkenrath Marauders | R | rare | 11 | 79 | 57% | 2.1 | 13/27 |
| Elite Inquisitor | W | rare | 14 | 104 | 57% | 2.7 | 16/43 |
| Villagers of Estwald | G | common | 40 | 297 | 57% | 4.5 | 83/377 |
| Midnight Haunting | W | uncommon | 32 | 237 | 57% | 3.6 | 39/140 |
| Stromkirk Noble | R | rare | 13 | 96 | 56% | 4.6 | 14/65 |
| Galvanic Juggernaut | C | uncommon | 43 | 324 | 55% | 1.2 | 43/51 |
| Furor of the Bitten | R | common | 28 | 203 | 55% | 9.6 | 78/741 |
| Lumberknot | G | uncommon | 30 | 223 | 55% | 4.4 | 38/166 |
| Into the Maw of Hell | R | uncommon | 34 | 256 | 55% | 3.3 | 42/138 |
| Woodland Sleuth | G | common | 40 | 292 | 55% | 6.3 | 75/475 |
| Gatstaf Shepherd | G | uncommon | 33 | 245 | 55% | 5.1 | 48/245 |
| Manor Gargoyle | C | rare | 17 | 127 | 55% | 2.2 | 17/38 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.6 | 16/41 |
| Grave Bramble | G | common | 31 | 230 | 55% | 8.9 | 71/629 |
| Bonds of Faith | W | common | 55 | 408 | 54% | 3.9 | 71/278 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.2 | 36/222 |
| Ashmouth Hound | R | common | 47 | 345 | 54% | 3.7 | 72/268 |
| Avacynian Priest | W | common | 57 | 418 | 54% | 5.5 | 72/398 |
| Avacyn's Pilgrim | G | common | 23 | 170 | 54% | 9.0 | 76/687 |
| Thraben Sentry | W | common | 40 | 296 | 54% | 7.6 | 66/500 |
| Voiceless Spirit | W | common | 61 | 449 | 54% | 3.5 | 72/250 |
| Spider Spawning | G | uncommon | 26 | 191 | 53% | 3.8 | 42/159 |
| Kindercatch | G | common | 46 | 342 | 53% | 7.3 | 71/521 |
| Rage Thrower | R | uncommon | 24 | 175 | 53% | 9.2 | 53/486 |
| Festerhide Boar | G | common | 47 | 347 | 53% | 6.6 | 76/502 |
| Hollowhenge Scavenger | G | uncommon | 23 | 170 | 53% | 4.3 | 34/146 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Pitchburn Devils | R | common | 56 | 415 | 53% | 4.7 | 68/321 |
| Morkrut Banshee | B | uncommon | 32 | 241 | 53% | 1.7 | 39/67 |
| Typhoid Rats | B | common | 52 | 378 | 53% | 3.4 | 74/253 |
| Doomed Traveler | W | common | 55 | 402 | 52% | 6.9 | 74/510 |
| Hamlet Captain | G | uncommon | 30 | 225 | 52% | 7.2 | 40/287 |
| Hysterical Blindness | U | common | 15 | 109 | 52% | 10.2 | 76/779 |
| Unruly Mob | W | common | 48 | 346 | 52% | 9.1 | 71/645 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 5 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Reaper from the Abyss | B | mythic | 1.0 | 4 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 12 | 8 |
| Bloodgift Demon | B | rare | 1.1 | 9 | 9 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 43 | 43 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Sever the Bloodline | B | rare | 1.2 | 13 | 11 |
| Olivia Voldaren | BR | mythic | 1.2 | 5 | 1 |
| Balefire Dragon | R | mythic | 1.3 | 6 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 42 | 34 |
| Geist-Honored Monk | W | rare | 1.6 | 11 | 7 |
| Victim of Night | B | common | 1.6 | 72 | 51 |
| Devil's Play | R | rare | 1.6 | 12 | 8 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 76 | 51 |
| Morkrut Banshee | B | uncommon | 1.7 | 39 | 32 |
| Slayer of the Wicked | W | uncommon | 1.8 | 41 | 35 |
| Falkenrath Noble | B | uncommon | 1.8 | 40 | 31 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 59 | 43 |
| Claustrophobia | U | common | 2.0 | 73 | 50 |
| Brimstone Volley | R | common | 2.0 | 69 | 44 |
| Murder of Crows | U | uncommon | 2.1 | 39 | 32 |
| Falkenrath Marauders | R | rare | 2.1 | 13 | 11 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Skaab Ruinator | U | mythic | 2.1 | 7 | 5 |
| Manor Gargoyle | C | rare | 2.2 | 17 | 17 |
| Daybreak Ranger | G | rare | 2.3 | 18 | 12 |
| Tribute to Hunger | B | uncommon | 2.3 | 38 | 32 |
