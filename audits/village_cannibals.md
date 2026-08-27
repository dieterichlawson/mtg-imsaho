## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/125/village-cannibals?utm_source=api
**Type line**: `Creature — Human` — {2}{B}, 2/2
**Oracle text**:
```
Whenever another Human creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another Human** creature dies" — self-exclusion matters here
  because Village Cannibals is itself `Creature — Human`, and it comes from the
  trigger kind rather than a hand-written id check. The Human test goes through
  `state.has_subtype`, so a token Human or a granted type counts.
- No controller filter, correctly: the wording says any Human, not yours.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/125/village-cannibals?utm_source=api
**Type line**: `Creature — Human` — {2}{B}, 2/2
**Oracle text**:
```
Whenever another Human creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever another **Human** creature dies" — the Human filter *and* the
  self-exclusion, and note there is **no** "you control": an opponent's Human
  dying feeds it too: PASS
- `has_subtype` reads the active face, so a transformed Werewolf that is no
  longer Human does not count: PASS
- It counts Human tokens: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Human death filter: `cards_morbid_and_ltb.rs`, `subtype.rs`
