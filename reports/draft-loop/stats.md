# Draft loop: what the nights add up to

21 night(s), 168 decks, 619 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 72 | 103-113-0 | 48% | 255-274 | 48% |
| conventional | 4 | 6-6-0 | 50% | 13-12 | 52% |
| unguided | 92 | 143-133-0 | 52% | 351-333 | 51% |

### The nights with all three (2)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 4 | 2-10-0 | 17% | 5-20 | 20% |
| conventional | 4 | 6-6-0 | 50% | 13-12 | 52% |
| unguided | 8 | 16-8-0 | 67% | 35-21 | 62% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 24 | 40-32-0 | 54% (174) |
| WU | 23 | 34-35-0 | 51% (169) |
| UB | 22 | 28-38-0 | 47% (168) |
| WR | 18 | 28-26-0 | 53% (129) |
| BR | 17 | 32-19-0 | 55% (123) |
| WG | 17 | 27-24-0 | 50% (121) |
| UR | 16 | 20-28-0 | 45% (119) |
| WB | 14 | 19-23-0 | 45% (105) |
| RG | 9 | 14-13-0 | 51% (68) |
| R | 2 | 3-3-0 | 50% (16) |
| U | 2 | 2-4-0 | 36% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 42 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Sever the Bloodline | B | rare | 6 | 46 | 70% | 1.5 | 6/9 |
| Falkenrath Marauders | R | rare | 7 | 49 | 67% | 2.4 | 8/19 |
| Mentor of the Meek | W | rare | 8 | 59 | 63% | 4.4 | 9/40 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.5 | 24/252 |
| Morkrut Banshee | B | uncommon | 18 | 137 | 60% | 1.7 | 21/36 |
| Slayer of the Wicked | W | uncommon | 21 | 149 | 59% | 1.6 | 25/40 |
| Elite Inquisitor | W | rare | 9 | 67 | 58% | 2.7 | 11/30 |
| Gatstaf Shepherd | G | uncommon | 21 | 156 | 58% | 4.6 | 29/132 |
| Skirsdag Cultist | R | uncommon | 14 | 104 | 58% | 7.8 | 22/172 |
| Stromkirk Noble | R | rare | 8 | 59 | 58% | 5.5 | 8/44 |
| Spider Spawning | G | uncommon | 18 | 133 | 57% | 3.2 | 29/94 |
| Ashmouth Hound | R | common | 29 | 211 | 57% | 3.9 | 44/170 |
| Disciple of Griselbrand | B | uncommon | 15 | 111 | 57% | 8.8 | 31/274 |
| Midnight Haunting | W | uncommon | 19 | 138 | 57% | 4.2 | 23/97 |
| Typhoid Rats | B | common | 33 | 239 | 56% | 3.3 | 46/151 |
| Into the Maw of Hell | R | uncommon | 24 | 181 | 56% | 3.7 | 27/99 |
| Lumberknot | G | uncommon | 18 | 133 | 56% | 4.0 | 23/93 |
| Silver-Inlaid Dagger | C | uncommon | 18 | 130 | 56% | 6.8 | 26/176 |
| Rally the Peasants | W | uncommon | 15 | 105 | 56% | 7.1 | 31/220 |
| Bonds of Faith | W | common | 31 | 228 | 56% | 4.1 | 43/175 |
| Furor of the Bitten | R | common | 13 | 91 | 56% | 10.4 | 47/491 |
| Woodland Sleuth | G | common | 25 | 179 | 56% | 5.5 | 45/247 |
| Galvanic Juggernaut | C | uncommon | 25 | 187 | 56% | 1.2 | 25/30 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Avacynian Priest | W | common | 32 | 232 | 55% | 5.6 | 44/245 |
| Villagers of Estwald | G | common | 24 | 178 | 55% | 3.9 | 54/213 |
| Avacyn's Pilgrim | G | common | 14 | 100 | 55% | 9.1 | 49/448 |
| Manor Gargoyle | C | rare | 10 | 75 | 55% | 2.4 | 10/24 |
| Thraben Sentry | W | common | 25 | 185 | 55% | 7.6 | 40/303 |
| Traitorous Blood | R | common | 12 | 85 | 54% | 9.7 | 49/475 |
| Victim of Night | B | common | 33 | 235 | 54% | 1.7 | 43/72 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Grave Bramble | G | common | 21 | 153 | 54% | 8.9 | 44/393 |
| Unruly Mob | W | common | 27 | 195 | 53% | 9.2 | 43/396 |
| Kindercatch | G | common | 27 | 199 | 53% | 7.0 | 44/310 |
| Hanweir Watchkeep | R | uncommon | 14 | 107 | 53% | 6.4 | 22/141 |
| Tormented Pariah | R | common | 30 | 220 | 53% | 7.1 | 56/397 |
| Ghoulcaller's Chant | B | common | 28 | 206 | 53% | 7.7 | 45/347 |
| Festerhide Boar | G | common | 29 | 211 | 53% | 6.2 | 46/283 |
| Curse of the Pierced Heart | R | common | 17 | 120 | 52% | 8.4 | 45/380 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 6 | 4 |
| Garruk Relentless | G | mythic | 1.0 | 5 | 1 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 25 | 25 |
| Angelic Overseer | W | mythic | 1.2 | 5 | 3 |
| Devil's Play | R | rare | 1.2 | 5 | 4 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 26 | 22 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Slayer of the Wicked | W | uncommon | 1.6 | 25 | 21 |
| Falkenrath Noble | B | uncommon | 1.6 | 25 | 18 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Victim of Night | B | common | 1.7 | 43 | 33 |
| Morkrut Banshee | B | uncommon | 1.7 | 21 | 18 |
| Claustrophobia | U | common | 1.8 | 45 | 34 |
| Dead Weight | B | common | 1.8 | 48 | 35 |
| Murder of Crows | U | uncommon | 1.9 | 26 | 22 |
| Brimstone Volley | R | common | 1.9 | 44 | 27 |
| Angel of Flight Alabaster | W | rare | 1.9 | 8 | 7 |
| Chapel Geist | W | common | 2.0 | 37 | 26 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Dearly Departed | W | rare | 2.0 | 8 | 8 |
| Instigator Gang | R | rare | 2.2 | 5 | 5 |
| Rebuke | W | common | 2.3 | 49 | 33 |
| Tribute to Hunger | B | uncommon | 2.3 | 25 | 23 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 11 | 6 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Falkenrath Marauders | R | rare | 2.4 | 8 | 7 |
