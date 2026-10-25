# Draft loop: what the nights add up to

5 night(s), 40 decks, 146 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 12 | 16-20-0 | 44% | 38-44 | 46% |
| unguided | 28 | 44-40-0 | 52% | 108-102 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 6 | 15-3-0 | 73% (44) |
| UB | 5 | 5-10-0 | 42% (38) |
| WG | 5 | 6-9-0 | 43% (35) |
| WR | 5 | 7-8-0 | 49% (35) |
| BR | 4 | 7-5-0 | 52% (29) |
| UR | 4 | 7-5-0 | 53% (30) |
| WU | 4 | 6-6-0 | 50% (30) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |
| WB | 1 | 0-3-0 | 14% (7) |

## Cards in winning decks (at least 10 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Noble | B | uncommon | 2 | 12 | 83% | 1.7 | 6/10 |
| Spider Spawning | G | uncommon | 4 | 29 | 79% | 4.6 | 8/37 |
| Woodland Sleuth | G | common | 5 | 36 | 75% | 7.1 | 10/71 |
| Villagers of Estwald | G | common | 6 | 45 | 73% | 3.7 | 14/52 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Prey Upon | G | common | 6 | 43 | 72% | 3.0 | 8/24 |
| Sturmgeist | U | rare | 2 | 14 | 71% | 4.0 | 2/8 |
| Falkenrath Marauders | R | rare | 3 | 22 | 68% | 3.7 | 3/11 |
| Gatstaf Shepherd | G | uncommon | 6 | 45 | 67% | 5.1 | 9/46 |
| Manor Gargoyle | C | rare | 2 | 15 | 67% | 3.0 | 2/6 |
| Cackling Counterpart | U | rare | 2 | 15 | 67% | 9.2 | 4/37 |
| Morkrut Banshee | B | uncommon | 4 | 29 | 66% | 1.5 | 6/9 |
| Diregraf Ghoul | B | uncommon | 2 | 14 | 64% | 7.0 | 3/21 |
| Typhoid Rats | B | common | 8 | 58 | 64% | 3.5 | 11/38 |
| Hamlet Captain | G | uncommon | 4 | 32 | 62% | 5.0 | 5/25 |
| Bump in the Night | B | common | 5 | 36 | 61% | 8.7 | 11/96 |
| Ghoulcaller's Chant | B | common | 6 | 41 | 61% | 8.8 | 10/87 |
| Kindercatch | G | common | 6 | 41 | 61% | 8.3 | 9/75 |
| Disciple of Griselbrand | B | uncommon | 5 | 38 | 61% | 8.9 | 9/80 |
| Grave Bramble | G | common | 6 | 43 | 60% | 9.3 | 11/102 |
| Corpse Lunge | B | common | 7 | 50 | 60% | 3.6 | 9/32 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.2 | 4/17 |
| Festerhide Boar | G | common | 8 | 57 | 60% | 5.1 | 12/61 |
| Kruin Outlaw | R | rare | 3 | 22 | 59% | 2.0 | 3/6 |
| Victim of Night | B | common | 7 | 51 | 59% | 2.2 | 10/22 |
| Moan of the Unhallowed | B | uncommon | 4 | 29 | 59% | 3.6 | 5/18 |
| Somberwald Spider | G | common | 8 | 62 | 58% | 3.4 | 10/34 |
| Slayer of the Wicked | W | uncommon | 4 | 26 | 58% | 1.2 | 5/6 |
| Hysterical Blindness | U | common | 3 | 21 | 57% | 11.2 | 12/133 |
| Frightful Delusion | U | common | 3 | 21 | 57% | 8.5 | 11/94 |
| Thraben Sentry | W | common | 3 | 21 | 57% | 9.1 | 7/64 |
| Dearly Departed | W | rare | 2 | 14 | 57% | 2.5 | 2/5 |
| Unruly Mob | W | common | 5 | 37 | 57% | 11.0 | 10/110 |
| Curse of the Pierced Heart | R | common | 2 | 16 | 56% | 9.2 | 10/92 |
| Hollowhenge Scavenger | G | uncommon | 6 | 43 | 56% | 2.5 | 6/15 |
| Lumberknot | G | uncommon | 6 | 45 | 56% | 4.2 | 6/25 |
| Village Ironsmith | R | common | 4 | 29 | 55% | 8.2 | 8/66 |
| Geistflame | R | common | 10 | 71 | 55% | 2.2 | 11/24 |
| Ambush Viper | G | common | 7 | 51 | 55% | 4.7 | 9/42 |
| Walking Corpse | B | common | 9 | 66 | 55% | 6.2 | 10/62 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Geist-Honored Monk | W | rare | 1.0 | 3 | 2 |
| Slayer of the Wicked | W | uncommon | 1.2 | 5 | 4 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 5 | 5 |
| Murder of Crows | U | uncommon | 1.4 | 7 | 7 |
| Fiend Hunter | W | uncommon | 1.4 | 7 | 6 |
| Morkrut Banshee | B | uncommon | 1.5 | 6 | 4 |
| Chapel Geist | W | common | 1.6 | 11 | 6 |
| Claustrophobia | U | common | 1.7 | 11 | 7 |
| Falkenrath Noble | B | uncommon | 1.7 | 6 | 2 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Brimstone Volley | R | common | 1.8 | 10 | 8 |
| Dead Weight | B | common | 1.9 | 10 | 7 |
| Essence of the Wild | G | mythic | 2.0 | 3 | 2 |
| Kruin Outlaw | R | rare | 2.0 | 3 | 3 |
| Geistcatcher's Rig | C | uncommon | 2.1 | 8 | 8 |
| Geistflame | R | common | 2.2 | 11 | 10 |
| Victim of Night | B | common | 2.2 | 10 | 7 |
| Tribute to Hunger | B | uncommon | 2.2 | 5 | 5 |
| Rebuke | W | common | 2.4 | 10 | 8 |
| Stitched Drake | U | common | 2.5 | 10 | 7 |
| Hollowhenge Scavenger | G | uncommon | 2.5 | 6 | 6 |
| Screeching Bat | B | uncommon | 2.5 | 4 | 3 |
| Prey Upon | G | common | 3.0 | 8 | 6 |
| Mayor of Avabruck | G | rare | 3.0 | 3 | 3 |
| Into the Maw of Hell | R | uncommon | 3.2 | 6 | 6 |
| Smite the Monstrous | W | common | 3.2 | 11 | 10 |
| Somberwald Spider | G | common | 3.4 | 10 | 8 |
| Grasp of Phantoms | U | uncommon | 3.4 | 5 | 5 |
| Typhoid Rats | B | common | 3.5 | 11 | 8 |
