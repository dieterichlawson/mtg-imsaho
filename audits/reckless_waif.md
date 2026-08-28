## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/159/reckless-waif-merciless-predator?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {R}, 1/1
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Merciless Predator — `Creature — Werewolf`, 3/2
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.

- The transform pair is the set's shared werewolf mechanism, and it turns on a
  distinction the wording makes precisely:
  - front → back, "if **no spells were cast** last turn":
    `num_spells_cast_last_turn.values().sum::<u32>() == 0` — nobody cast anything.
  - back → front, "if **a player** cast two or more spells last turn":
    `.values().any(|&count| count >= 2)` — one player individually reached two,
    not the table's total.
  Summing for the second, or requiring a single caster for the first, would both
  be wrong; neither is done.
- Both are intervening-if clauses (CR 603.4), and the card overrides
  `should_trigger` through `helpers::werewolf_should_trigger`, so with the
  condition unmet nothing reaches the stack.
- That helper also refuses the trigger for a token copy, which cannot transform
  (CR 111.7) — so it does not sit on the stack with nothing to do.
- Back-face P/T comes from `dynamic_pt`, which supplies the *base* in
  `effective_power`, leaving counters and anthems to layer on top.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`werewolf_cards.rs` (29 tests over the family), `intervening_if.rs` (the spell-count condition), `transform_dfc.rs` (CR 712 face characteristics).
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/159/reckless-waif-merciless-predator?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {R}, 1/1
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Merciless Predator — `Creature — Werewolf`, 3/2
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.

- The flip conditions are the shared `werewolf_should_trigger` /
  `werewolf_should_transform` helpers — one implementation of "if no spells were
  cast last turn" and "if a player cast two or more spells last turn" for the
  whole family: PASS
- CR 603.4: both are intervening-ifs, checked when the trigger would go on the
  stack *and* again on resolution: PASS
- "At the beginning of **each** upkeep" — `TriggerScope::Each`, so it fires on
  the opponent's turn too: PASS
- Each face's P/T, keywords and subtypes were checked against the fetched oracle
  face by face, not against Scryfall's merged `Keywords` list — which is the trap
  here, since that list names keywords the front face does not have: PASS
- Both faces are vanilla apart from the flip trigger, so the whole card is its
  card data plus the shared werewolf helpers — verified face by face against the
  fetched oracle: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The flip conditions: `werewolf_cards.rs`, `intervening_if.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/159/reckless-waif-merciless-predator?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {R}, 1/1
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Merciless Predator — `Creature — Werewolf`, 3/2
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

One found on the card, one in its tests.

1. **The back face had no colour indicator, so Merciless Predator was colourless.** `reckless_waif.rs:37`
   - Type line (back face) is `Creature — Werewolf` with no mana cost, so CR 204.2 puts the colour in an indicator beside it.
   - Code declared `power`, `toughness`, `subtypes` and `oracle_text` for the back face and no colour, so `colors_of` returned an empty vector for a transformed Waif.
   - Now `color_indicator: vec![Color::Red]`. A web search this session, over the Scryfall and mtg.wtf results for the card, returned "Merciless Predator is a red Werewolf creature with a color indicator" — external source obtained during this audit, not memory. Five of the set's twenty back faces now have theirs.

2. **`reckless_waif_transforms_back_when_two_spells_cast` invented the flipped state.** `werewolf_cards.rs` (before the fix)
   - It wrote `is_transformed = true` and `name = "Merciless Predator"` by hand instead of calling `apply_transform`. That is not the state a real flip leaves behind — `apply_transform` sets both fields *and* declines to flip a token copy of a double-faced card (CR 111.7), so a hand-written flip walks past the one decision the function exists to make.

### Set-wide follow-up

Twelve tests across four files did the same thing. All twelve now go through `apply_transform`. The one remaining hand-written flip is in `dfc_zone_cleanup.rs` and is deliberate — the zone-change reset has to hold for a flipped state however it was reached — but the comment justifying it said Garruk Relentless transforms that way himself, which stopped being true when he moved onto `apply_transform`. Corrected, since a comment that misdescribes the code is what an audit reads first.

### Checked and correct

- Front: `{R}`, `Creature — Human Rogue Werewolf`, 1/1, no keywords. All **three** subtypes present.
- Back: `Merciless Predator`, `Creature — Werewolf`, 3/2, no keywords, no continuous effects, no activated abilities.
- Oracle text on both faces matches the cache (the back face's stale "transform Merciless Predator" was fixed in the sweep under Village Ironsmith and is now held by `oracle_text_says_what_scryfall_says`).
- Both faces declare the upkeep trigger at the default `TriggerScope::Each`.
- The transform runs on the shared werewolf mechanism.

### Tricky interactions checked

- **Transforms on the game's first upkeep.** "if no spells were cast last turn" has no first-turn exception: with no previous turn, no spells were cast in it. This is the card's sharpest corner and it is already tested — the test's comment records that all twelve werewolves once carried a private copy of the condition and every copy had invented `&& !state.is_first_turn`.
- Does not transform when a spell was cast last turn: PASS.
- Transforms back when a player cast two or more: PASS.
- Loses Human **and** Rogue on transforming, keeping only Werewolf: PASS.
- Colour kept across the transform: PASS (after fix).
- Token copy cannot transform: PASS (set-wide werewolf coverage).

### Test coverage

- transforms on the first upkeep, no invented first-turn exception: `werewolf_cards.rs` `reckless_waif_transforms_on_the_games_first_upkeep`
- stays human when a spell was cast: `werewolf_cards.rs` `reckless_waif_stays_human_when_spells_cast`
- transforms back on two spells: `werewolf_cards.rs` `reckless_waif_transforms_back_when_two_spells_cast` (now flips through `apply_transform`)
- used as the buff target in the Mayor of Avabruck tests: `werewolf_cards.rs`
- intervening-if on the upkeep trigger: `intervening_if.rs`
- trigger snapshot: `trigger_snapshots.rs`
- a red 1/1 Human Rogue Werewolf that becomes a red 3/2 Werewolf and nothing else: `werewolf_cards.rs` `reckless_waif_is_a_red_1_1_human_rogue_that_becomes_a_red_3_2` (NEW, mutation-checked by removing the indicator)

