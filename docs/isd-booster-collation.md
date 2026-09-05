# Innistrad (ISD) Booster Pack Collation

This document describes how Innistrad booster packs are physically constructed, based
on print sheet collation data. The goal is to enable a pack simulator that reproduces
not just the correct marginal card frequencies, but also the correct conditional
structure (which cards can and cannot appear together in the same pack).

## Sources

All print sheet data comes from **The Collation Project** by lethe:

- ISD-specific page: https://www.lethe.xyz/mtg/collation/isd.html
- C1/C2 common collation explanation: https://www.lethe.xyz/mtg/collation/c1-c2-common-collation.html
- Sequential collation explanation: https://www.lethe.xyz/mtg/collation/sequential-collation.html

Foil rate data comes from:

- Mark Rosewater, "Project Booster Fun" (2019): confirmed pre-M20 foil rate is 1:67 cards = 22.5% of packs
- MTG Salvation forums "How do Booster Pack Foils work?" thread: the 11/16, 3/16, 7/128, 1/128, 1/16 rarity model
- Ultimate Masters 32-box empirical data on lethe.xyz (549C/161U/52R/6M from 768 non-basic foils): validates the MTGS model

The raw HTML from the ISD collation page was fetched via `curl` and parsed
programmatically. All card names and sheet positions were extracted from `<img title="...">` tags
in the HTML, preserving the exact sheet order. The data was validated against
expected run sizes (all 12 runs matched their expected slot counts exactly).

## Overview

Innistrad uses **sequential collation** with **C1/C2 common collation**, modified for
9 commons per pack instead of the standard 10 (because the DFC slot replaces one
common slot).

An ISD booster pack contains 14 draft-relevant cards:

| Slot       | Count | Source                      |
|------------|-------|-----------------------------|
| Common     | 9     | Common print runs (A, B, C) |
| Uncommon   | 3     | Uncommon print runs (A, B)  |
| Rare/Mythic| 1     | Rare print sheets (1 or 2)  |
| DFC        | 1     | DFC print sheet             |

Plus 1 basic land or checklist card (not draft-relevant) and 1 ad card.

Approximately 22.5% of packs also contain a foil card that replaces one of the above
slots (see Foils section).

## Sequential Collation

Cards are printed on physical sheets containing multiple copies of each card arranged
in a grid. The sheet is read left-to-right, top-to-bottom, wrapping from the last
position back to the first. This circular sequence is called a **run**.

Each run acts as an independent circular buffer with a cursor. To produce a pack, the
factory takes the next N cards from each run, advancing that run's cursor by N. Cards
within a pack that come from the same run are always **consecutive on the sheet**. This
means certain card combinations within a run always appear together and others never do.

Multiple runs may share a physical sheet (e.g., runs A and C1 are on the same common
sheet), but each run's cursor advances independently. A card appears on exactly one run
(never duplicated across runs), which prevents the same card appearing twice in one pack.

For our simulation, the key operation is:

```
fn take_from_run(run: &[String], cursor: &mut usize, count: usize) -> Vec<String> {
    let mut cards = Vec::new();
    for _ in 0..count {
        cards.push(run[*cursor % run.len()].clone());
        *cursor += 1;
    }
    cards
}
```

Within a booster box (36 packs), cursors advance continuously. Between boxes, cursor
positions are independent (random starting positions).

## Commons

There are **101 distinct non-DFC commons** distributed across 4 runs on 2 physical sheets:

### Common Sheet 1

**A Run** (66 slots = 33 distinct cards x 2 copies each):

Cards in sheet order:
```
Gruesome Deformity, Claustrophobia, Frightful Delusion, Ashmouth Hound,
Doomed Traveler, Selfless Cathar, Dead Weight, Somberwald Spider, Riot Devils,
Altar's Reap, Forbidden Alchemy, Elder Cathar, Kessig Wolf, Wooden Stake,
Festerhide Boar, Hysterical Blindness, Voiceless Spirit, Ghoulcaller's Chant,
[...repeats with different neighbors...]
```

Each of the 33 cards appears exactly twice, spaced roughly 33 positions apart. Two
A-run cards that appear in the same pack are always adjacent (or near-adjacent) on the
sheet.

