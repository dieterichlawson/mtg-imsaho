# Draft loop: what the nights add up to

4 night(s), 32 decks, 119 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 8 | 11-13-0 | 46% | 25-29 | 46% |
| unguided | 24 | 37-35-0 | 51% | 94-90 | 51% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| WG | 5 | 6-9-0 | 43% (35) |
| BG | 4 | 10-2-0 | 70% (30) |
| BR | 4 | 7-5-0 | 52% (29) |
| UB | 4 | 5-7-0 | 50% (32) |
| UR | 3 | 5-4-0 | 52% (23) |
| WR | 3 | 4-5-0 | 50% (22) |
| WU | 3 | 4-5-0 | 46% (24) |
| RG | 2 | 3-3-0 | 50% (14) |
| U | 1 | 1-2-0 | 33% (6) |
| UG | 1 | 1-2-0 | 50% (8) |
| W | 1 | 2-1-0 | 50% (8) |
| WB | 1 | 0-3-0 | 14% (7) |

## Cards in winning decks (at least 8 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Trepanation Blade | C | uncommon | 1 | 8 | 75% | 4.2 | 5/21 |
| Manor Gargoyle | C | rare | 1 | 8 | 75% | 2.0 | 1/2 |
| Sturmgeist | U | rare | 1 | 8 | 75% | 5.0 | 1/5 |
| Cackling Counterpart | U | rare | 1 | 8 | 75% | 9.0 | 2/18 |
| Spider Spawning | G | uncommon | 3 | 23 | 74% | 5.2 | 6/31 |
| Gatstaf Shepherd | G | uncommon | 3 | 23 | 74% | 6.3 | 6/38 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Woodland Sleuth | G | common | 3 | 22 | 73% | 7.6 | 8/61 |
| Villagers of Estwald | G | common | 4 | 31 | 71% | 4.6 | 10/46 |
| Prey Upon | G | common | 4 | 29 | 69% | 3.3 | 6/20 |
| Silent Departure | U | common | 2 | 16 | 69% | 8.8 | 9/79 |
| Falkenrath Marauders | R | rare | 3 | 22 | 68% | 3.7 | 3/11 |
| Ghoulcaller's Chant | B | common | 4 | 29 | 66% | 9.0 | 8/71 |
| Morkrut Banshee | B | uncommon | 4 | 29 | 66% | 1.5 | 6/9 |
| Diregraf Ghoul | B | uncommon | 2 | 14 | 64% | 7.0 | 3/21 |
| Somberwald Spider | G | common | 6 | 46 | 63% | 3.5 | 8/28 |
| Daybreak Ranger | G | rare | 1 | 8 | 62% | 1.0 | 2/2 |
| Hamlet Captain | G | uncommon | 3 | 24 | 62% | 4.5 | 4/18 |
| Creepy Doll | C | rare | 1 | 8 | 62% | 6.2 | 4/25 |
| Bloodline Keeper | B | rare | 1 | 8 | 62% | 1.0 | 2/2 |
| Lumberknot | G | uncommon | 5 | 37 | 62% | 4.8 | 5/24 |
| Victim of Night | B | common | 5 | 39 | 62% | 2.1 | 8/17 |
| Brain Weevil | B | common | 5 | 36 | 61% | 6.2 | 8/50 |
| Kruin Outlaw | R | rare | 2 | 15 | 60% | 2.5 | 2/5 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.2 | 4/17 |
| Disciple of Griselbrand | B | uncommon | 4 | 30 | 60% | 9.3 | 7/65 |
| Selhoff Occultist | U | common | 5 | 37 | 59% | 10.4 | 8/83 |
| Typhoid Rats | B | common | 6 | 44 | 59% | 3.4 | 9/31 |
| Reckless Waif | R | uncommon | 3 | 24 | 58% | 3.8 | 5/19 |
| Orchard Spirit | G | common | 6 | 45 | 58% | 4.4 | 10/44 |
| Hysterical Blindness | U | common | 3 | 21 | 57% | 11.8 | 10/117 |
| Manor Skeleton | B | common | 4 | 28 | 57% | 10.9 | 8/86 |
| Moan of the Unhallowed | B | uncommon | 3 | 21 | 57% | 4.2 | 4/17 |
| Frightful Delusion | U | common | 3 | 21 | 57% | 8.4 | 9/76 |
| Thraben Sentry | W | common | 3 | 21 | 57% | 9.1 | 7/64 |
| Geistflame | R | common | 8 | 58 | 57% | 1.7 | 9/15 |
| Boneyard Wurm | G | uncommon | 5 | 37 | 57% | 4.8 | 5/24 |
| Unruly Mob | W | common | 4 | 30 | 57% | 11.4 | 8/91 |
| Curse of the Pierced Heart | R | common | 2 | 16 | 56% | 9.0 | 8/72 |
| Walking Corpse | B | common | 7 | 54 | 56% | 6.4 | 8/51 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Geist-Honored Monk | W | rare | 1.0 | 3 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 4 | 4 |
| Slayer of the Wicked | W | uncommon | 1.3 | 3 | 2 |
| Fiend Hunter | W | uncommon | 1.3 | 6 | 5 |
| Chapel Geist | W | common | 1.4 | 9 | 5 |
| Murder of Crows | U | uncommon | 1.5 | 6 | 6 |
| Morkrut Banshee | B | uncommon | 1.5 | 6 | 4 |
| Falkenrath Noble | B | uncommon | 1.6 | 5 | 1 |
| Geistflame | R | common | 1.7 | 9 | 8 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Claustrophobia | U | common | 1.8 | 9 | 5 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Brimstone Volley | R | common | 1.9 | 8 | 7 |
| Essence of the Wild | G | mythic | 2.0 | 3 | 2 |
| Dead Weight | B | common | 2.1 | 8 | 6 |
| Victim of Night | B | common | 2.1 | 8 | 5 |
| Geistcatcher's Rig | C | uncommon | 2.3 | 7 | 7 |
| Tribute to Hunger | B | uncommon | 2.3 | 3 | 3 |
| Hollowhenge Scavenger | G | uncommon | 2.4 | 5 | 5 |
| Rebuke | W | common | 2.5 | 8 | 7 |
| Screeching Bat | B | uncommon | 2.5 | 4 | 3 |
| Stitched Drake | U | common | 2.9 | 8 | 5 |
| Mayor of Avabruck | G | rare | 3.0 | 3 | 3 |
| Smite the Monstrous | W | common | 3.1 | 9 | 8 |
| Into the Maw of Hell | R | uncommon | 3.2 | 5 | 5 |
| Grasp of Phantoms | U | uncommon | 3.2 | 4 | 4 |
| Prey Upon | G | common | 3.3 | 6 | 4 |
| Corpse Lunge | B | common | 3.4 | 8 | 6 |
| Typhoid Rats | B | common | 3.4 | 9 | 6 |
| Somberwald Spider | G | common | 3.5 | 8 | 6 |
