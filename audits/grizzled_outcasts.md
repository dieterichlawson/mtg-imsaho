## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/185/grizzled-outcasts-krallenhorde-wantons?utm_source=api
**Type line**: `Creature — Human Werewolf` — {4}{G}, 4/4
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Krallenhorde Wantons — `Creature — Werewolf`, 7/7
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/185/grizzled-outcasts-krallenhorde-wantons?utm_source=api
**Type line**: `Creature — Human Werewolf` — {4}{G}, 4/4
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Krallenhorde Wantons — `Creature — Werewolf`, 7/7
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/185/grizzled-outcasts-krallenhorde-wantons?utm_source=api
**Type line**: `Creature — Human Werewolf` — {4}{G}, 4/4
**Oracle text**:
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Krallenhorde Wantons — `Creature — Werewolf`, 7/7
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

One found, of the class opened under Gatstaf Shepherd.

1. **The back face had no colour indicator, so Krallenhorde Wantons was colourless.** `grizzled_outcasts.rs:37`
   - Type line (back face) is `Creature — Werewolf` with no mana cost, so CR 204.2 puts the colour in an indicator beside it.
   - Code declared `power`, `toughness`, `subtypes` and `oracle_text` for the back face and no colour, and `colors_of` therefore returned an empty vector for a transformed Outcasts.
   - Now `color_indicator: vec![Color::Green]`.

Nothing on this card reads its own colour — it is a vanilla body on both faces — so unlike Gatstaf Howler there is no ability here that visibly misfires. It still matters for anything that asks: protection from green, a colour-matters block restriction, another card's intimidate. Two of the set's twenty back faces now have their indicator; `audits/BACK_FACE_COLORS.md` tracks the rest.

**Where the green came from.** A web search this session, over the Scryfall and mtg.wtf results for the card, returned that "Krallenhorde Wantons has a color indicator of green and is a Creature — Werewolf. It has stats of 7/7." External source obtained during this audit, not memory.

### Checked and correct

- Front: `{4}{G}`, `Creature — Human Werewolf`, 4/4, no keywords. Scryfall's aggregate keyword list is `Transform` alone, so there is nothing else to declare on either face.
- Back: `Krallenhorde Wantons`, `Creature — Werewolf`, 7/7, no keywords, no continuous effects, no activated abilities — the plainest double-faced card in the set.
- Oracle text on both faces matches the cache (the back face's stale "transform Krallenhorde Wantons" was fixed in the sweep under Village Ironsmith and is now held by `oracle_text_says_what_scryfall_says`).
- Both faces declare the upkeep trigger at the default `TriggerScope::Each` — "At the beginning of **each** upkeep".
- The transform runs on the shared werewolf mechanism: `werewolf_should_trigger` (suppressing the trigger for a token copy), `werewolf_should_transform` (reading the face the ability triggered from, CR 603.4 + CR 712.8), and `werewolf_on_upkeep`.
- The front face is Human and the back is not, which is what makes it a legal Moonmist target on one side only.

### Tricky interactions checked

- Transforms with other werewolves on the same upkeep, and back again: PASS.
- Intervening-if on both faces' conditions: PASS.
- Loses the Human subtype on transforming: PASS.
- Colour is kept across the transform: PASS (after fix).
- Token copy cannot transform: PASS (set-wide werewolf coverage).

### Test coverage

- transforms alongside other werewolves, and back: `werewolf_cards.rs:726` and `:741`
- intervening-if on the upkeep trigger: `intervening_if.rs:129`
- trigger snapshot: `trigger_snapshots.rs:130`
- a green 4/4 that becomes a green 7/7, and stops being Human: `werewolf_cards.rs` `grizzled_outcasts_is_a_green_4_4_that_becomes_a_green_7_7` (NEW, mutation-checked by removing the indicator)

Before this the card had a section header in `werewolf_cards.rs` with nothing under it.

