# Draft loop: what the nights add up to

10 night(s), 80 decks, 291 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 32 | 42-54-0 | 44% | 106-125 | 46% |
| unguided | 48 | 78-66-0 | 54% | 185-166 | 53% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 16 | 30-18-0 | 58% (115) |
| WU | 10 | 15-15-0 | 51% (73) |
| BR | 9 | 16-11-0 | 54% (63) |
| UB | 9 | 9-18-0 | 44% (70) |
| WR | 9 | 15-12-0 | 56% (63) |
| UR | 8 | 11-13-0 | 45% (58) |
| WG | 7 | 10-11-0 | 47% (51) |
| WB | 4 | 4-8-0 | 34% (29) |
| R | 2 | 3-3-0 | 50% (16) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 20 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Spider Spawning | G | uncommon | 9 | 67 | 70% | 3.6 | 14/51 |
| Falkenrath Noble | B | uncommon | 8 | 55 | 67% | 1.4 | 12/17 |
| Morkrut Banshee | B | uncommon | 6 | 44 | 64% | 1.6 | 9/14 |
| Woodland Sleuth | G | common | 13 | 94 | 62% | 5.2 | 20/105 |
| Typhoid Rats | B | common | 17 | 128 | 61% | 3.4 | 22/74 |
| Gatstaf Shepherd | G | uncommon | 13 | 98 | 60% | 4.4 | 19/83 |
| Slayer of the Wicked | W | uncommon | 8 | 55 | 60% | 1.5 | 11/17 |
| Prey Upon | G | common | 14 | 100 | 60% | 2.4 | 19/46 |
| Galvanic Juggernaut | C | uncommon | 11 | 79 | 59% | 1.3 | 11/14 |
| Silver-Inlaid Dagger | C | uncommon | 7 | 49 | 59% | 8.2 | 12/99 |
| Thraben Sentry | W | common | 6 | 44 | 59% | 9.5 | 11/105 |
| Villagers of Estwald | G | common | 13 | 97 | 59% | 3.1 | 33/101 |
| Cackling Counterpart | U | rare | 4 | 29 | 59% | 9.2 | 9/83 |
| Hamlet Captain | G | uncommon | 8 | 60 | 58% | 5.8 | 12/70 |
| Ashmouth Hound | R | common | 12 | 86 | 58% | 3.8 | 19/72 |
| Into the Maw of Hell | R | uncommon | 12 | 88 | 58% | 3.8 | 13/50 |
| Furor of the Bitten | R | common | 5 | 35 | 57% | 12.0 | 23/276 |
| Curse of the Pierced Heart | R | common | 5 | 35 | 57% | 9.6 | 21/201 |
| Dearly Departed | W | rare | 3 | 21 | 57% | 3.0 | 3/9 |
| Ghoulcaller's Chant | B | common | 14 | 102 | 57% | 7.9 | 21/165 |
| Lumberknot | G | uncommon | 11 | 81 | 57% | 3.8 | 12/46 |
| Avacynian Priest | W | common | 14 | 99 | 57% | 5.0 | 21/106 |
| Victim of Night | B | common | 15 | 106 | 57% | 1.8 | 19/34 |
| Somberwald Spider | G | common | 15 | 114 | 56% | 4.5 | 21/94 |
| Kindercatch | G | common | 15 | 109 | 56% | 6.3 | 20/127 |
| Grave Bramble | G | common | 13 | 93 | 56% | 8.3 | 21/175 |
| Diregraf Ghoul | B | uncommon | 8 | 59 | 56% | 5.9 | 11/65 |
| Skirsdag Cultist | R | uncommon | 6 | 45 | 56% | 8.4 | 11/92 |
| Silverchase Fox | W | common | 14 | 101 | 55% | 7.7 | 22/170 |
| Unruly Mob | W | common | 9 | 65 | 55% | 11.1 | 20/222 |
| Bonds of Faith | W | common | 14 | 103 | 55% | 4.2 | 23/97 |
| Hysterical Blindness | U | common | 4 | 29 | 55% | 10.8 | 25/271 |
| Hollowhenge Scavenger | G | uncommon | 7 | 51 | 55% | 3.1 | 8/25 |
| Riot Devils | R | common | 14 | 101 | 54% | 7.9 | 20/158 |
| Essence of the Wild | G | mythic | 3 | 22 | 55% | 2.4 | 5/12 |
| Village Ironsmith | R | common | 6 | 44 | 55% | 8.3 | 14/116 |
| Avacyn's Pilgrim | G | common | 5 | 37 | 54% | 10.3 | 21/216 |
| Moan of the Unhallowed | B | uncommon | 10 | 74 | 54% | 2.7 | 12/32 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Angelic Overseer | W | mythic | 1.0 | 3 | 1 |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 3 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 11 | 11 |
| Bloodgift Demon | B | rare | 1.3 | 3 | 3 |
| Falkenrath Noble | B | uncommon | 1.4 | 12 | 8 |
| Slayer of the Wicked | W | uncommon | 1.5 | 11 | 8 |
| Murder of Crows | U | uncommon | 1.5 | 12 | 10 |
| Fiend Hunter | W | uncommon | 1.5 | 14 | 12 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Daybreak Ranger | G | rare | 1.6 | 5 | 3 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Morkrut Banshee | B | uncommon | 1.6 | 9 | 6 |
| Chapel Geist | W | common | 1.7 | 20 | 14 |
| Claustrophobia | U | common | 1.8 | 21 | 14 |
| Victim of Night | B | common | 1.8 | 19 | 15 |
| Dead Weight | B | common | 1.9 | 21 | 16 |
| Brimstone Volley | R | common | 1.9 | 21 | 13 |
| Kruin Outlaw | R | rare | 2.1 | 7 | 7 |
| Screeching Bat | B | uncommon | 2.1 | 7 | 6 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Blasphemous Act | R | rare | 2.2 | 4 | 1 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Tribute to Hunger | B | uncommon | 2.4 | 11 | 11 |
| Prey Upon | G | common | 2.4 | 19 | 14 |
| Manor Gargoyle | C | rare | 2.4 | 5 | 5 |
| Geistcatcher's Rig | C | uncommon | 2.5 | 13 | 13 |
| Rebuke | W | common | 2.5 | 21 | 15 |
