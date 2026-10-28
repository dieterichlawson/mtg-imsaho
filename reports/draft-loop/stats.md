# Draft loop: what the nights add up to

8 night(s), 64 decks, 233 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 24 | 34-38-0 | 47% | 82-88 | 48% |
| unguided | 40 | 62-58-0 | 52% | 151-145 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 14 | 28-14-0 | 61% (100) |
| UR | 8 | 11-13-0 | 45% (58) |
| WR | 8 | 13-11-0 | 54% (57) |
| WU | 8 | 13-11-0 | 53% (60) |
| UB | 6 | 6-12-0 | 43% (46) |
| BR | 5 | 8-7-0 | 50% (36) |
| WG | 5 | 6-9-0 | 43% (35) |
| WB | 3 | 2-7-0 | 27% (22) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| R | 1 | 2-1-0 | 62% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 16 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 4 | 28 | 75% | 3.2 | 5/16 |
| Spider Spawning | G | uncommon | 7 | 51 | 75% | 4.0 | 12/48 |
| Cackling Counterpart | U | rare | 3 | 22 | 73% | 9.4 | 7/66 |
| Gatstaf Shepherd | G | uncommon | 9 | 67 | 66% | 4.6 | 14/64 |
| Prey Upon | G | common | 10 | 69 | 65% | 2.6 | 14/37 |
| Woodland Sleuth | G | common | 10 | 70 | 64% | 5.5 | 16/88 |
| Villagers of Estwald | G | common | 9 | 66 | 64% | 3.2 | 23/74 |
| Morkrut Banshee | B | uncommon | 6 | 44 | 64% | 1.6 | 9/14 |
| Manor Gargoyle | C | rare | 4 | 30 | 63% | 2.8 | 4/11 |
| Woodland Cemetery | C | rare | 3 | 19 | 63% | 8.2 | 5/41 |
| Hamlet Captain | G | uncommon | 7 | 53 | 62% | 5.2 | 9/47 |
| Ghoulcaller's Chant | B | common | 11 | 78 | 62% | 7.9 | 17/133 |
| Unruly Mob | W | common | 6 | 44 | 61% | 11.2 | 16/179 |
| Slayer of the Wicked | W | uncommon | 6 | 41 | 61% | 1.6 | 8/13 |
| Falkenrath Noble | B | uncommon | 6 | 41 | 61% | 1.4 | 10/14 |
| Lumberknot | G | uncommon | 9 | 66 | 61% | 4.3 | 9/39 |
| Typhoid Rats | B | common | 12 | 91 | 60% | 3.4 | 16/54 |
| Thraben Sentry | W | common | 6 | 44 | 59% | 9.5 | 11/105 |
| Avacynian Priest | W | common | 11 | 78 | 59% | 5.2 | 17/88 |
| Kindercatch | G | common | 11 | 78 | 59% | 6.9 | 15/104 |
| Galvanic Juggernaut | C | uncommon | 9 | 67 | 58% | 1.3 | 9/12 |
| Somberwald Spider | G | common | 12 | 91 | 58% | 4.2 | 16/67 |
| Bonds of Faith | W | common | 10 | 74 | 58% | 4.4 | 18/80 |
| Hysterical Blindness | U | common | 3 | 21 | 57% | 11.1 | 21/233 |
| Dearly Departed | W | rare | 3 | 21 | 57% | 3.0 | 3/9 |
| Corpse Lunge | B | common | 12 | 85 | 56% | 3.1 | 15/47 |
| Moan of the Unhallowed | B | uncommon | 6 | 46 | 57% | 3.1 | 8/25 |
| Victim of Night | B | common | 12 | 85 | 56% | 1.9 | 16/30 |
| Midnight Haunting | W | uncommon | 7 | 50 | 56% | 4.6 | 8/37 |
| Hollowhenge Scavenger | G | uncommon | 6 | 43 | 56% | 2.5 | 6/15 |
| Grave Bramble | G | common | 11 | 77 | 56% | 8.3 | 17/141 |
| Festerhide Boar | G | common | 15 | 106 | 56% | 4.6 | 20/92 |
| Ambush Viper | G | common | 10 | 72 | 56% | 5.2 | 14/73 |
| Traitorous Blood | R | common | 4 | 29 | 55% | 9.9 | 19/189 |
| Silverchase Fox | W | common | 11 | 80 | 55% | 7.8 | 18/140 |
| Into the Maw of Hell | R | uncommon | 9 | 66 | 55% | 3.3 | 10/33 |
| Essence of the Wild | G | mythic | 3 | 22 | 55% | 1.8 | 4/7 |
| Village Ironsmith | R | common | 6 | 44 | 55% | 8.1 | 13/105 |
| Boneyard Wurm | G | uncommon | 10 | 74 | 54% | 5.2 | 10/52 |
| Spidery Grasp | G | common | 11 | 80 | 54% | 8.6 | 16/138 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Bloodline Keeper | B | rare | 1.0 | 3 | 2 |
| Murder of Crows | U | uncommon | 1.3 | 9 | 9 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 9 | 9 |
| Falkenrath Noble | B | uncommon | 1.4 | 10 | 6 |
| Daybreak Ranger | G | rare | 1.5 | 4 | 2 |
| Fiend Hunter | W | uncommon | 1.5 | 11 | 10 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Slayer of the Wicked | W | uncommon | 1.6 | 8 | 6 |
| Chapel Geist | W | common | 1.6 | 17 | 11 |
| Morkrut Banshee | B | uncommon | 1.6 | 9 | 6 |
| Claustrophobia | U | common | 1.7 | 17 | 11 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Essence of the Wild | G | mythic | 1.8 | 4 | 3 |
| Dead Weight | B | common | 1.9 | 16 | 12 |
| Brimstone Volley | R | common | 1.9 | 17 | 11 |
| Victim of Night | B | common | 1.9 | 16 | 12 |
| Kruin Outlaw | R | rare | 2.0 | 5 | 5 |
| Geistcatcher's Rig | C | uncommon | 2.1 | 11 | 11 |
| Screeching Bat | B | uncommon | 2.1 | 7 | 6 |
| Geistflame | R | common | 2.4 | 17 | 15 |
| Rebuke | W | common | 2.4 | 16 | 11 |
| Hollowhenge Scavenger | G | uncommon | 2.5 | 6 | 6 |
| Tribute to Hunger | B | uncommon | 2.6 | 7 | 7 |
| Prey Upon | G | common | 2.6 | 14 | 10 |
| Stitched Drake | U | common | 2.7 | 16 | 12 |
| Manor Gargoyle | C | rare | 2.8 | 4 | 4 |
| Grasp of Phantoms | U | uncommon | 3.0 | 8 | 8 |
| Mayor of Avabruck | G | rare | 3.0 | 3 | 3 |
| Dearly Departed | W | rare | 3.0 | 3 | 3 |