The full sheet-order sequence is in `data/sets/isd.json` under `runs.common_a`.

**C1 Run** (55 slots = 27 distinct cards x 2 copies each + Shimmering Grotto x 1):

Shimmering Grotto is the "short-printed" common. It appears only once in the C1 run
(instead of twice like all other C1 commons), and once in the C2 run. This makes it
roughly 20% less common than a typical common.

Full sequence in `runs.common_c1`.

### Common Sheet 2

**B Run** (66 slots = 22 distinct cards x 3 copies each):

Each of the 22 B-run commons appears 3 times on the sheet. Because B-run cards appear
3x (vs 2x for A-run), you might expect them to be more common. However, the pack
assembly rules draw fewer B-run cards per pack, which exactly compensates (see Pack
Assembly below).

Full sequence in `runs.common_b`.

**C2 Run** (55 slots = 18 distinct cards x 3 copies each + Shimmering Grotto x 1):

Full sequence in `runs.common_c2`.

### Common Pack Assembly

Packs alternate strictly between C1 and C2 types (50/50 split, every other pack in
a box).

**C1 packs** (9 commons total) come in two sub-variants:

| Variant     | A cards | B cards | C1 cards | Estimated frequency |
|-------------|---------|---------|----------|---------------------|
| 2A+2B+5C1  | 2       | 2       | 5        | ~60% of C1 packs    |
| 3A+1B+5C1  | 3       | 1       | 5        | ~40% of C1 packs    |

The ~60/40 ratio is estimated from sheet math constraints (lethe.xyz C1/C2 page:
"A foil rare rate of 1/32 would correspond to about 3/5 of C1 packs being 2B").
The true ratio for ISD specifically is not directly measured.

**C2 packs** (9 commons total) come in two sub-variants:

| Variant     | A cards | B cards | C2 cards | Frequency           |
|-------------|---------|---------|----------|---------------------|
| 3A+2B+4C2  | 3       | 2       | 4        | 50% of C2 packs     |
| 4A+3B+2C2  | 4       | 3       | 2        | 50% of C2 packs     |

The 50/50 ratio is stated directly on the C1/C2 page: "3 or 4 A cards, each with
equal probability."

For each variant, cards are drawn sequentially: first N cards from the A run, then
M cards from the B run, then K cards from the C1/C2 run. Each run's cursor advances
independently.

### Per-Card Common Frequencies

The pack assembly rules produce these approximate per-card-per-pack probabilities:

| Run              | Cards | Per-card frequency |
|------------------|-------|--------------------|
| A run (33 cards) | 2x    | ~9.09%             |
| B run (22 cards) | 3x    | ~9.09%             |
| C1 run (27 cards)| 2x    | ~9.09%             |
| C2 run (18 cards)| 3x    | ~8.18%             |
| Shimmering Grotto| 1x    | ~7.27%             |

The A, B, and C1 commons are essentially equally frequent. C2 commons are about 10%
less frequent. Shimmering Grotto is about 20% less frequent. These differences emerge
naturally from the sequential collation simulation.

## Uncommons

There are **60 distinct non-DFC uncommons** on one physical sheet:

**A Run** (66 slots = 33 distinct cards x 2 copies each)
**B Run** (54 slots = 27 distinct cards x 2 copies each)

Plus 1 filler slot (not a real card) to bring the sheet total to 121.

Full sequences in `runs.uncommon_a` and `runs.uncommon_b`.

### Uncommon Pack Assembly

Each pack gets 3 uncommons in one of two configurations:

| Variant | A cards | B cards | Frequency |
|---------|---------|---------|-----------|
| 2A+1B   | 2       | 1       | 13/20     |
| 1A+2B   | 1       | 2       | 7/20      |

This 13:7 ratio is stated on the ISD page: "Since the runs are unbalanced, 2 A cards
happens 13/20 of the time."

Cards are drawn sequentially: A cards first, then B cards.

### Per-Card Uncommon Frequencies

Despite the unbalanced runs and asymmetric pack variants, all uncommons have **exactly
equal frequency**: 5.00% per card per pack. The 13/20 ratio precisely compensates for
the run size difference (66 vs 54). This was verified mathematically:

