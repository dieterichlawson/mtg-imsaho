# Draft loop: what the nights add up to

2 night(s), 16 decks, 63 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| unguided | 16 | 24-24-0 | 50% | 63-63 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 3 | 3-6-0 | 44% (25) |
| BG | 2 | 6-0-0 | 80% (15) |
| BR | 2 | 4-2-0 | 56% (16) |
| UR | 2 | 2-4-0 | 38% (16) |
| WG | 2 | 3-3-0 | 53% (15) |
| WU | 2 | 1-5-0 | 31% (16) |
| RG | 1 | 2-1-0 | 62% (8) |
| W | 1 | 2-1-0 | 50% (8) |
| WR | 1 | 1-2-0 | 43% (7) |

## Cards in winning decks (at least 6 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spidery Grasp | G | common | 1 | 7 | 86% | 7.8 | 4/31 |
| Spider Spawning | G | uncommon | 2 | 15 | 80% | 6.5 | 4/26 |
| Woodland Sleuth | G | common | 2 | 15 | 80% | 9.5 | 4/38 |
| Hollowhenge Scavenger | G | uncommon | 2 | 15 | 80% | 3.0 | 2/6 |
| Prey Upon | G | common | 2 | 15 | 80% | 3.0 | 2/6 |
| Boneyard Wurm | G | uncommon | 2 | 15 | 80% | 7.0 | 2/14 |
| Morkrut Banshee | B | uncommon | 2 | 15 | 80% | 1.0 | 2/2 |
| Lumberknot | G | uncommon | 2 | 15 | 80% | 8.0 | 2/16 |
| Manor Skeleton | B | common | 1 | 8 | 75% | 11.8 | 4/46 |
| Ghoulcaller's Chant | B | common | 1 | 8 | 75% | 11.0 | 4/43 |
| Trepanation Blade | C | uncommon | 1 | 8 | 75% | 2.5 | 2/5 |
| Festerhide Boar | G | common | 3 | 23 | 74% | 5.7 | 6/34 |
| Gatstaf Shepherd | G | uncommon | 3 | 23 | 74% | 6.3 | 6/38 |
| Darkthicket Wolf | G | common | 3 | 23 | 74% | 5.8 | 4/23 |
| Disciple of Griselbrand | B | uncommon | 2 | 15 | 73% | 11.5 | 4/46 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Villagers of Estwald | G | common | 4 | 31 | 71% | 4.6 | 10/46 |
| Orchard Spirit | G | common | 4 | 31 | 71% | 5.0 | 6/30 |
| Somberwald Spider | G | common | 3 | 23 | 70% | 4.5 | 4/18 |
| Kindercatch | G | common | 2 | 16 | 69% | 10.5 | 4/42 |
| Grave Bramble | G | common | 2 | 16 | 69% | 11.0 | 4/44 |
| Daybreak Ranger | G | rare | 1 | 8 | 62% | 1.0 | 2/2 |
| Ambush Viper | G | common | 1 | 8 | 62% | 4.0 | 2/8 |
| Essence of the Wild | G | mythic | 1 | 8 | 62% | 2.0 | 2/4 |
| Hamlet Captain | G | uncommon | 2 | 16 | 62% | 6.5 | 2/13 |
| Typhoid Rats | B | common | 3 | 24 | 62% | 3.0 | 4/12 |
| Silent Departure | U | common | 1 | 8 | 62% | 7.2 | 4/29 |
| Frightful Delusion | U | common | 1 | 8 | 62% | 6.5 | 4/26 |
| Creepy Doll | C | rare | 1 | 8 | 62% | 6.2 | 4/25 |
| Brain Weevil | B | common | 2 | 15 | 60% | 8.0 | 4/32 |
| Kruin Outlaw | R | rare | 2 | 15 | 60% | 2.5 | 2/5 |
| Bloodcrazed Neonate | R | common | 2 | 15 | 60% | 8.8 | 4/35 |
| Feral Ridgewolf | R | common | 2 | 15 | 60% | 6.5 | 4/26 |
| Falkenrath Marauders | R | rare | 2 | 15 | 60% | 3.5 | 2/7 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.5 | 2/9 |
| Victim of Night | B | common | 3 | 25 | 60% | 3.0 | 4/12 |
| Walking Corpse | B | common | 4 | 32 | 59% | 6.5 | 4/26 |
| Tribute to Hunger | B | uncommon | 2 | 17 | 59% | 2.5 | 2/5 |
| Reckless Waif | R | uncommon | 3 | 24 | 58% | 3.2 | 4/13 |
| Curse of the Pierced Heart | R | common | 2 | 16 | 56% | 9.8 | 4/39 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Claustrophobia | U | common | 1.0 | 4 | 2 |
| Murder of Crows | U | uncommon | 1.0 | 4 | 4 |
| Brimstone Volley | R | common | 1.0 | 4 | 3 |
| Geistflame | R | common | 1.2 | 4 | 4 |
| Chapel Geist | W | common | 1.5 | 6 | 3 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Corpse Lunge | B | common | 2.2 | 4 | 3 |
| Stitched Drake | U | common | 2.2 | 4 | 2 |
| Rebuke | W | common | 2.5 | 4 | 4 |
| Smite the Monstrous | W | common | 2.5 | 4 | 3 |
| Sensory Deprivation | U | common | 2.8 | 4 | 3 |
| Geistcatcher's Rig | C | uncommon | 3.0 | 4 | 4 |
| Scourge of Geier Reach | R | uncommon | 3.0 | 4 | 2 |
| Dead Weight | B | common | 3.0 | 4 | 3 |
| Typhoid Rats | B | common | 3.0 | 4 | 3 |
| Victim of Night | B | common | 3.0 | 4 | 3 |
| Gallows Warden | W | uncommon | 3.2 | 4 | 4 |
| Reckless Waif | R | uncommon | 3.2 | 4 | 3 |
| Ashmouth Hound | R | common | 3.5 | 4 | 2 |
| Makeshift Mauler | U | common | 3.5 | 4 | 3 |
| Village Cannibals | B | uncommon | 3.8 | 4 | 3 |
| Moon Heron | U | common | 3.8 | 4 | 4 |
| Mausoleum Guard | W | uncommon | 4.0 | 4 | 3 |
| Voiceless Spirit | W | common | 4.0 | 4 | 3 |
| Abbey Griffin | W | common | 4.2 | 4 | 2 |
| Somberwald Spider | G | common | 4.5 | 4 | 3 |
| Harvest Pyre | R | common | 4.5 | 4 | 2 |
| Villagers of Estwald | G | common | 4.6 | 10 | 4 |
| Orchard Spirit | G | common | 5.0 | 6 | 4 |
| Night Terrors | B | common | 5.2 | 4 | 2 |
