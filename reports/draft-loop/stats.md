# Draft loop: what the nights add up to

3 night(s), 24 decks, 91 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 4 | 5-7-0 | 42% | 11-16 | 41% |
| unguided | 20 | 31-29-0 | 52% | 80-75 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 4 | 5-7-0 | 50% (32) |
| WG | 4 | 5-7-0 | 44% (27) |
| BG | 3 | 8-1-0 | 74% (23) |
| BR | 3 | 5-4-0 | 48% (23) |
| WU | 3 | 4-5-0 | 46% (24) |
| UR | 2 | 2-4-0 | 38% (16) |
| WR | 2 | 2-4-0 | 47% (15) |
| RG | 1 | 2-1-0 | 62% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 6 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Woodland Sleuth | G | common | 2 | 15 | 80% | 8.5 | 6/51 |
| Prey Upon | G | common | 3 | 21 | 76% | 3.0 | 4/12 |
| Trepanation Blade | C | uncommon | 1 | 8 | 75% | 4.0 | 4/16 |
| Manor Gargoyle | C | rare | 1 | 8 | 75% | 2.0 | 1/2 |
| Sturmgeist | U | rare | 1 | 8 | 75% | 5.0 | 1/5 |
| Cackling Counterpart | U | rare | 1 | 8 | 75% | 9.0 | 2/18 |
| Spider Spawning | G | uncommon | 3 | 23 | 74% | 5.4 | 5/27 |
| Gatstaf Shepherd | G | uncommon | 3 | 23 | 74% | 6.3 | 6/38 |
| Lumberknot | G | uncommon | 3 | 23 | 74% | 6.0 | 3/18 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Laboratory Maniac | U | rare | 1 | 7 | 71% | 11.3 | 3/34 |
| Screeching Bat | B | uncommon | 1 | 7 | 71% | 2.5 | 2/5 |
| Hinterland Harbor | C | rare | 1 | 7 | 71% | 14.0 | 1/13 |
| Bloodgift Demon | B | rare | 1 | 7 | 71% | 2.0 | 1/2 |
| Diregraf Ghoul | B | uncommon | 1 | 7 | 71% | 4.0 | 1/4 |
| Evil Twin | UB | rare | 1 | 7 | 71% | 6.0 | 1/6 |
| Villagers of Estwald | G | common | 4 | 31 | 71% | 4.6 | 10/46 |
| Ghoulcaller's Chant | B | common | 2 | 16 | 69% | 10.5 | 6/62 |
| Silent Departure | U | common | 2 | 16 | 69% | 8.6 | 7/60 |
| Somberwald Spider | G | common | 4 | 31 | 68% | 4.0 | 6/24 |
| Mayor of Avabruck | G | rare | 1 | 6 | 67% | 2.0 | 1/2 |
| Morkrut Banshee | B | uncommon | 4 | 29 | 66% | 1.0 | 4/4 |
| Daybreak Ranger | G | rare | 1 | 8 | 62% | 1.0 | 2/2 |
| Bramblecrush | G | uncommon | 1 | 8 | 62% | 9.8 | 4/39 |
| Essence of the Wild | G | mythic | 1 | 8 | 62% | 2.0 | 2/4 |
| Hamlet Captain | G | uncommon | 3 | 24 | 62% | 5.0 | 3/15 |
| Creepy Doll | C | rare | 1 | 8 | 62% | 6.2 | 4/25 |
| Victim of Night | B | common | 4 | 32 | 62% | 2.3 | 6/14 |
| Bloodline Keeper | B | rare | 1 | 8 | 62% | 1.0 | 1/1 |
| Cloistered Youth | W | uncommon | 2 | 16 | 62% | 4.0 | 2/8 |
| Brain Weevil | B | common | 3 | 23 | 61% | 7.0 | 5/35 |
| Kruin Outlaw | R | rare | 2 | 15 | 60% | 2.5 | 2/5 |
| Feral Ridgewolf | R | common | 2 | 15 | 60% | 7.3 | 6/44 |
| Falkenrath Marauders | R | rare | 2 | 15 | 60% | 3.5 | 2/7 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.2 | 4/17 |
| Disciple of Griselbrand | B | uncommon | 4 | 30 | 60% | 9.2 | 6/55 |
| Festerhide Boar | G | common | 5 | 37 | 59% | 5.1 | 8/41 |
| Orchard Spirit | G | common | 5 | 37 | 59% | 4.8 | 8/38 |
| Darkthicket Wolf | G | common | 4 | 29 | 59% | 4.2 | 6/25 |
| Hollowhenge Scavenger | G | uncommon | 4 | 29 | 59% | 2.8 | 4/11 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Galvanic Juggernaut | C | uncommon | 1.0 | 3 | 3 |
| Morkrut Banshee | B | uncommon | 1.0 | 4 | 4 |
| Murder of Crows | U | uncommon | 1.2 | 5 | 5 |
| Fiend Hunter | W | uncommon | 1.2 | 4 | 4 |
| Slayer of the Wicked | W | uncommon | 1.3 | 3 | 2 |
| Chapel Geist | W | common | 1.4 | 7 | 4 |
| Geistflame | R | common | 1.5 | 6 | 5 |
| Brimstone Volley | R | common | 1.7 | 6 | 5 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Claustrophobia | U | common | 1.8 | 6 | 4 |
| Falkenrath Noble | B | uncommon | 1.8 | 4 | 0 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Dead Weight | B | common | 2.3 | 6 | 5 |
| Victim of Night | B | common | 2.3 | 6 | 4 |
| Tribute to Hunger | B | uncommon | 2.3 | 3 | 3 |
| Geistcatcher's Rig | C | uncommon | 2.6 | 5 | 5 |
| Into the Maw of Hell | R | uncommon | 2.7 | 3 | 3 |
| Corpse Lunge | B | common | 2.8 | 6 | 4 |
| Rebuke | W | common | 2.8 | 6 | 6 |
| Hollowhenge Scavenger | G | uncommon | 2.8 | 4 | 4 |
| Abattoir Ghoul | B | uncommon | 3.0 | 3 | 2 |
| Stitched Drake | U | common | 3.0 | 6 | 4 |
| Prey Upon | G | common | 3.0 | 4 | 3 |
| Smite the Monstrous | W | common | 3.1 | 7 | 6 |
| Gallows Warden | W | uncommon | 3.2 | 4 | 4 |
| Ashmouth Hound | R | common | 3.3 | 6 | 3 |
| Sensory Deprivation | U | common | 3.3 | 6 | 5 |
| Typhoid Rats | B | common | 3.3 | 7 | 5 |
| Grasp of Phantoms | U | uncommon | 3.3 | 3 | 3 |
| Scourge of Geier Reach | R | uncommon | 3.4 | 5 | 3 |
