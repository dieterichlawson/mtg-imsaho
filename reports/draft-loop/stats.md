# Draft loop: what the nights add up to

7 night(s), 56 decks, 205 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 20 | 30-30-0 | 50% | 70-71 | 50% |
| unguided | 36 | 54-54-0 | 50% | 135-134 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 11 | 25-8-0 | 67% (78) |
| WR | 8 | 13-11-0 | 54% (57) |
| UB | 6 | 6-12-0 | 43% (46) |
| UR | 6 | 8-10-0 | 44% (45) |
| WU | 6 | 8-10-0 | 48% (46) |
| WG | 5 | 6-9-0 | 43% (35) |
| BR | 4 | 7-5-0 | 52% (29) |
| WB | 3 | 2-7-0 | 27% (22) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| R | 1 | 2-1-0 | 62% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 14 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spider Spawning | G | uncommon | 6 | 43 | 77% | 4.0 | 11/44 |
| Falkenrath Marauders | R | rare | 4 | 28 | 75% | 3.2 | 5/16 |
| Falkenrath Noble | B | uncommon | 4 | 27 | 74% | 1.5 | 8/12 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Prey Upon | G | common | 9 | 63 | 71% | 2.7 | 13/35 |
| Sturmgeist | U | rare | 2 | 14 | 71% | 4.0 | 2/8 |
| Villagers of Estwald | G | common | 7 | 52 | 71% | 3.7 | 18/66 |
| Woodland Sleuth | G | common | 9 | 64 | 70% | 5.9 | 14/82 |
| Gatstaf Shepherd | G | uncommon | 8 | 59 | 68% | 4.6 | 12/55 |
| Cackling Counterpart | U | rare | 2 | 15 | 67% | 9.0 | 5/45 |
| Manor Gargoyle | C | rare | 3 | 23 | 65% | 3.3 | 3/10 |
| Morkrut Banshee | B | uncommon | 5 | 36 | 64% | 1.5 | 8/12 |
| Ghoulcaller's Chant | B | common | 9 | 63 | 63% | 8.1 | 14/113 |
| Hamlet Captain | G | uncommon | 6 | 45 | 62% | 4.9 | 8/39 |
| Diregraf Ghoul | B | uncommon | 4 | 29 | 62% | 6.5 | 6/39 |
| Victim of Night | B | common | 11 | 79 | 61% | 1.9 | 14/27 |
| Grave Bramble | G | common | 9 | 63 | 60% | 8.9 | 15/133 |
| Lumberknot | G | uncommon | 8 | 58 | 60% | 4.8 | 8/38 |
| Typhoid Rats | B | common | 11 | 83 | 60% | 3.5 | 14/49 |
| Furor of the Bitten | R | common | 2 | 15 | 60% | 11.6 | 16/183 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.2 | 5/21 |
| Kindercatch | G | common | 9 | 62 | 60% | 7.7 | 13/100 |
| Festerhide Boar | G | common | 12 | 84 | 60% | 4.8 | 17/82 |
| Thraben Sentry | W | common | 6 | 44 | 59% | 9.5 | 11/105 |
| Galvanic Juggernaut | C | uncommon | 8 | 60 | 58% | 1.2 | 8/10 |
| Bonds of Faith | W | common | 10 | 74 | 58% | 4.7 | 16/75 |
| Somberwald Spider | G | common | 11 | 83 | 58% | 4.1 | 14/57 |
| Corpse Lunge | B | common | 11 | 77 | 57% | 3.1 | 14/44 |
| Hysterical Blindness | U | common | 3 | 21 | 57% | 11.1 | 18/199 |
| Dearly Departed | W | rare | 2 | 14 | 57% | 2.5 | 2/5 |
| Disciple of Griselbrand | B | uncommon | 7 | 51 | 57% | 8.4 | 12/101 |
| Unruly Mob | W | common | 5 | 37 | 57% | 11.0 | 14/154 |
| Village Ironsmith | R | common | 5 | 37 | 57% | 8.3 | 12/100 |
| Moan of the Unhallowed | B | uncommon | 6 | 46 | 57% | 3.3 | 7/23 |
| Avacynian Priest | W | common | 10 | 71 | 56% | 5.3 | 15/79 |
| Ambush Viper | G | common | 9 | 64 | 56% | 5.0 | 12/60 |
| Curse of the Pierced Heart | R | common | 2 | 16 | 56% | 9.3 | 15/139 |
| Bump in the Night | B | common | 8 | 57 | 56% | 8.0 | 15/120 |
| Slayer of the Wicked | W | uncommon | 5 | 34 | 56% | 1.5 | 6/9 |
| Midnight Haunting | W | uncommon | 6 | 43 | 56% | 4.2 | 6/25 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Daybreak Ranger | G | rare | 1.0 | 3 | 1 |
| Bloodline Keeper | B | rare | 1.0 | 3 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.2 | 8 | 8 |
| Murder of Crows | U | uncommon | 1.4 | 8 | 8 |
| Fiend Hunter | W | uncommon | 1.4 | 10 | 9 |
| Slayer of the Wicked | W | uncommon | 1.5 | 6 | 5 |
| Chapel Geist | W | common | 1.5 | 16 | 11 |
| Falkenrath Noble | B | uncommon | 1.5 | 8 | 4 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Morkrut Banshee | B | uncommon | 1.5 | 8 | 5 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Claustrophobia | U | common | 1.8 | 15 | 9 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Essence of the Wild | G | mythic | 1.8 | 4 | 3 |
| Dead Weight | B | common | 1.9 | 14 | 11 |
| Victim of Night | B | common | 1.9 | 14 | 11 |
| Kruin Outlaw | R | rare | 2.0 | 4 | 4 |
| Geistcatcher's Rig | C | uncommon | 2.1 | 10 | 10 |
| Brimstone Volley | R | common | 2.1 | 14 | 10 |
| Screeching Bat | B | uncommon | 2.1 | 7 | 6 |
| Geistflame | R | common | 2.3 | 15 | 13 |
| Rebuke | W | common | 2.4 | 14 | 10 |
| Hollowhenge Scavenger | G | uncommon | 2.5 | 6 | 6 |
| Tribute to Hunger | B | uncommon | 2.5 | 6 | 6 |
| Prey Upon | G | common | 2.7 | 13 | 9 |
| Stitched Drake | U | common | 2.8 | 14 | 10 |
| Mayor of Avabruck | G | rare | 3.0 | 3 | 3 |
| Curse of Death's Hold | B | rare | 3.0 | 3 | 2 |
| Kessig Cagebreakers | G | rare | 3.0 | 3 | 3 |
| Into the Maw of Hell | R | uncommon | 3.1 | 9 | 9 |