- P(specific A uncommon) = (13/20 x 2 + 7/20 x 1) x (2/66) = 1/20 = 5.00%
- P(specific B uncommon) = (13/20 x 1 + 7/20 x 2) x (2/54) = 1/20 = 5.00%

## Rares and Mythic Rares

There are **53 distinct non-DFC rares** and **15 distinct non-DFC mythic rares** printed
on two separate 121-card sheets.

### Rare Sheet 1 (121 slots = A run 55 + C run 66)

**A Run** (55 slots):
- 15 mythic rares, each appearing **2 times** (30 slots)
- 6 rares, each appearing **4 times** (24 slots)
- Kessig Wolf Run appearing **1 time** (1 slot)

Mythics in A run: Liliana of the Veil, Mikaeus the Lunarch, Tree of Redemption,
Rooftop Storm, Geist of Saint Traft, Grimgrin Corpse-Born, Olivia Voldaren,
Skaab Ruinator, Balefire Dragon, Reaper from the Abyss, Mirror-Mad Phantasm,
Essence of the Wild, Elder of Laurels, Sever the Bloodline, Army of the Damned.

Rares in A run: Angelic Overseer, Falkenrath Marauders, Witchbane Orb,
Geist-Honored Monk, Grimoire of the Dead, Past in Flames.

**C Run** (66 slots):
- 16 rares, each appearing **4 times** (64 slots)
- Evil Twin appearing **2 times** (2 slots)

### Rare Sheet 2 (121 slots = B run 55 + D run 66)

**B Run** (55 slots):
- 13 rares, each appearing **4 times** (52 slots)
- Kessig Wolf Run appearing **3 times** (3 slots)

**D Run** (66 slots):
- 16 rares, each appearing **4 times** (64 slots)
- Evil Twin appearing **2 times** (2 slots)

Full sequences for all four runs in `runs.rare_a`, `runs.rare_b`, `runs.rare_c`,
`runs.rare_d`.

### Rare Pack Assembly

Each pack gets 1 card from one of the two rare sheets, alternating between sheets.
The cursor advances through the combined A+C sequence (sheet 1) or B+D sequence
(sheet 2) as a single 121-card loop.

### Per-Card Rare/Mythic Frequencies

All rares and mythics have **exactly equal frequency within their rarity**, despite
the split across sheets:

- P(specific mythic) = 0.5 x 2/121 = 1/121 = **0.83%**
- P(specific rare) = 0.5 x 4/121 = 2/121 = **1.65%**
- P(any mythic) = 15/121 = **12.4%** (close to the standard 1/8 = 12.5%)

Kessig Wolf Run (split 1+3 across sheets) and Evil Twin (split 2+2 across sheets)
both work out to exactly 2/121, the same as every other rare. The uneven splits are
compensated by the 50/50 sheet alternation.

## Double-Faced Cards (DFCs)

There are **20 DFCs** printed on one 121-card sheet:

### DFC Sheet (121 slots = A run 66 + B run 55)

**A Run** (66 slots):

| Card                    | Copies | Printed Rarity |
|-------------------------|--------|----------------|
| Garruk Relentless       | 1      | Mythic         |
| Instigator Gang         | 2      | Rare           |
| Bloodline Keeper        | 2      | Rare           |
| Mayor of Avabruck       | 2      | Rare           |
| Ludevic's Test Subject  | 2      | Rare           |
| Ulvenwald Mystics       | 6      | Uncommon       |
| Cloistered Youth        | 6      | Uncommon       |
| Screeching Bat          | 6      | Uncommon       |
| Hanweir Watchkeep       | 6      | Uncommon       |
| Village Ironsmith       | 11     | Common         |
| Grizzled Outcasts       | 11     | Common         |
| Thraben Sentry          | 11     | Common         |

**B Run** (55 slots):

| Card                    | Copies | Printed Rarity |
|-------------------------|--------|----------------|
| Daybreak Ranger         | 2      | Rare           |
| Kruin Outlaw            | 2      | Rare           |
| Reckless Waif           | 6      | Uncommon       |
| Gatstaf Shepherd        | 6      | Uncommon       |
| Civilized Scholar       | 6      | Uncommon       |
| Tormented Pariah        | 11     | Common         |
| Villagers of Estwald    | 11     | Common         |
| Delver of Secrets       | 11     | Common         |

