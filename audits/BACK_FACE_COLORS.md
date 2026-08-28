# Back-face colour indicators (CR 204.2)

A double-faced card's back face has no mana cost, so its colour comes from the
printed **colour indicator** — the small filled circle before the type line.
Without one in `back_face_data`, `colors_of` falls back to the (absent) mana
cost and the transformed permanent is **colourless**, dodging every colour-based
effect in the set.

This file tracks which of the 20 declared back faces have had their colour
established from an external source. **7 of 20 established.**

A back face's colour is *not* derivable from the front's: Garruk Relentless is
mono-green and Garruk, the Veil-Cursed is black-green. Every entry below has to
be looked up; none may be guessed.

Regenerated from `mtg-engine/src/cards/isd/*.rs` on 2026-08-28.

| card file | back face | colour indicator |
|---|---|---|
| `bloodline_keeper` | Lord of Lineage | **not yet established** |
| `civilized_scholar` | Homicidal Brute | **not yet established** |
| `cloistered_youth` | Unholy Fiend | **not yet established** |
| `daybreak_ranger` | Nightfall Predator | **not yet established** |
| `delver_of_secrets` | Insectile Aberration | **not yet established** |
| `garruk_relentless` | Garruk, the Veil-Cursed | **not yet established** |
| `gatstaf_shepherd` | Gatstaf Howler | **Green** |
| `grizzled_outcasts` | Krallenhorde Wantons | **Green** |
| `hanweir_watchkeep` | Bane of Hanweir | **not yet established** |
| `instigator_gang` | Wildblood Pack | **not yet established** |
| `kruin_outlaw` | Terror of Kruin Pass | **not yet established** |
| `ludevics_test_subject` | Ludevic's Abomination | **Blue** |
| `mayor_of_avabruck` | Howlpack Alpha | **not yet established** |
| `reckless_waif` | Merciless Predator | **Red** |
| `screeching_bat` | Stalking Vampire | **not yet established** |
| `thraben_sentry` | Thraben Militia | **White** |
| `tormented_pariah` | Rampaging Werewolf | **Red** |
| `ulvenwald_mystics` | Ulvenwald Primordials | **not yet established** |
| `village_ironsmith` | Ironfang | **not yet established** |
| `villagers_of_estwald` | Howlpack of Estwald | **Green** |
