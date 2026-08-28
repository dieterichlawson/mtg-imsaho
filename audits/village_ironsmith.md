## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/168/village-ironsmith-ironfang?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{R}, 1/1
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Ironfang — `Creature — Werewolf`, 3/1
```
First strike
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/168/village-ironsmith-ironfang?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{R}, 1/1
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Ironfang — `Creature — Werewolf`, 3/1
```
First strike
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
- First strike is on **both** faces (1/1 front, 3/1 back), unlike its siblings
  where the keyword is back-face only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The flip and both faces' first strike: `werewolf_cards.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/168/village-ironsmith-ironfang?utm_source=api
**Type line**: `Creature — Human Werewolf` — {1}{R}, 1/1
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Ironfang — `Creature — Werewolf`, 3/1
```
First strike
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

One found, and it was seven cards wide.

1. **The back face carried pre-errata wording.** `village_ironsmith.rs:44`
   - Oracle text (back face) says: `First strike\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
   - Code had: `"First strike\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform Ironfang."`
   - The front face had already been updated to "this creature"; the back had not.

**Set-wide follow-up.** This is the second time I have hit it (Instigator Gang was the first), so rather than fix a third later I checked every card against the cache. Seven back faces were stale in the same way — Tormented Pariah, Village Ironsmith, Grizzled Outcasts, Villagers of Estwald, Reckless Waif, Gatstaf Shepherd and Ulvenwald Mystics, the last of which also had Ulvenwald Primordials regenerating itself by name rather than "Regenerate this creature". All seven fixed.

Nothing behavioural reads these strings, which is precisely why they sat there — and why they matter. The oracle text is what a reader, a log line, and an audit compare against; auditing a card against the wrong words is how a real difference gets waved through. `card_data_invariants::oracle_text_says_what_scryfall_says` now cross-checks every face, front and back, against `data/oracle_cache.json`, and fails the build on drift. It cross-checks 253 faces today. Parenthesised reminder text is optional on either side (the set is inconsistent about carrying it, and that is not a rules difference); the five basic lands are exempt, because Scryfall gives them reminder text as their whole oracle text — the mana ability is intrinsic (CR 305.6), not printed — and the cards state it as the ability it is.

### Checked and correct

- Front: `{1}{R}`, `Creature — Human Werewolf`, 1/1, `keywords: [FirstStrike]`.
- Back: `Ironfang`, `Creature — Werewolf`, 3/1, `keywords: [FirstStrike]`. First strike is on **both** faces here, unlike Kruin Outlaw where it is replaced by double strike — and Scryfall's aggregate keyword list for the card (`Transform, First strike`) is consistent with that.
- The 3/1 back face is the whole change: same keyword, +2 power, −0 toughness... actually 1/1 to 3/1, so +2/+0.
- Both faces declare their upkeep trigger at the default `TriggerScope::Each` — "At the beginning of **each** upkeep".
- The transform runs on the shared werewolf mechanism: `werewolf_should_trigger` (suppressing the trigger entirely for a token copy), `werewolf_should_transform` (reading the face the ability triggered from, CR 603.4 + CR 712.8), and `werewolf_on_upkeep`.
- No activated abilities, no continuous effects, nothing else on either face — matching the text.

### Tricky interactions checked

- First strike survives the transform, because both faces declare it and `has_keyword` reads the active face: PASS.
- Moonmist transforms it as a Human: PASS (`moonmist.rs:177` uses this card precisely because its back face is not a Human).
- Intervening-if on the upkeep trigger (no spells last turn / two or more): PASS.
- Token copy cannot transform and puts no trigger on the stack: PASS (set-wide werewolf coverage).

### Test coverage

- Moonmist transforms it, and the back face is no longer a Human: `moonmist.rs:177`
- intervening-if on the upkeep trigger: `intervening_if.rs:132`
- trigger snapshot: `trigger_snapshots.rs:129`
- oracle text matches the fetched cache, both faces: `card_data_invariants.rs` `oracle_text_says_what_scryfall_says` (NEW, mutation-checked, covers all 253 faces)
- the werewolf transform mechanism itself: `werewolf_cards.rs` and `werewolf_subtype_after_transform.rs`, set-wide.

