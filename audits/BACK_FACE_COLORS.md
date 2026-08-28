# Back-face colour indicators (CR 204.2)

A double-faced card's back face has no mana cost, so its colour comes from the
printed **colour indicator** — the small filled circle before the type line.
Without one in `back_face_data`, `colors_of` falls back to the (absent) mana
cost and the transformed permanent is **colourless**, dodging every colour-based
effect in the set.

This file tracks which of the 20 declared back faces have had their colour
established from an external source. **20 of 20 established.** All colours were looked up from external sources
(web search over Scryfall / mtg.wtf / Gatherer results, one card at a time)
and are pinned by
`card_data_invariants.rs::every_back_face_declares_the_colour_its_indicator_prints`,
which also fails the build on any future back face missing from its table.

A back face's colour is *not* derivable from the front's: Garruk Relentless is
mono-green and Garruk, the Veil-Cursed is black-green. Every entry below has to
be looked up; none may be guessed.

Regenerated from `mtg-engine/src/cards/isd/*.rs` on 2026-08-28; the last 13
indicators were established and added the same day.

| card file | back face | colour indicator |
|---|---|---|
| `bloodline_keeper` | Lord of Lineage | **Black** |
| `civilized_scholar` | Homicidal Brute | **Red** |
| `cloistered_youth` | Unholy Fiend | **Black** |
| `daybreak_ranger` | Nightfall Predator | **Green** |
| `delver_of_secrets` | Insectile Aberration | **Blue** |
| `garruk_relentless` | Garruk, the Veil-Cursed | **Black and Green** |
| `gatstaf_shepherd` | Gatstaf Howler | **Green** |
| `grizzled_outcasts` | Krallenhorde Wantons | **Green** |
| `hanweir_watchkeep` | Bane of Hanweir | **Red** |
| `instigator_gang` | Wildblood Pack | **Red** |
| `kruin_outlaw` | Terror of Kruin Pass | **Red** |
| `ludevics_test_subject` | Ludevic's Abomination | **Blue** |
| `mayor_of_avabruck` | Howlpack Alpha | **Green** |
| `reckless_waif` | Merciless Predator | **Red** |
| `screeching_bat` | Stalking Vampire | **Black** |
| `thraben_sentry` | Thraben Militia | **White** |
| `tormented_pariah` | Rampaging Werewolf | **Red** |
| `ulvenwald_mystics` | Ulvenwald Primordials | **Green** |
| `village_ironsmith` | Ironfang | **Red** |
| `villagers_of_estwald` | Howlpack of Estwald | **Green** |
