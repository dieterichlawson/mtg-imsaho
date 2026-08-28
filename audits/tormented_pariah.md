## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/165/tormented-pariah-rampaging-werewolf?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {3}{R}, 3/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Rampaging Werewolf — `Creature — Werewolf`, 6/4
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/165/tormented-pariah-rampaging-werewolf?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {3}{R}, 3/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Rampaging Werewolf — `Creature — Werewolf`, 6/4
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/165/tormented-pariah-rampaging-werewolf?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {3}{R}, 3/2
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Rampaging Werewolf — `Creature — Werewolf`, 6/4
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

Two found, both fixed.

1. **The back face had no colour indicator, so Rampaging Werewolf was colourless.** `tormented_pariah.rs:37`
   - Type line (back face) is `Creature — Werewolf` with no mana cost, so CR 204.2 puts the colour in an indicator beside it.
   - Code declared `power`, `toughness`, `subtypes` and `oracle_text` for the back face and no colour, so `colors_of` returned an empty vector for a transformed Pariah.
   - Now `color_indicator: vec![Color::Red]`. A web search this session, over the Scryfall and mtg.wtf results for the card, returned "Rampaging Werewolf has a color indicator showing it is red, and it is a 6/4 creature" — external source obtained during this audit, not memory.

2. **The doc comment dropped a subtype.** `tormented_pariah.rs:8`
   - Type line says: `Creature — Human Warrior Werewolf`
   - Comment said: `/// Tormented Pariah {3}{R} 3/2 Human Warrior // Rampaging Werewolf 6/4 Werewolf`
   - The `subtypes` vector had all three, so nothing behaved wrongly — but the comment is the first thing read when auditing the card, and it disagreed with the card. Fixed.

**Also: the card had no test of its own**, the last of three `// ── ... ──` headers in `werewolf_cards.rs` with nothing beneath them. Now tested, and guarded — see below.

### Set-wide follow-up

`test_suite_guards::a_card_section_header_has_tests_under_it` fails the build on a section header with no tests under it. Eighteen test files divide themselves this way, 134 sections in all, and the header is how a reader — or an auditor grepping for a card — concludes the card is covered.

The case worth recording is Villagers of Estwald, which I hit one card ago: it appears in three other test files, so it greps as thoroughly tested. But every one of those uses it as a *fixture*, because its back face is a Werewolf that stopped being a Human. Nothing asserted the card's own body. A grep for a card's name is not evidence that the card is tested, and the guard now makes the weaker claim — "there is a section for it" — mean something.

### Checked and correct

- Front: `{3}{R}`, `Creature — Human Warrior Werewolf`, 3/2, no keywords. All **three** subtypes present in the data.
- Back: `Rampaging Werewolf`, `Creature — Werewolf`, 6/4, no keywords, no continuous effects, no activated abilities.
- Oracle text on both faces matches the cache (the back face's stale "transform Rampaging Werewolf" was fixed in the sweep under Village Ironsmith and is now held by `oracle_text_says_what_scryfall_says`).
- Both faces declare the upkeep trigger at the default `TriggerScope::Each`.
- The transform runs on the shared werewolf mechanism: `werewolf_should_trigger`, `werewolf_should_transform`, `werewolf_on_upkeep`.
- The stack shows the *active* face's name — a transformed Pariah's trigger reads "Rampaging Werewolf", not "Tormented Pariah" (`harness_display.rs:59`).

### Tricky interactions checked

- Loses Human **and** Warrior on transforming, keeping only Werewolf: PASS.
- The stack entry names the active face: PASS.
- Transforms with other werewolves on the same upkeep, and back: PASS.
- Intervening-if on both faces' conditions: PASS.
- Colour kept across the transform: PASS (after fix).
- Token copy cannot transform: PASS (set-wide werewolf coverage).

### Test coverage

- the stack names the back face, not the front: `harness_display.rs:59`
- subtypes after transform: `subtype.rs:498`
- intervening-if on the upkeep trigger: `intervening_if.rs`
- trigger snapshot: `trigger_snapshots.rs`
- a red 3/2 Human Warrior Werewolf that becomes a red 6/4 Werewolf and nothing else: `werewolf_cards.rs` `tormented_pariah_is_a_red_3_2_human_warrior_that_becomes_a_red_6_4` (NEW, mutation-checked by removing the indicator)
- no section header claims coverage it does not have: `test_suite_guards.rs` `a_card_section_header_has_tests_under_it` (NEW, mutation-checked)

