## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/202/somberwald-spider?utm_source=api
**Type line**: `Creature — Spider` — {4}{G}, 2/4
**Oracle text**:
```
Reach (This creature can block creatures with flying.)
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- Identical morbid replacement to Festerhide Boar, through the same helper.
  Reach is a keyword rather than a continuous effect, which is correct.

### Test coverage
`intervening_if.rs::morbid_enters_with_counters_only_when_a_creature_died`, both arms
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/202/somberwald-spider?utm_source=api
**Type line**: `Creature — Spider` — {4}{G}, 2/4
**Oracle text**:
```
Reach (This creature can block creatures with flying.)
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.

Same morbid replacement effect as Festerhide Boar, through the shared helper.
Reach on the printed keywords, 2/4 base.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_morbid_and_ltb.rs` — the shared morbid tests cover both.

## Audit — 2026-08-28 19:09

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Somberwald Spider"`, https://scryfall.com/card/isd/202/somberwald-spider
**Oracle text**:
```
Reach (This creature can block creatures with flying.)
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```
**Type line**: Creature — Spider
**Mana cost**: {4}{G}   **P/T**: 2/4   **Keywords (Scryfall)**: Reach, Morbid
**Rulings**: none on Scryfall for this card.
**Status**: PASS

### Code issues
No issues found in `mtg-engine/src/cards/isd/somberwald_spider.rs`.

`{4}{G}`, `Creature`, `subtypes: ["Spider"]`, 2/4, `keywords: vec![Reach]` — Morbid is an
ability word, not a keyword ability — oracle text verbatim including the reminder text.

The morbid line is byte-for-byte the same shape as Festerhide Boar's, audited immediately
before this: a CR 614.1c replacement through the shared `helpers::enters_with_counters`, with
the condition read from `creature_died_this_turn` at the moment the replacement applies. The
mechanism-level facts established there — the flag's reset and three set-sites, entry
visibility, reanimation, the Essence-copy interaction — carry over because they are the shared
helper's and the flag's, not either card's.

### Tricky interactions checked
- **Both morbid states**: PASS — the shared table has a row per card per state.
- **Reach blocks fliers**: keyword, engine-side; pinned by the set-wide Scryfall keyword sweep.
- **Everything else**: as Festerhide Boar — same code shape, same shared helper.

### Test coverage
- both morbid states, by name:
  `intervening_if.rs:282 morbid_enters_with_counters_only_when_a_creature_died` (table rows)
- entry visibility and reanimation: tested at the Boar through the shared helper
- Reach: `card_data_invariants.rs keywords_say_what_scryfall_says`

Mutation-checked **on this card's own file** — the closure is a separate copy of the condition,
so the Boar's mutations prove nothing about it: making the Spider's counters unconditional
fails its died=false row by name; three counters instead of two fails its died=true row by name.

### Changes made
None — code and coverage were both right.
