# Draft loop: what the nights add up to

24 night(s), 192 decks, 711 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

`guided` seats draft with `guide.md`, the guide this loop writes; `conventional` seats with `conventional.md`, a fixed guide of standard Innistrad advice; `unguided` seats with none. Odd seats are unguided. Every even seat was guided until the conventional guide came in; since, seats 0 and 4 are guided and 2 and 6 conventional.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 78 | 114-120-0 | 49% | 280-295 | 49% |
| conventional | 10 | 16-14-0 | 53% | 36-32 | 53% |
| unguided | 104 | 158-154-0 | 51% | 395-384 | 51% |

### The nights with all three (5)

The same seats on the same nights, which is the fair test of the two guides.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 10 | 13-17-0 | 43% | 30-41 | 42% |
| conventional | 10 | 16-14-0 | 53% | 36-32 | 53% |
| unguided | 20 | 31-29-0 | 52% | 79-72 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 25 | 41-34-0 | 54% (183) |
| WU | 25 | 37-38-0 | 50% (185) |
| UB | 23 | 28-41-0 | 46% (177) |
| WG | 22 | 35-31-0 | 51% (156) |
| BR | 21 | 36-27-0 | 52% (153) |
| WR | 21 | 36-27-0 | 56% (153) |
| UR | 19 | 24-33-0 | 45% (141) |
| WB | 16 | 23-25-0 | 46% (122) |
| RG | 9 | 14-13-0 | 51% (68) |
| UG | 4 | 3-9-0 | 42% (31) |
| U | 3 | 4-5-0 | 48% (21) |
| R | 2 | 3-3-0 | 50% (16) |
| G | 1 | 2-1-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 48 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 7 | 49 | 67% | 2.4 | 8/19 |
| Mentor of the Meek | W | rare | 9 | 68 | 63% | 4.4 | 10/44 |
| Desperate Ravings | R | uncommon | 7 | 50 | 60% | 10.5 | 28/293 |
| Slayer of the Wicked | W | uncommon | 24 | 169 | 60% | 1.7 | 28/47 |
| Furor of the Bitten | R | common | 16 | 115 | 59% | 10.2 | 56/570 |
| Midnight Haunting | W | uncommon | 21 | 153 | 58% | 4.0 | 25/100 |
| Rally the Peasants | W | uncommon | 19 | 137 | 58% | 6.9 | 36/247 |
| Bonds of Faith | W | common | 37 | 273 | 57% | 3.9 | 49/192 |
| Disciple of Griselbrand | B | uncommon | 16 | 119 | 57% | 9.2 | 36/332 |
| Moldgraf Monstrosity | G | rare | 7 | 51 | 57% | 3.9 | 9/35 |
| Into the Maw of Hell | R | uncommon | 25 | 190 | 57% | 3.5 | 30/104 |
| Ashmouth Hound | R | common | 33 | 241 | 56% | 4.0 | 51/203 |
| Lumberknot | G | uncommon | 21 | 156 | 56% | 3.8 | 26/99 |
| Manor Gargoyle | C | rare | 11 | 82 | 56% | 2.3 | 11/25 |
| Gatstaf Shepherd | G | uncommon | 22 | 163 | 56% | 4.5 | 30/136 |
| Kruin Outlaw | R | rare | 11 | 81 | 56% | 2.6 | 12/31 |
| Elite Inquisitor | W | rare | 11 | 83 | 55% | 2.8 | 13/37 |
| Morkrut Banshee | B | uncommon | 21 | 161 | 55% | 1.6 | 25/41 |
| Thraben Sentry | W | common | 30 | 223 | 55% | 7.5 | 49/367 |
| Stromkirk Noble | R | rare | 9 | 67 | 55% | 5.4 | 9/49 |
| Skirsdag Cultist | R | uncommon | 17 | 127 | 55% | 8.3 | 27/223 |
| Typhoid Rats | B | common | 37 | 270 | 55% | 3.3 | 52/173 |
| Woodland Sleuth | G | common | 28 | 201 | 55% | 5.7 | 51/291 |
| Spider Spawning | G | uncommon | 20 | 147 | 54% | 3.4 | 33/112 |
| Avacynian Priest | W | common | 37 | 269 | 54% | 5.5 | 50/276 |
| Galvanic Juggernaut | C | uncommon | 28 | 210 | 54% | 1.2 | 28/33 |
| Unruly Mob | W | common | 32 | 233 | 54% | 9.1 | 50/455 |
| Curse of the Pierced Heart | R | common | 19 | 135 | 54% | 8.5 | 52/440 |
| Voiceless Spirit | W | common | 46 | 341 | 54% | 3.8 | 52/195 |
| Bloodgift Demon | B | rare | 7 | 50 | 54% | 1.1 | 7/8 |
| Villagers of Estwald | G | common | 26 | 191 | 54% | 4.1 | 56/230 |
| Tormented Pariah | R | common | 32 | 235 | 54% | 7.1 | 59/419 |
| Woodland Cemetery | C | rare | 8 | 58 | 53% | 9.8 | 16/156 |
| Avacyn's Pilgrim | G | common | 17 | 122 | 53% | 9.1 | 54/489 |
| Grave Bramble | G | common | 23 | 169 | 53% | 8.9 | 50/444 |
| Victim of Night | B | common | 38 | 278 | 53% | 1.6 | 49/80 |
| Butcher's Cleaver | C | uncommon | 11 | 81 | 53% | 5.8 | 29/167 |
| Silver-Inlaid Dagger | C | uncommon | 20 | 145 | 53% | 6.8 | 28/189 |
| Traitorous Blood | R | common | 14 | 100 | 53% | 9.4 | 54/505 |
| Hamlet Captain | G | uncommon | 24 | 178 | 53% | 6.7 | 29/194 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 4 | 3 |
| Bloodline Keeper | B | rare | 1.0 | 8 | 5 |
| Garruk Relentless | G | mythic | 1.0 | 6 | 2 |
| Bloodgift Demon | B | rare | 1.1 | 7 | 7 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 28 | 28 |
| Angelic Overseer | W | mythic | 1.2 | 6 | 4 |
| Geist-Honored Monk | W | rare | 1.3 | 6 | 5 |
| Fiend Hunter | W | uncommon | 1.5 | 31 | 25 |
| Sever the Bloodline | B | rare | 1.5 | 6 | 6 |
| Falkenrath Noble | B | uncommon | 1.6 | 30 | 22 |
| Victim of Night | B | common | 1.6 | 49 | 38 |
| Morkrut Banshee | B | uncommon | 1.6 | 25 | 21 |
| Slayer of the Wicked | W | uncommon | 1.7 | 28 | 24 |
| Grimgrin, Corpse-Born | UB | mythic | 1.7 | 6 | 4 |
| Dead Weight | B | common | 1.7 | 55 | 38 |
| Devil's Play | R | rare | 1.7 | 7 | 5 |
| Angel of Flight Alabaster | W | rare | 1.8 | 10 | 8 |
| Claustrophobia | U | common | 1.9 | 52 | 37 |
| Murder of Crows | U | uncommon | 1.9 | 29 | 25 |
| Brimstone Volley | R | common | 1.9 | 50 | 31 |
| Chapel Geist | W | common | 2.0 | 42 | 30 |
| Geist of Saint Traft | WU | mythic | 2.0 | 3 | 0 |
| Dearly Departed | W | rare | 2.1 | 10 | 9 |
| Instigator Gang | R | rare | 2.1 | 7 | 6 |
| Rebuke | W | common | 2.2 | 57 | 39 |
| Tribute to Hunger | B | uncommon | 2.3 | 27 | 25 |
| Manor Gargoyle | C | rare | 2.3 | 11 | 11 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Daybreak Ranger | G | rare | 2.4 | 12 | 7 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
