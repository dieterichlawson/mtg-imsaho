# Draft loop: what the nights add up to

9 night(s), 72 decks, 263 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| guided | 28 | 38-46-0 | 45% | 95-106 | 47% |
| unguided | 44 | 70-62-0 | 53% | 168-157 | 52% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| BG | 15 | 29-16-0 | 59% (107) |
| WU | 9 | 15-12-0 | 55% (66) |
| UB | 8 | 7-17-0 | 42% (62) |
| UR | 8 | 11-13-0 | 45% (58) |
| WR | 8 | 13-11-0 | 54% (57) |
| BR | 6 | 11-7-0 | 55% (44) |
| WG | 6 | 8-10-0 | 47% (43) |
| WB | 4 | 4-8-0 | 34% (29) |
| R | 2 | 3-3-0 | 50% (16) |
| RG | 2 | 3-3-0 | 50% (14) |
| UG | 2 | 1-5-0 | 38% (16) |
| U | 1 | 1-2-0 | 33% (6) |
| W | 1 | 2-1-0 | 50% (8) |

## Cards in winning decks (at least 18 games in a deck)

`games` are the games the card was in the deck for, sideboarding included, and `deck game win` is how many of them its deck won; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Falkenrath Marauders | R | rare | 5 | 36 | 75% | 2.8 | 6/17 |
| Sever the Bloodline | B | rare | 3 | 23 | 74% | 1.7 | 3/5 |
| Spider Spawning | G | uncommon | 8 | 59 | 73% | 3.8 | 13/49 |
| Cackling Counterpart | U | rare | 3 | 22 | 73% | 9.4 | 7/66 |
| Woodland Sleuth | G | common | 11 | 78 | 64% | 5.3 | 18/96 |
| Morkrut Banshee | B | uncommon | 6 | 44 | 64% | 1.6 | 9/14 |
| Falkenrath Noble | B | uncommon | 7 | 49 | 63% | 1.4 | 11/15 |
| Manor Gargoyle | C | rare | 4 | 30 | 63% | 2.8 | 4/11 |
| Woodland Cemetery | C | rare | 3 | 19 | 63% | 8.2 | 5/41 |
| Gatstaf Shepherd | G | uncommon | 11 | 82 | 62% | 4.4 | 16/70 |
| Unruly Mob | W | common | 7 | 50 | 62% | 11.1 | 18/199 |
| Prey Upon | G | common | 12 | 84 | 62% | 2.5 | 17/43 |
| Slayer of the Wicked | W | uncommon | 7 | 47 | 62% | 1.6 | 9/14 |
| Villagers of Estwald | G | common | 11 | 81 | 60% | 3.1 | 28/86 |
| Avacynian Priest | W | common | 13 | 92 | 60% | 5.2 | 19/99 |
| Typhoid Rats | B | common | 14 | 106 | 59% | 3.5 | 19/66 |
| Bonds of Faith | W | common | 12 | 88 | 59% | 4.4 | 20/88 |
| Thraben Sentry | W | common | 6 | 44 | 59% | 9.5 | 11/105 |
| Moan of the Unhallowed | B | uncommon | 8 | 61 | 59% | 3.0 | 10/30 |
| Galvanic Juggernaut | C | uncommon | 10 | 73 | 59% | 1.3 | 10/13 |
| Hamlet Captain | G | uncommon | 8 | 60 | 58% | 5.4 | 11/59 |
| Lumberknot | G | uncommon | 10 | 73 | 58% | 4.3 | 10/43 |
| Ghoulcaller's Chant | B | common | 13 | 94 | 57% | 7.7 | 19/147 |
| Midnight Haunting | W | uncommon | 8 | 56 | 57% | 4.4 | 9/40 |
| Dearly Departed | W | rare | 3 | 21 | 57% | 3.0 | 3/9 |
| Kindercatch | G | common | 13 | 93 | 57% | 6.6 | 18/119 |
| Somberwald Spider | G | common | 14 | 106 | 57% | 4.3 | 18/77 |
| Grave Bramble | G | common | 12 | 85 | 56% | 8.4 | 19/159 |
| Hollowhenge Scavenger | G | uncommon | 6 | 43 | 56% | 2.7 | 7/19 |
| Traitorous Blood | R | common | 4 | 29 | 55% | 10.0 | 21/209 |
| Hysterical Blindness | U | common | 4 | 29 | 55% | 10.9 | 23/251 |
| Silverchase Fox | W | common | 12 | 87 | 55% | 7.7 | 20/153 |
| Into the Maw of Hell | R | uncommon | 11 | 82 | 55% | 3.7 | 12/44 |
| Diregraf Ghoul | B | uncommon | 7 | 51 | 55% | 5.7 | 10/57 |
| Essence of the Wild | G | mythic | 3 | 22 | 55% | 2.4 | 5/12 |
| Village Ironsmith | R | common | 6 | 44 | 55% | 8.3 | 14/116 |
| Ashmouth Hound | R | common | 10 | 74 | 54% | 3.8 | 17/64 |
| Avacyn's Pilgrim | G | common | 5 | 37 | 54% | 10.4 | 19/197 |
| Festerhide Boar | G | common | 16 | 113 | 54% | 4.7 | 22/103 |
| Victim of Night | B | common | 14 | 100 | 54% | 1.8 | 18/33 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Angelic Overseer | W | mythic | 1.0 | 3 | 1 |
| Mikaeus, the Lunarch | W | mythic | 1.0 | 3 | 2 |
| Bloodline Keeper | B | rare | 1.0 | 3 | 2 |
| Galvanic Juggernaut | C | uncommon | 1.3 | 10 | 10 |
| Falkenrath Noble | B | uncommon | 1.4 | 11 | 7 |
| Murder of Crows | U | uncommon | 1.5 | 11 | 10 |
| Fiend Hunter | W | uncommon | 1.5 | 13 | 12 |
| Geist-Honored Monk | W | rare | 1.5 | 4 | 3 |
| Claustrophobia | U | common | 1.6 | 19 | 12 |
| Slayer of the Wicked | W | uncommon | 1.6 | 9 | 7 |
| Daybreak Ranger | G | rare | 1.6 | 5 | 3 |
| Grimgrin, Corpse-Born | UB | mythic | 1.6 | 5 | 4 |
| Morkrut Banshee | B | uncommon | 1.6 | 9 | 6 |
| Chapel Geist | W | common | 1.7 | 18 | 12 |
| Sever the Bloodline | B | rare | 1.7 | 3 | 3 |
| Victim of Night | B | common | 1.8 | 18 | 14 |
| Dead Weight | B | common | 1.9 | 18 | 14 |
| Brimstone Volley | R | common | 1.9 | 19 | 12 |
| Kruin Outlaw | R | rare | 2.0 | 6 | 6 |
| Screeching Bat | B | uncommon | 2.1 | 7 | 6 |
| Geistcatcher's Rig | C | uncommon | 2.2 | 12 | 12 |
| Blasphemous Act | R | rare | 2.2 | 4 | 1 |
| Liliana of the Veil | B | mythic | 2.3 | 3 | 3 |
| Geistflame | R | common | 2.4 | 20 | 16 |
| Essence of the Wild | G | mythic | 2.4 | 5 | 3 |
| Tribute to Hunger | B | uncommon | 2.4 | 9 | 9 |
| Rebuke | W | common | 2.5 | 19 | 14 |
| Prey Upon | G | common | 2.5 | 17 | 12 |
| Divine Reckoning | W | rare | 2.7 | 3 | 0 |
| Hollowhenge Scavenger | G | uncommon | 2.7 | 7 | 6 |
