# Back-face color indicators still to establish

CR 204.2: a transforming back face has no mana cost, so its colors come from
the color indicator printed beside its type line. `CardData::color_indicator`
carries it and `colors_of` reads it; a back face that does not state one is
treated as colorless, which is wrong for every card below.

`data/oracle_cache.json` does not record back-face colors and Scryfall is not
reachable from this environment, so each one has to be established from an
external source individually. That happens in each card's own audit — every
card marked below is still ahead on `ISD_FULL_AUDIT_TODO.md`.

Assuming the back face matches the front's colour is not available as a
shortcut: Garruk Relentless is green and Garruk, the Veil-Cursed is
black-green.

Established: 6 of 20.

| card | back face | indicator |
|---|---|---|
| `gatstaf_shepherd` | Gatstaf Howler | Green |
| `grizzled_outcasts` | Krallenhorde Wantons | Green |
| `reckless_waif` | Merciless Predator | Red |
| `thraben_sentry` | Thraben Militia | White |
| `tormented_pariah` | Rampaging Werewolf | Red |
| `villagers_of_estwald` | Howlpack of Estwald | Green |
| `bloodline_keeper` | Lord of Lineage | **not yet established** |
| `civilized_scholar` | Homicidal Brute | **not yet established** |
| `cloistered_youth` | Unholy Fiend | **not yet established** |
| `daybreak_ranger` | Nightfall Predator | **not yet established** |
| `delver_of_secrets` | Insectile Aberration | **not yet established** |
| `garruk_relentless` | Garruk, the Veil-Cursed | **not yet established** |
| `hanweir_watchkeep` | Bane of Hanweir | **not yet established** |
| `instigator_gang` | Wildblood Pack | **not yet established** |
| `kruin_outlaw` | Terror of Kruin Pass | **not yet established** |
| `ludevics_test_subject` | Ludevic's Abomination | **not yet established** |
| `mayor_of_avabruck` | Howlpack Alpha | **not yet established** |
| `screeching_bat` | Stalking Vampire | **not yet established** |
| `ulvenwald_mystics` | Ulvenwald Primordials | **not yet established** |
| `village_ironsmith` | Ironfang | **not yet established** |
