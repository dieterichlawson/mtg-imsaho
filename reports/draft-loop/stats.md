# Draft loop: what the nights add up to

27 night(s), 216 decks, 798 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 84 | 123-129-0 | 49% | 302-317 | 49% |
| conventional | 16 | 23-25-0 | 48% | 54-57 | 49% |
| unguided | 116 | 178-170-0 | 51% | 442-424 | 51% |

### The nights with all three (8)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 16 | 22-26-0 | 46% | 52-63 | 45% |
| conventional | 16 | 23-25-0 | 48% | 54-57 | 49% |
| unguided | 32 | 51-45-0 | 53% | 126-112 | 53% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 28 | 46-38-0 | 54% (203) |
| UB | 27 | 32-49-0 | 45% (204) |
| WU | 27 | 40-41-0 | 51% (201) |
| BR | 25 | 40-35-0 | 50% (181) |
| WR | 25 | 42-33-0 | 55% (184) |
| WG | 22 | 35-31-0 | 51% (156) |
| UR | 20 | 27-33-0 | 46% (149) |
| WB | 19 | 27-30-0 | 45% (143) |
| RG | 11 | 19-14-0 | 55% (84) |
| UG | 5 | 5-10-0 | 45% (38) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 54 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Moldgraf Monstrosity | G | rare | 8 | 57 | 61% | 3.9 | 10/39 |
| Furor of the Bitten | R | common | 19 | 138 | 60% | 9.9 | 62/613 |
| Slayer of the Wicked | W | uncommon | 26 | 187 | 59% | 1.7 | 30/50 |
| Mentor of the Meek | W | rare | 11 | 81 | 59% | 3.9 | 12/47 |
| Midnight Haunting | W | uncommon | 23 | 169 | 59% | 4.0 | 29/116 |
| Into the Maw of Hell | R | uncommon | 28 | 213 | 58% | 3.4 | 34/117 |
| Disciple of Griselbrand | B | uncommon | 17 | 126 | 58% | 9.1 | 39/355 |
| Ashmouth Hound | R | common | 36 | 264 | 57% | 3.8 | 56/215 |
| Lumberknot | G | uncommon | 24 | 176 | 57% | 4.1 | 30/123 |
| Stromkirk Noble | R | rare | 10 | 74 | 57% | 5.0 | 11/55 |
| Gatstaf Shepherd | G | uncommon | 25 | 184 | 56% | 4.6 | 34/156 |
| Bonds of Faith | W | common | 42 | 310 | 56% | 3.8 | 56/212 |
| Manor Gargoyle | C | rare | 13 | 95 | 56% | 2.2 | 13/28 |
| Villagers of Estwald | G | common | 29 | 212 | 56% | 4.2 | 61/257 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Woodland Sleuth | G | common | 31 | 222 | 55% | 6.0 | 58/350 |
| Elite Inquisitor | W | rare | 11 | 83 | 55% | 2.8 | 13/37 |
| Spider Spawning | G | uncommon | 21 | 154 | 55% | 3.6 | 36/131 |
| Rally the Peasants | W | uncommon | 23 | 165 | 55% | 6.7 | 41/276 |
| Sulfur Falls | C | rare | 8 | 60 | 55% | 11.1 | 15/167 |
| Typhoid Rats | B | common | 42 | 303 | 55% | 3.3 | 59/194 |
| Avacynian Priest | W | common | 43 | 316 | 54% | 5.4 | 57/309 |
| Hollowhenge Scavenger | G | uncommon | 19 | 140 | 54% | 4.2 | 27/114 |
| Grave Bramble | G | common | 25 | 184 | 54% | 8.8 | 56/495 |
| Voiceless Spirit | W | common | 51 | 379 | 54% | 3.6 | 58/211 |
| Tormented Pariah | R | common | 37 | 271 | 54% | 7.0 | 65/452 |
| Unruly Mob | W | common | 38 | 275 | 53% | 9.0 | 56/502 |
| Thraben Sentry | W | common | 35 | 260 | 53% | 7.6 | 58/442 |
| Festerhide Boar | G | common | 36 | 264 | 53% | 6.5 | 59/382 |
| Pitchburn Devils | R | common | 43 | 323 | 53% | 5.0 | 54/268 |
| Avacyn's Pilgrim | G | common | 17 | 122 | 53% | 9.2 | 60/549 |
| Silver-Inlaid Dagger | C | uncommon | 21 | 154 | 53% | 6.7 | 29/195 |
| Rage Thrower | R | uncommon | 18 | 130 | 53% | 9.6 | 40/384 |
| Galvanic Juggernaut | C | uncommon | 32 | 241 | 53% | 1.2 | 32/39 |
| Morkrut Banshee | B | uncommon | 28 | 211 | 53% | 1.7 | 32/53 |
| Hamlet Captain | G | uncommon | 24 | 178 | 53% | 6.9 | 32/220 |
| Kindercatch | G | common | 36 | 265 | 53% | 7.2 | 56/403 |
| Mindshrieker | U | rare | 10 | 72 | 53% | 6.1 | 12/73 |
| Spidery Grasp | G | common | 36 | 262 | 53% | 8.4 | 56/471 |
| Victim of Night | B | common | 41 | 298 | 53% | 1.6 | 55/87 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 10 | 7 |
| Garruk Relentless | G | mythic | 1.0 | 7 | 3 |
| Balefire Dragon | R | mythic | 1.0 | 3 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 32 | 32 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Geist-Honored Monk | W | rare | 1.3 | 7 | 5 |
| Skaab Ruinator | U | mythic | 1.3 | 3 | 2 |
| Sever the Bloodline | B | rare | 1.4 | 8 | 7 |
| Fiend Hunter | W | uncommon | 1.6 | 34 | 26 |
| Victim of Night | B | common | 1.6 | 55 | 41 |
| Slayer of the Wicked | W | uncommon | 1.7 | 30 | 26 |
| Falkenrath Noble | B | uncommon | 1.7 | 32 | 24 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 62 | 43 |
| Morkrut Banshee | B | uncommon | 1.7 | 32 | 28 |
| Angel of Flight Alabaster | W | rare | 1.8 | 11 | 8 |
| Devil's Play | R | rare | 1.8 | 8 | 6 |
| Claustrophobia | U | common | 1.9 | 58 | 39 |
| Chapel Geist | W | common | 2.0 | 48 | 34 |
| Murder of Crows | U | uncommon | 2.0 | 32 | 28 |
| Brimstone Volley | R | common | 2.0 | 55 | 36 |
| Dearly Departed | W | rare | 2.0 | 13 | 9 |
| Rebuke | W | common | 2.1 | 63 | 43 |
| Falkenrath Marauders | R | rare | 2.2 | 9 | 7 |
| Manor Gargoyle | C | rare | 2.2 | 13 | 13 |
| Instigator Gang | R | rare | 2.2 | 8 | 6 |
| Tribute to Hunger | B | uncommon | 2.3 | 28 | 26 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
