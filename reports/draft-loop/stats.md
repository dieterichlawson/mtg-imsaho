# Draft loop: what the nights add up to

11 night(s), 88 decks, 320 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 36 | 47-61-0 | 44% | 119-142 | 46% |
| unguided | 52 | 85-71-0 | 54% | 201-178 | 53% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 18 | 32-22-0 | 56% (130) |
| WU | 12 | 20-16-0 | 53% (88) |
| BR | 10 | 18-12-0 | 55% (69) |
| WR | 10 | 16-14-0 | 54% (71) |
| UB | 9 | 9-18-0 | 44% (70) |
| UR | 9 | 12-15-0 | 45% (65) |
| WG | 7 | 10-11-0 | 47% (51) |
| WB | 5 | 5-10-0 | 36% (36) |
| R | 2 | 3-3-0 | 50% (16) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 22 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 4 | 31 | 71% | 1.5 | 4/6 |
| Spider Spawning | G | uncommon | 10 | 74 | 65% | 3.3 | 16/53 |
| Morkrut Banshee | B | uncommon | 7 | 52 | 63% | 1.5 | 10/15 |
| Woodland Sleuth | G | common | 14 | 102 | 62% | 5.1 | 22/112 |
| Falkenrath Noble | B | uncommon | 9 | 62 | 61% | 1.4 | 13/18 |
| Gatstaf Shepherd | G | uncommon | 13 | 98 | 60% | 4.4 | 19/83 |
| Slayer of the Wicked | W | uncommon | 11 | 77 | 60% | 1.6 | 14/22 |
| Galvanic Juggernaut | C | uncommon | 13 | 94 | 60% | 1.3 | 13/17 |
| Villagers of Estwald | G | common | 13 | 97 | 59% | 3.1 | 33/101 |
| Hamlet Captain | G | uncommon | 9 | 68 | 59% | 5.9 | 13/77 |
| Cackling Counterpart | U | rare | 4 | 29 | 59% | 9.2 | 9/83 |
| Into the Maw of Hell | R | uncommon | 13 | 94 | 59% | 3.6 | 14/51 |
| Typhoid Rats | B | common | 18 | 135 | 59% | 3.4 | 24/81 |
| Thraben Sentry | W | common | 8 | 60 | 58% | 9.6 | 14/134 |
| Ashmouth Hound | R | common | 12 | 86 | 58% | 4.0 | 21/83 |
| Silver-Inlaid Dagger | C | uncommon | 8 | 56 | 57% | 7.9 | 13/103 |
| Furor of the Bitten | R | common | 5 | 35 | 57% | 12.0 | 25/299 |
| Prey Upon | G | common | 15 | 107 | 57% | 2.4 | 21/51 |
| Lumberknot | G | uncommon | 11 | 81 | 57% | 3.7 | 13/48 |
| Avacynian Priest | W | common | 16 | 115 | 57% | 5.1 | 23/117 |
| Diregraf Ghoul | B | uncommon | 8 | 59 | 56% | 5.9 | 11/65 |
| Midnight Haunting | W | uncommon | 11 | 77 | 56% | 4.4 | 12/53 |
| Hysterical Blindness | U | common | 5 | 36 | 56% | 10.5 | 27/284 |
| Silverchase Fox | W | common | 15 | 108 | 56% | 7.6 | 24/182 |
| Unruly Mob | W | common | 11 | 81 | 56% | 11.1 | 22/244 |
| Skirsdag Cultist | R | uncommon | 6 | 45 | 56% | 8.2 | 12/98 |
| Selfless Cathar | W | common | 10 | 74 | 55% | 10.7 | 22/236 |
| Curse of the Pierced Heart | R | common | 7 | 49 | 55% | 9.3 | 23/213 |
| Rally the Peasants | W | uncommon | 10 | 71 | 55% | 8.0 | 17/136 |
| Hollowhenge Scavenger | G | uncommon | 7 | 51 | 55% | 3.1 | 8/25 |
| Ghoulcaller's Chant | B | common | 16 | 117 | 55% | 7.7 | 23/178 |
| Tribute to Hunger | B | uncommon | 12 | 86 | 55% | 2.3 | 12/28 |
| Voiceless Spirit | W | common | 19 | 134 | 54% | 4.4 | 23/101 |
| Essence of the Wild | G | mythic | 3 | 22 | 55% | 2.4 | 5/12 |
| Elite Inquisitor | W | rare | 3 | 22 | 55% | 4.0 | 4/16 |
| Somberwald Spider | G | common | 17 | 129 | 54% | 4.2 | 23/97 |
| Avacyn's Pilgrim | G | common | 5 | 37 | 54% | 10.4 | 24/250 |
| Moan of the Unhallowed | B | uncommon | 10 | 74 | 54% | 2.6 | 13/34 |
| Bonds of Faith | W | common | 15 | 111 | 54% | 4.2 | 25/106 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 4 | 3 |
| Garruk Relentless | G | mythic | 1.0 | 3 | 0 |
| Angelic Overseer | W | mythic | 1.2 | 4 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 13 | 13 |
| Bloodgift Demon | B | rare | 1.3 | 3 | 3 |
| Devil's Play | R | rare | 1.3 | 3 | 2 |
| Falkenrath Noble | B | uncommon | 1.4 | 13 | 9 |
| Fiend Hunter | W | uncommon | 1.5 | 14 | 12 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Sever the Bloodline | B | rare | 1.5 | 4 | 4 |
| Morkrut Banshee | B | uncommon | 1.5 | 10 | 7 |
| Slayer of the Wicked | W | uncommon | 1.6 | 14 | 11 |
| Daybreak Ranger | G | rare | 1.6 | 5 | 3 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Claustrophobia | U | common | 1.7 | 23 | 16 |
| Chapel Geist | W | common | 1.7 | 22 | 16 |
| Murder of Crows | U | uncommon | 1.7 | 13 | 11 |
| Dead Weight | B | common | 1.8 | 23 | 17 |
| Brimstone Volley | R | common | 1.8 | 23 | 14 |
| Victim of Night | B | common | 1.8 | 21 | 17 |
| Kruin Outlaw | R | rare | 2.1 | 7 | 7 |
| Divine Reckoning | W | rare | 2.2 | 4 | 0 |
| Blasphemous Act | R | rare | 2.2 | 4 | 1 |
| Tribute to Hunger | B | uncommon | 2.3 | 12 | 12 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Geistcatcher's Rig | C | uncommon | 2.4 | 14 | 14 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Prey Upon | G | common | 2.4 | 21 | 15 |
| Manor Gargoyle | C | rare | 2.4 | 5 | 5 |
