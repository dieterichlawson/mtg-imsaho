## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/179/festerhide-boar?utm_source=api
**Type line**: `Creature — Boar` — {3}{G}, 3/3
**Oracle text**:
```
Trample
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

- Morbid on this card is a **replacement effect**, not a triggered ability:
  "enters with two +1/+1 counters on it if a creature died this turn" modifies
  how the permanent enters (CR 614.1c). Correctly not a trigger, so no
  `should_trigger` is needed here — unlike the morbid cards whose text is "at
  the beginning of ...".
- Shares `helpers::enters_with_counters` with Somberwald Spider, so the two
  cannot drift apart.

### Test coverage
`intervening_if.rs::morbid_enters_with_counters_only_when_a_creature_died`, both arms
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/179/festerhide-boar?utm_source=api
**Type line**: `Creature — Boar` — {3}{G}, 3/3
**Oracle text**:
```
Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.

Morbid "enters with two +1/+1 counters" is a replacement effect (CR 614.1c),
implemented through the shared `helpers::enters_with_counters` on
`ReplaceableEvent::EntersBattlefield` — not an ETB trigger that adds counters
afterwards. The morbid condition reads `state.creature_died_this_turn`.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_morbid_and_ltb.rs` and `enters_tapped_replacement.rs` — counters present on entry, none without a death.

## Audit — 2026-08-28 19:08

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Festerhide Boar"`, https://scryfall.com/card/isd/179/festerhide-boar
**Oracle text**:
```
Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```
**Type line**: Creature — Boar
**Mana cost**: {3}{G}   **P/T**: 3/3   **Keywords (Scryfall)**: Trample, Morbid
**Rulings**: none on Scryfall for this card.
**Status**: PASS (one misleading test comment corrected)

### Code issues
No issues found in `mtg-engine/src/cards/isd/festerhide_boar.rs`.

`{3}{G}`, `Creature`, `subtypes: ["Boar"]`, 3/3, `keywords: vec![Trample]` — Morbid is an
ability word, not a keyword ability, so the engine correctly does not model it as one — and
oracle text verbatim.

"Enters with" is a replacement effect (CR 614.1c) and is implemented as one, through the shared
`helpers::enters_with_counters`: the counters are part of how the Boar enters, so it is 5/5 from
the first moment anything can look at it, and there is no window in which a 3/3 exists. The
morbid condition reads `state.creature_died_this_turn` inside the closure, i.e. at the moment
the replacement applies — which is when "if a creature died this turn" is asked.

The flag is turn-based state, reset at turn start (`engine.rs:700`) and set from all three death
paths (established at the Brimstone Volley audit).

### Tricky interactions checked
- **Both morbid states**: PASS — with a death this turn, two counters; without, none.
- **The counters are there at entry** (replacement, not trigger): PASS.
- **Reanimated, not just cast**: PASS — "enters with" applies however it arrives, and there is
  a test that brings it back from the graveyard.
- **The Boar entering as an Essence of the Wild copy**: after this pass's copy-ordering fix, the
  Boar is an Essence when it enters and its own "enters with" is not applied — the ruling shape
  tested at Essence with Grimgrin.
- **A death by sacrifice or -X/-X counts**: the flag is set by every death path.
- **Trample**: keyword, pinned by the set-wide Scryfall sweep.

### Test coverage
- both morbid states, shared table with Somberwald Spider:
  `intervening_if.rs:282 morbid_enters_with_counters_only_when_a_creature_died`
- 5/5 at entry: `cards_shortcuts_taken.rs:361 festerhide_boar_morbid_counters_visible_to_etb_checks`
- reanimated: `cards_shortcuts_taken.rs:413 festerhide_boar_gets_morbid_counters_when_reanimated`

Mutation-checked: making the counters unconditional fails the table's died=false rows; one
counter instead of two fails both shortcuts tests.

**One misleading comment corrected**: the entry-visibility test's doc claimed "Mentor of the
Meek should NOT trigger" — but the test never stages a Mentor, and a 3/3 base is over Mentor's
"power 2 or less" either way, so no ETB watcher in this pool can tell 3/3-then-counters from
enters-as-5/5. The effective power at entry is the honest observable, and the comment now says
so instead of citing an interaction that cannot discriminate.

### Changes made
- `cards_shortcuts_taken.rs`: the doc comment. No behavioural change, no code change.
