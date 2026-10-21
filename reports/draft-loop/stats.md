# Draft loop: what the nights add up to

1 night(s), 8 decks, 31 games. Written by `scripts/draft-loop/ledger.py stats`; the data is `nights/*.json`.

## Guided against unguided seats

Even seats draft with `guide.md`, odd seats without it, every night.

| seats | n | matches W-L-D | match win | games W-L | game win |
| --- | ---: | ---: | ---: | ---: | ---: |
| unguided | 8 | 12-12-0 | 50% | 31-31 | 50% |

## Colour pairs

A deck's colours are those with four or more of its spells (or three on five basics); a lighter colour is a splash.

| colours | decks | matches W-L-D | game win |
| --- | ---: | ---: | ---: |
| UB | 2 | 2-4-0 | 44% (16) |
| BG | 1 | 3-0-0 | 75% (8) |
| RG | 1 | 2-1-0 | 62% (8) |
| UR | 1 | 1-2-0 | 38% (8) |
| W | 1 | 2-1-0 | 50% (8) |
| WG | 1 | 1-2-0 | 43% (7) |
| WR | 1 | 1-2-0 | 43% (7) |

## Cards in winning decks (at least 6 games in a deck)

`deck game win` is the game win rate of the decks that ran the card; `avg pick` is its pick within a pack (1-14) when taken.

| card | colours | rarity | decks | games | deck game win | avg pick | taken / seen |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spider Spawning | G | uncommon | 1 | 8 | 75% | 6.0 | 2/12 |
| Woodland Sleuth | G | common | 1 | 8 | 75% | 9.0 | 2/18 |
| Manor Skeleton | B | common | 1 | 8 | 75% | 13.0 | 2/25 |
| Ghoulcaller's Chant | B | common | 1 | 8 | 75% | 12.5 | 2/24 |
| Festerhide Boar | G | common | 1 | 8 | 75% | 6.3 | 3/19 |
| Typhoid Rats | B | common | 1 | 8 | 75% | 2.5 | 2/5 |
| Kindercatch | G | common | 1 | 8 | 75% | 9.5 | 2/19 |
| Hollowhenge Scavenger | G | uncommon | 1 | 8 | 75% | 3.0 | 1/3 |
| Tribute to Hunger | B | uncommon | 1 | 8 | 75% | 1.0 | 1/1 |
| Trepanation Blade | C | uncommon | 1 | 8 | 75% | 2.0 | 1/2 |
| Prey Upon | G | common | 1 | 8 | 75% | 3.0 | 1/3 |
| Boneyard Wurm | G | uncommon | 1 | 8 | 75% | 7.0 | 1/7 |
| Grave Bramble | G | common | 1 | 8 | 75% | 11.0 | 2/22 |
| Morkrut Banshee | B | uncommon | 1 | 8 | 75% | 1.0 | 1/1 |
| Lumberknot | G | uncommon | 1 | 8 | 75% | 8.0 | 1/8 |
| Villagers of Estwald | G | common | 2 | 16 | 69% | 4.8 | 5/24 |
| Gatstaf Shepherd | G | uncommon | 2 | 16 | 69% | 6.3 | 3/19 |
| Orchard Spirit | G | common | 2 | 16 | 69% | 5.0 | 3/15 |
| Darkthicket Wolf | G | common | 2 | 16 | 69% | 5.0 | 2/10 |
| Victim of Night | B | common | 2 | 16 | 69% | 3.0 | 2/6 |
| Brain Weevil | B | common | 1 | 8 | 62% | 8.0 | 2/16 |
| Selhoff Occultist | U | common | 1 | 8 | 62% | 11.0 | 2/22 |
| Essence of the Wild | G | mythic | 1 | 8 | 62% | 2.0 | 1/2 |
| Kruin Outlaw | R | rare | 1 | 8 | 62% | 3.0 | 1/3 |
| Reckless Waif | R | uncommon | 1 | 8 | 62% | 4.0 | 2/8 |
| Bloodcrazed Neonate | R | common | 1 | 8 | 62% | 9.5 | 2/19 |
| Hamlet Captain | G | uncommon | 1 | 8 | 62% | 8.0 | 1/8 |
| Skirsdag Cultist | R | uncommon | 1 | 8 | 62% | 9.0 | 1/9 |
| Stitched Drake | U | common | 1 | 8 | 62% | 2.5 | 2/5 |
| Feral Ridgewolf | R | common | 1 | 8 | 62% | 6.5 | 2/13 |
| Somberwald Spider | G | common | 1 | 8 | 62% | 4.0 | 2/8 |
| Falkenrath Marauders | R | rare | 1 | 8 | 62% | 4.0 | 1/4 |
| Heretic's Punishment | R | rare | 1 | 8 | 62% | 5.0 | 1/5 |
| Silent Departure | U | common | 1 | 8 | 62% | 8.0 | 2/16 |
| Frightful Delusion | U | common | 1 | 8 | 62% | 7.0 | 2/14 |
| Creepy Doll | C | rare | 1 | 8 | 62% | 7.0 | 2/14 |
| Disciple of Griselbrand | B | uncommon | 1 | 8 | 62% | 12.0 | 2/24 |
| Sever the Bloodline | B | rare | 1 | 8 | 62% | 1.0 | 1/1 |
| Ghoulraiser | B | common | 1 | 8 | 62% | 6.0 | 1/6 |
| Geistcatcher's Rig | C | uncommon | 2 | 15 | 53% | 3.0 | 2/6 |

## Taken earliest (picked at least 3 times)

| card | colours | rarity | avg pick | picked | decks |
| --- | --- | --- | ---: | ---: | ---: |
| Chapel Geist | W | common | 1.7 | 3 | 1 |
| Villagers of Estwald | G | common | 4.8 | 5 | 2 |
| Orchard Spirit | G | common | 5.0 | 3 | 2 |
| Ghostly Possession | W | common | 5.3 | 3 | 1 |
| Elder Cathar | W | common | 5.7 | 3 | 1 |
| Festerhide Boar | G | common | 6.3 | 3 | 1 |
| Gatstaf Shepherd | G | uncommon | 6.3 | 3 | 2 |
| Tormented Pariah | R | common | 6.4 | 5 | 3 |
| Forbidden Alchemy | U | common | 7.0 | 3 | 2 |
| Delver of Secrets | U | common | 7.6 | 5 | 2 |
| Kessig Wolf | R | common | 7.7 | 3 | 1 |
| Avacyn's Pilgrim | G | common | 8.0 | 3 | 1 |
| Markov Patrician | B | common | 8.0 | 3 | 2 |
| Altar's Reap | B | common | 8.7 | 3 | 1 |
| Thraben Purebloods | W | common | 9.3 | 3 | 2 |
| One-Eyed Scarecrow | C | common | 9.3 | 3 | 1 |
| Traitorous Blood | R | common | 9.7 | 3 | 1 |
| Feeling of Dread | W | common | 11.0 | 3 | 2 |
| Moonmist | G | common | 11.0 | 3 | 0 |
| Fortress Crab | U | common | 11.3 | 3 | 0 |
| Ancient Grudge | R | common | 12.3 | 3 | 0 |
| Wooden Stake | C | common | 12.5 | 4 | 0 |
| Hysterical Blindness | U | common | 12.7 | 3 | 0 |
| Ghoulcaller's Bell | C | common | 13.3 | 3 | 0 |
