## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/209/villagers-of-estwald-howlpack-of-estwald?utm_source=api
**Type line**: `Creature — Human Werewolf` — {2}{G}, 2/3
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Howlpack of Estwald — `Creature — Werewolf`, 4/6
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/209/villagers-of-estwald-howlpack-of-estwald?utm_source=api
**Type line**: `Creature — Human Werewolf` — {2}{G}, 2/3
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Howlpack of Estwald — `Creature — Werewolf`, 4/6
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/209/villagers-of-estwald-howlpack-of-estwald?utm_source=api
**Type line**: `Creature — Human Werewolf` — {2}{G}, 2/3
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Howlpack of Estwald — `Creature — Werewolf`, 4/6
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

One found, of the class opened under Gatstaf Shepherd.

1. **The back face had no colour indicator, so Howlpack of Estwald was colourless.** `villagers_of_estwald.rs:37`
   - Type line (back face) is `Creature — Werewolf` with no mana cost, so CR 204.2 puts the colour in an indicator beside it.
   - Code declared `power`, `toughness`, `subtypes` and `oracle_text` for the back face and no colour, so `colors_of` returned an empty vector for a transformed Villagers.
   - Now `color_indicator: vec![Color::Green]`.

**Where the green came from.** A web search this session, over the Scryfall results for the card, returned "Howlpack of Estwald is a Green Creature — Werewolf". External source obtained during this audit, not memory. Three of the set's twenty back faces now have their indicator; `audits/BACK_FACE_COLORS.md` tracks the rest.

**Also found: the card had no test of its own.** `werewolf_cards.rs` carried a `// ── Villagers of Estwald ──` header with nothing under it. The card is used by three other test files — `equipment_human_conditional.rs`, `characteristics_card_sweep.rs`, `werewolf_subtype_after_transform.rs` — but always as a *fixture*, because its back face is a Werewolf that stopped being a Human. Nothing asserted the card's own body. Now it does.

**And one gap in earlier work.** Two other sections in the same file were empty headers: Village Ironsmith and Tormented Pariah. Village Ironsmith is a card I audited two turns ago and signed off without noticing, so its test is added here rather than left — first strike is on **both** of its faces, which is worth pinning precisely because its neighbour Kruin Outlaw trades first strike for double strike on the flip. Tormented Pariah is next on the audit list and will get its own.

### Checked and correct

- Front: `{2}{G}`, `Creature — Human Werewolf`, 2/3, no keywords. Scryfall's aggregate keyword list is `Transform` alone, so there is nothing else to declare on either face.
- Back: `Howlpack of Estwald`, `Creature — Werewolf`, 4/6, no keywords, no continuous effects, no activated abilities.
- Oracle text on both faces matches the cache (the back face's stale "transform Howlpack of Estwald" was fixed in the sweep under Village Ironsmith and is now held by `oracle_text_says_what_scryfall_says`).
- Both faces declare the upkeep trigger at the default `TriggerScope::Each`.
- The transform runs on the shared werewolf mechanism: `werewolf_should_trigger`, `werewolf_should_transform`, `werewolf_on_upkeep`.
- The front face is Human and the back is not — which is exactly what the three other test files use it for.

### Tricky interactions checked

- Loses the Human subtype on transforming, so Human-conditional equipment stops applying: PASS (`equipment_human_conditional.rs:160`).
- Characteristics read through the active face after transforming: PASS (`characteristics_card_sweep.rs:127`).
- Subtypes after transform: PASS (`werewolf_subtype_after_transform.rs:32`).
- Transforms with other werewolves on the same upkeep, and back: PASS.
- Intervening-if on both faces' conditions: PASS.
- Colour kept across the transform: PASS (after fix).
- Token copy cannot transform: PASS (set-wide werewolf coverage).

### Test coverage

- Human-conditional equipment stops on the back face: `equipment_human_conditional.rs:160`
- characteristics sweep: `characteristics_card_sweep.rs:127`
- subtypes after transform: `werewolf_subtype_after_transform.rs:32`
- intervening-if on the upkeep trigger: `intervening_if.rs`
- trigger snapshot: `trigger_snapshots.rs`
- a green 2/3 that becomes a green 4/6 and stops being Human: `werewolf_cards.rs` `villagers_of_estwald_is_a_green_2_3_that_becomes_a_green_4_6` (NEW, mutation-checked by removing the indicator)
- Village Ironsmith keeps first strike across its transform: `werewolf_cards.rs` `village_ironsmith_keeps_first_strike_across_the_transform` (NEW, filling a gap left by that card's own audit)