DFC sheet copy counts **exactly match printed rarity**: mythic=1, rare=2, uncommon=6,
common=11. This was verified by parsing the actual sheet positions from the HTML.

Full sequences in `runs.dfc_a` and `runs.dfc_b`.

### DFC Pack Assembly

Each pack gets 1 DFC. The cursor advances through the combined A+B sequence as a
single 121-card loop.

### Per-Card DFC Frequencies

| DFC Rarity      | Cards | Per-card frequency |
|-----------------|-------|--------------------|
| Mythic (1 card) | 1     | 1/121 = 0.83%     |
| Rare (6 cards)  | 2     | 2/121 = 1.65%     |
| Uncommon (7)    | 6     | 6/121 = 4.96%     |
| Common (6)      | 11    | 11/121 = 9.09%    |

Every pack has two independent chances at a rare or mythic: the normal rare slot and
the DFC slot. This was a notable feature of Innistrad.

## Foils

### Foil Rate

**9/40 packs (22.5%)** contain a foil card. This is the standard pre-M20 foil rate,
confirmed by Mark Rosewater's "Project Booster Fun" article. It corresponds to the
advertised 1:67 card ratio (9 foils per 40 packs of 15 cards = 9/600 = 1:66.7).

Per booster box (36 packs): expect ~8.1 packs with a foil.

### Foil Rarity Distribution

When a foil appears, its rarity follows the MTGS forum model, validated by 32 boxes
of Ultimate Masters empirical data (549C/161U/52R/6M out of 768 non-basic foils,
from lethe.xyz/mtg/collation/uma.html):

| Foil Rarity  | Conditional P | Per-pack rate | ~1 in N packs |
|--------------|---------------|---------------|---------------|
| Common       | 11/16 = 68.75%| 15.47%        | 6.5           |
| Uncommon     | 3/16 = 18.75% | 4.22%         | 23.7          |
| Rare         | 7/128 = 5.47% | 1.23%         | 81.3          |
| Mythic Rare  | 1/128 = 0.78% | 0.18%         | 568.9         |
| Basic Land   | 1/16 = 6.25%  | 1.41%         | 71.1          |

### Non-DFC Foil Displacement

Non-DFC foils replace a common card. **Which** common is replaced depends on the foil's
rarity, and this affects which run's cursor is "wasted" (the cursor still advances;
the card is consumed from the run but replaced by the foil in the pack):

| Foil Rarity           | Displaces      | Pack type restriction              |
|-----------------------|----------------|-------------------------------------|
| Common or Basic Land  | C-run common   | None (see below)                    |
| Uncommon              | B common        | Any pack type                       |
| Rare or Mythic Rare   | A common        | Any pack type                       |

