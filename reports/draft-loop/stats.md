# Draft loop: what the nights add up to

35 night(s), 280 decks, 1037 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 100 | 143-157-0 | 48% | 354-382 | 48% |
| conventional | 32 | 50-46-0 | 52% | 121-109 | 53% |
| unguided | 148 | 227-217-0 | 51% | 562-546 | 51% |

### The nights with all three (16)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 32 | 42-54-0 | 44% | 104-128 | 45% |
| conventional | 32 | 50-46-0 | 52% | 121-109 | 53% |
| unguided | 64 | 100-92-0 | 52% | 246-234 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BR | 36 | 57-51-0 | 51% (265) |
| UB | 35 | 41-64-0 | 45% (266) |
| WR | 34 | 52-50-0 | 51% (247) |
| WU | 33 | 53-46-0 | 53% (246) |
| BG | 31 | 51-42-0 | 54% (225) |
| WG | 30 | 49-41-0 | 52% (220) |
| UR | 24 | 33-39-0 | 46% (177) |
| WB | 24 | 35-37-0 | 47% (177) |
| RG | 14 | 24-18-0 | 55% (108) |
| UG | 11 | 14-19-0 | 47% (83) |
| U | 3 | 4-5-0 | 48% (21) |
| G | 2 | 2-4-0 | 33% (15) |
| R | 2 | 3-3-0 | 50% (16) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 70 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 12 | 84 | 60% | 1.3 | 14/18 |
| Slayer of the Wicked | W | uncommon | 35 | 252 | 59% | 1.8 | 41/73 |
| Moldgraf Monstrosity | G | rare | 10 | 70 | 59% | 4.3 | 13/56 |
| Disciple of Griselbrand | B | uncommon | 21 | 154 | 58% | 9.2 | 47/434 |
| Mentor of the Meek | W | rare | 14 | 104 | 58% | 3.7 | 15/56 |
| Desperate Ravings | R | uncommon | 11 | 82 | 57% | 9.9 | 44/434 |
| Falkenrath Marauders | R | rare | 11 | 79 | 57% | 2.1 | 13/27 |
| Midnight Haunting | W | uncommon | 33 | 243 | 57% | 3.5 | 40/142 |
| Elite Inquisitor | W | rare | 14 | 104 | 57% | 2.7 | 16/43 |
| Bloodgift Demon | B | rare | 10 | 73 | 56% | 1.1 | 10/11 |
| Stromkirk Noble | R | rare | 13 | 96 | 56% | 4.6 | 14/65 |
| Villagers of Estwald | G | common | 41 | 305 | 56% | 4.5 | 85/385 |
| Into the Maw of Hell | R | uncommon | 36 | 270 | 55% | 3.3 | 44/145 |
| Furor of the Bitten | R | common | 28 | 203 | 55% | 9.8 | 81/780 |
| Manor Gargoyle | C | rare | 17 | 127 | 55% | 2.2 | 17/38 |
| Galvanic Juggernaut | C | uncommon | 44 | 332 | 55% | 1.2 | 44/52 |
| Kruin Outlaw | R | rare | 14 | 104 | 55% | 2.6 | 16/41 |
| Ashmouth Hound | R | common | 48 | 353 | 55% | 3.7 | 74/275 |
| Bonds of Faith | W | common | 56 | 416 | 55% | 3.9 | 73/284 |
| Lumberknot | G | uncommon | 31 | 231 | 55% | 4.4 | 39/173 |
| Hanweir Watchkeep | R | uncommon | 21 | 160 | 54% | 6.2 | 38/234 |
| Avacynian Priest | W | common | 59 | 431 | 54% | 5.6 | 74/411 |
| Thraben Sentry | W | common | 41 | 304 | 54% | 7.6 | 68/519 |
| Gatstaf Shepherd | G | uncommon | 34 | 253 | 54% | 5.2 | 49/253 |
| Rage Thrower | R | uncommon | 25 | 183 | 54% | 9.1 | 55/500 |
| Avacyn's Pilgrim | G | common | 23 | 170 | 54% | 9.0 | 78/702 |
| Voiceless Spirit | W | common | 62 | 456 | 54% | 3.5 | 74/260 |
| Woodland Sleuth | G | common | 42 | 308 | 54% | 6.4 | 77/494 |
| Spider Spawning | G | uncommon | 26 | 191 | 53% | 4.0 | 43/171 |
| Grave Bramble | G | common | 33 | 246 | 53% | 8.7 | 74/644 |
| Doomed Traveler | W | common | 57 | 416 | 53% | 6.9 | 77/529 |
| Riot Devils | R | common | 44 | 329 | 53% | 7.8 | 76/591 |
| Hollowhenge Scavenger | G | uncommon | 23 | 170 | 53% | 4.4 | 35/153 |
| Dearly Departed | W | rare | 12 | 87 | 53% | 2.1 | 16/33 |
| Morkrut Banshee | B | uncommon | 33 | 248 | 53% | 1.8 | 41/72 |
| Pitchburn Devils | R | common | 57 | 424 | 53% | 4.7 | 70/332 |
| Kindercatch | G | common | 47 | 350 | 53% | 7.4 | 73/540 |
| Kessig Wolf | R | common | 56 | 417 | 53% | 7.5 | 79/589 |
| Unruly Mob | W | common | 49 | 354 | 53% | 9.1 | 73/665 |
| Typhoid Rats | B | common | 54 | 394 | 53% | 3.4 | 77/261 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Garruk Relentless | G | mythic | 1.0 | 8 | 4 |
| Bloodline Keeper | B | rare | 1.1 | 13 | 9 |
| Bloodgift Demon | B | rare | 1.1 | 10 | 10 |
| Olivia Voldaren | BR | mythic | 1.1 | 7 | 3 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 44 | 44 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Mikaeus, the Lunarch | W | mythic | 1.2 | 6 | 4 |
| Reaper from the Abyss | B | mythic | 1.2 | 5 | 5 |
| Sever the Bloodline | B | rare | 1.3 | 14 | 12 |
| Balefire Dragon | R | mythic | 1.3 | 7 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 44 | 35 |
| Devil's Play | R | rare | 1.5 | 13 | 9 |
| Geist-Honored Monk | W | rare | 1.6 | 11 | 7 |
| Victim of Night | B | common | 1.6 | 74 | 52 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 78 | 52 |
| Slayer of the Wicked | W | uncommon | 1.8 | 41 | 35 |
| Falkenrath Noble | B | uncommon | 1.8 | 41 | 32 |
| Morkrut Banshee | B | uncommon | 1.8 | 41 | 33 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Chapel Geist | W | common | 1.9 | 61 | 44 |
| Claustrophobia | U | common | 2.0 | 75 | 52 |
| Brimstone Volley | R | common | 2.0 | 71 | 46 |
| Skaab Ruinator | U | mythic | 2.0 | 8 | 6 |
| Murder of Crows | U | uncommon | 2.1 | 40 | 32 |
| Falkenrath Marauders | R | rare | 2.1 | 13 | 11 |
| Dearly Departed | W | rare | 2.1 | 16 | 12 |
| Manor Gargoyle | C | rare | 2.2 | 17 | 17 |
| Rebuke | W | common | 2.3 | 81 | 55 |
| Daybreak Ranger | G | rare | 2.3 | 19 | 12 |
