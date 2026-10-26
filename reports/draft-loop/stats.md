# Draft loop: what the nights add up to

6 night(s), 48 decks, 174 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 16 | 22-26-0 | 46% | 51-59 | 46% |
| unguided | 32 | 50-46-0 | 52% | 123-115 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 9 | 20-7-0 | 66% (64) |
| UB | 6 | 6-12-0 | 43% (46) |
| WR | 6 | 10-8-0 | 56% (41) |
| WG | 5 | 6-9-0 | 43% (35) |
| WU | 5 | 7-8-0 | 47% (38) |
| BR | 4 | 7-5-0 | 52% (29) |
| UR | 4 | 7-5-0 | 53% (30) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| WB | 2 | 0-6-0 | 8% (13) |
| R | 1 | 2-1-0 | 62% (8) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 12 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spider Spawning | G | uncommon | 4 | 29 | 79% | 4.3 | 9/39 |
| Falkenrath Marauders | R | rare | 4 | 28 | 75% | 3.2 | 5/16 |
| Villagers of Estwald | G | common | 6 | 45 | 73% | 3.7 | 14/52 |
| Sever the Bloodline | B | rare | 2 | 15 | 73% | 2.0 | 2/4 |
| Prey Upon | G | common | 7 | 49 | 71% | 2.9 | 10/29 |
| Sturmgeist | U | rare | 2 | 14 | 71% | 4.0 | 2/8 |
| Woodland Sleuth | G | common | 7 | 50 | 70% | 6.4 | 12/77 |
| Falkenrath Noble | B | uncommon | 3 | 20 | 70% | 1.6 | 7/11 |
| Gatstaf Shepherd | G | uncommon | 6 | 45 | 67% | 5.1 | 9/46 |
| Manor Gargoyle | C | rare | 2 | 15 | 67% | 3.0 | 2/6 |
| Cackling Counterpart | U | rare | 2 | 15 | 67% | 9.2 | 4/37 |
| Moldgraf Monstrosity | G | rare | 2 | 12 | 67% | 7.5 | 2/15 |
| Morkrut Banshee | B | uncommon | 4 | 29 | 66% | 1.6 | 7/11 |
| Diregraf Ghoul | B | uncommon | 3 | 20 | 65% | 6.8 | 5/34 |
| Ghoulcaller's Chant | B | common | 7 | 47 | 62% | 8.4 | 12/100 |
| Typhoid Rats | B | common | 10 | 74 | 61% | 3.5 | 13/45 |
| Kindercatch | G | common | 8 | 55 | 60% | 7.4 | 11/81 |
| Heretic's Punishment | R | rare | 2 | 15 | 60% | 4.2 | 4/17 |
| Thraben Sentry | W | common | 5 | 35 | 60% | 9.7 | 10/97 |
| Kruin Outlaw | R | rare | 3 | 22 | 59% | 2.0 | 3/6 |
| Galvanic Juggernaut | C | uncommon | 7 | 51 | 59% | 1.3 | 7/9 |
| Bonds of Faith | W | common | 9 | 65 | 58% | 4.9 | 14/68 |
| Victim of Night | B | common | 9 | 65 | 58% | 2.0 | 12/24 |
| Hamlet Captain | G | uncommon | 5 | 38 | 58% | 4.6 | 7/32 |
| Slayer of the Wicked | W | uncommon | 4 | 26 | 58% | 1.2 | 5/6 |
| Midnight Haunting | W | uncommon | 5 | 35 | 57% | 4.0 | 5/20 |
| Hysterical Blindness | U | common | 3 | 21 | 57% | 11.1 | 16/177 |
| Festerhide Boar | G | common | 11 | 77 | 57% | 4.9 | 15/73 |
| Frightful Delusion | U | common | 3 | 21 | 57% | 8.5 | 13/111 |
| Grave Bramble | G | common | 7 | 49 | 57% | 9.2 | 13/120 |
| Dearly Departed | W | rare | 2 | 14 | 57% | 2.5 | 2/5 |
| Into the Maw of Hell | R | uncommon | 8 | 58 | 57% | 3.1 | 8/25 |
| Lumberknot | G | uncommon | 7 | 51 | 57% | 4.7 | 7/33 |
| Unruly Mob | W | common | 5 | 37 | 57% | 11.2 | 12/134 |
| Moan of the Unhallowed | B | uncommon | 5 | 37 | 57% | 3.2 | 6/19 |
| Disciple of Griselbrand | B | uncommon | 6 | 44 | 57% | 8.8 | 10/88 |
| Village Ironsmith | R | common | 5 | 37 | 57% | 8.3 | 12/100 |
| Avacynian Priest | W | common | 9 | 62 | 56% | 5.4 | 13/70 |
| Curse of the Pierced Heart | R | common | 2 | 16 | 56% | 9.3 | 13/121 |
| Bump in the Night | B | common | 7 | 50 | 56% | 8.2 | 13/107 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Bloodline Keeper | B | rare | 1.0 | 3 | 2 |
| Slayer of the Wicked | W | uncommon | 1.2 | 5 | 4 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 7 | 7 |
| Murder of Crows | U | uncommon | 1.4 | 8 | 8 |
| Fiend Hunter | W | uncommon | 1.4 | 9 | 8 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Chapel Geist | W | common | 1.6 | 14 | 9 |
| Falkenrath Noble | B | uncommon | 1.6 | 7 | 3 |
| Morkrut Banshee | B | uncommon | 1.6 | 7 | 4 |
| Blasphemous Act | R | rare | 1.7 | 3 | 1 |
| Claustrophobia | U | common | 1.8 | 13 | 8 |
| Grimgrin, Corpse-Born | UB | mythic | 1.8 | 4 | 3 |
| Dead Weight | B | common | 1.8 | 12 | 9 |
| Essence of the Wild | G | mythic | 1.8 | 4 | 3 |
| Kruin Outlaw | R | rare | 2.0 | 3 | 3 |
| Victim of Night | B | common | 2.0 | 12 | 9 |
| Geistcatcher's Rig | C | uncommon | 2.1 | 9 | 9 |
| Screeching Bat | B | uncommon | 2.1 | 7 | 6 |
| Brimstone Volley | R | common | 2.2 | 12 | 9 |
| Rebuke | W | common | 2.4 | 12 | 9 |
| Geistflame | R | common | 2.5 | 13 | 11 |
| Hollowhenge Scavenger | G | uncommon | 2.5 | 6 | 6 |
| Tribute to Hunger | B | uncommon | 2.5 | 6 | 6 |
| Stitched Drake | U | common | 2.8 | 12 | 9 |
| Prey Upon | G | common | 2.9 | 10 | 7 |
| Mayor of Avabruck | G | rare | 3.0 | 3 | 3 |
| Curse of Death's Hold | B | rare | 3.0 | 3 | 2 |
| Kessig Cagebreakers | G | rare | 3.0 | 3 | 3 |
| Into the Maw of Hell | R | uncommon | 3.1 | 8 | 8 |
| Moan of the Unhallowed | B | uncommon | 3.2 | 6 | 5 |