Source: ISD lethe.xyz page: "Foil commons (and basic lands) displace C1 commons.
(As far as I've seen, these are always in packs with basic lands.) Foil uncommons
displace B commons, and foil rares and mythic rares displace A commons."

The source's "displace C1 commons" is a statement about which slot the foil takes,
and this document used to read it as "common foils appear only in C1 packs". The
two readings cannot both hold alongside the rarity table above: packs alternate C1
and C2 evenly, so a marginal 11/16 common-foil rate would require a 22/16 rate
inside C1 packs. The simulator therefore lets a common or basic foil displace the
pack's last C-run common whichever type of pack it is in, which reproduces the
validated rarity table exactly. Restricting it and substituting an uncommon foil in
C2 packs — what the code did until issue #204 — inverted the table, producing 48%
uncommon and 29% common foils.

When a foil displaces a common, the pack has 8 commons + 1 foil instead of 9 commons.

### DFC Foils

A foil DFC replaces the normal DFC slot. The pack keeps its full 9 commons. Source:
ISD lethe.xyz page: "A foil double-faced card will displace the normal double-faced
card."

The fraction of foils that are DFC foils (vs non-DFC foils) is **unknown**. The
lethe.xyz author explicitly notes: "Calculating the exact rarities of double-faced
cards depends on unknown foil rates." For simulation, we estimate ~14% of foils are
DFC foils (based on 1 DFC sheet out of ~7 total print sheets), but this is a rough
estimate.

### Foil Card Selection

The specific foil card is chosen from all cards of the determined rarity. The real
foil sheet layout is unknown, so we select uniformly at random within the rarity.
For DFC foils, we select from the 20 DFCs weighted by their sheet copy counts (same
weights as the normal DFC slot).

## Simulation Algorithm

### State

The simulator maintains cursor positions for each independent run:

```
struct CollationState {
    common_a_cursor: usize,    // wraps at 66
    common_b_cursor: usize,    // wraps at 66
    common_c1_cursor: usize,   // wraps at 55
    common_c2_cursor: usize,   // wraps at 55
    uncommon_a_cursor: usize,  // wraps at 66
    uncommon_b_cursor: usize,  // wraps at 54
    rare_sheet1_cursor: usize, // wraps at 121 (A run 55 + C run 66)
    rare_sheet2_cursor: usize, // wraps at 121 (B run 55 + D run 66)
    dfc_cursor: usize,         // wraps at 121 (A run 66 + B run 55)
    pack_index: usize,         // tracks C1/C2 alternation and rare sheet alternation
}
```

### Initialization

For a new booster box (or draft), all cursors are initialized to random positions.
This simulates opening packs from a random point in the print run.

### Per-Pack Generation

```
fn generate_pack(state: &mut CollationState, rng: &mut Rng) -> Pack {
    let is_c1 = state.pack_index % 2 == 0;
    let use_rare_sheet1 = state.pack_index % 2 == 0;  // alternates with C1/C2

    // 1. Determine pack variant
    let (n_a, n_b, n_c) = if is_c1 {
        // ~60% chance of 2A+2B+5C1, ~40% chance of 3A+1B+5C1
        if rng.gen_ratio(3, 5) { (2, 2, 5) } else { (3, 1, 5) }
    } else {
        // 50/50 between 3A+2B+4C2 and 4A+3B+2C2
        if rng.gen_bool(0.5) { (3, 2, 4) } else { (4, 3, 2) }
    };

    // 2. Draw commons sequentially from each run
    let mut commons = vec![];
    commons.extend(take_from_run(&COMMON_A, &mut state.common_a_cursor, n_a));
    commons.extend(take_from_run(&COMMON_B, &mut state.common_b_cursor, n_b));
    if is_c1 {
        commons.extend(take_from_run(&COMMON_C1, &mut state.common_c1_cursor, n_c));
    } else {
        commons.extend(take_from_run(&COMMON_C2, &mut state.common_c2_cursor, n_c));
    }

    // 3. Draw uncommons
    let (n_ua, n_ub) = if rng.gen_ratio(13, 20) { (2, 1) } else { (1, 2) };
    let mut uncommons = vec![];
    uncommons.extend(take_from_run(&UNCOMMON_A, &mut state.uncommon_a_cursor, n_ua));
    uncommons.extend(take_from_run(&UNCOMMON_B, &mut state.uncommon_b_cursor, n_ub));

    // 4. Draw rare (from combined sheet sequence)
    let rare = if use_rare_sheet1 {
        take_from_run(&RARE_SHEET1, &mut state.rare_sheet1_cursor, 1)
    } else {
        take_from_run(&RARE_SHEET2, &mut state.rare_sheet2_cursor, 1)
    };

    // 5. Draw DFC
    let dfc = take_from_run(&DFC_SHEET, &mut state.dfc_cursor, 1);

    // 6. Handle foils (22.5% chance)
    let foil = if rng.gen_ratio(9, 40) {
        Some(generate_foil(rng, is_c1, &mut commons))
    } else {
        None
    };

    state.pack_index += 1;

    Pack { commons, uncommons, rare, dfc, foil }
}
```

Where `RARE_SHEET1` is the concatenation of `rare_a ++ rare_c` (121 cards), and
`DFC_SHEET` is the concatenation of `dfc_a ++ dfc_b` (121 cards).

### Foil Generation

```
fn generate_foil(rng: &mut Rng, is_c1: bool, commons: &mut Vec<String>) -> Foil {
    // Determine if DFC foil (~14%) or non-DFC foil (~86%)
    let is_dfc_foil = rng.gen_ratio(1, 7);  // rough estimate

    if is_dfc_foil {
        // DFC foil replaces the normal DFC slot
        // Select a random DFC weighted by sheet copies
        return Foil::Dfc(random_dfc_by_weight(rng));
    }

    // Non-DFC foil: determine rarity
    let roll = rng.gen_range(0..128);
    let rarity = if roll < 88 {       // 11/16 = 88/128
        FoilRarity::Common
    } else if roll < 112 {            // 3/16 = 24/128
        FoilRarity::Uncommon
    } else if roll < 119 {            // 7/128
        FoilRarity::Rare
    } else if roll < 120 {            // 1/128
        FoilRarity::Mythic
    } else {                          // 8/128 = 1/16
        FoilRarity::BasicLand
    };

    // Displace the appropriate common and select a random card of that rarity
    match rarity {
        Common | BasicLand => {
            // Displace the pack's last C-run common, in either pack type
            commons.pop();
            Foil::Card(random_card_of_rarity(rng, rarity))
        }
        Uncommon => {
            // Displace a B common
            remove_b_common(commons);
            Foil::Card(random_card_of_rarity(rng, rarity))
        }
        Rare | Mythic => {
            // Displace an A common
            remove_a_common(commons);
            Foil::Card(random_card_of_rarity(rng, rarity))
        }
    }
}
```

### Draft Pack Generation

For a draft pod (e.g., 8 players x 3 packs = 24 packs):

1. Initialize a `CollationState` with random cursor positions
2. Generate 24 packs sequentially (simulating packs from the same booster box)
3. Shuffle the 24 packs
4. Distribute: packs 0-7 as each player's pack 1, packs 8-15 as pack 2, packs 16-23 as pack 3

This preserves the sequential structure: consecutive packs from the same box share
collation patterns, which is realistic for a draft where all packs come from the
same (or nearby) boxes.

The shuffle in step 3 is what keeps a seat from being locked to one half of the
collation. A pack's index in the box decides both its C1/C2 type and which rare
sheet it draws from, so dealing the box in generation order gave every seat in an
even-sized pod the same index parity for all three of its packs. All 30 ISD mythic
slots are on rare sheet 1, so half the pod first-picked every mythic in the draft
and the other half never opened one, and the same lock split the C1 and C2 commons
between them (issue #202). Which packs of a box a seat ends up with is arbitrary in
a real draft, so the deal is where that correlation is broken — the collation
sequence itself is unchanged.

For 8 players x 3 packs = 24 packs from a 36-pack box, we use 2/3 of a box. For
4 players x 3 packs = 12 packs, we use 1/3 of a box.

## Remaining Approximations

These are aspects of the collation that are not fully determined from available data:

1. **C1 sub-variant ratio (~60/40)**: The lethe.xyz C1/C2 page estimates ~3/5 of C1
   packs are the 2B variant, derived from sheet math constraints involving foil rates.
   The exact ratio for ISD (which has modified pack structure due to DFCs) is not
   directly measured. True value is likely close to 3/5 but could differ slightly.

2. **Rare sheet alternation**: We assume sheets 1 and 2 alternate strictly (even-indexed
   packs use sheet 1, odd-indexed use sheet 2). This is the natural pattern for
   sequential collation but is not explicitly confirmed for ISD.

3. **DFC foil fraction (~14%)**: The fraction of foils that are DFC foils is unknown.
   The lethe.xyz author explicitly notes this uncertainty. Our estimate of ~14% (1
   DFC sheet out of ~7 total sheets) is a rough heuristic.

4. **Foil card selection**: We choose uniformly at random within the determined rarity.
   In reality, foils are printed on their own sheet(s) with a specific layout that
   determines which foil cards can appear together (analogous to the main collation).
   This sheet layout is unknown.

5. **Uncommon filler slot**: The uncommon sheet has 121 slots but only 120 are real
   cards (66 A-run + 54 B-run). The 121st slot is a filler. When the cursor lands on
   it during sequential collation, the behavior is unknown (likely skipped or produces
   a duplicate). This affects 1/121 = 0.8% of uncommon draws. We skip it.

## Data File

All sheet-order sequences are stored in `data/sets/isd.json`. The file contains:

- `runs.*`: All 12 print run sequences in exact sheet order
- `collation.*`: Pack assembly rules, variant weights, foil model
- `approximations`: List of known approximations

The JSON structure is designed to be directly loadable by the pack generator code
without any additional data transformation.
