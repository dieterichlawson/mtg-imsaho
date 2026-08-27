## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/191/lumberknot?utm_source=api
**Type line**: `Creature — Treefolk` — {2}{G}{G}, 1/1
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **a** creature dies" — no controller, token or subtype filter, and
  none is applied.
- Worth recording: the wording is "a creature", not "another", so strictly the
  ability also triggers on Lumberknot's own death, and `AnyCreatureDies` excludes
  the source. Immaterial — the effect is "put a +1/+1 counter on this creature",
  and a counter on a permanent that has left the battlefield does nothing. Noted
  rather than changed, since no observable behaviour differs.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/191/lumberknot?utm_source=api
**Type line**: `Creature — Treefolk` — {2}{G}{G}, 1/1
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **a** creature dies" — any creature, either player's, including
  tokens: PASS
- Hexproof means opponents cannot target it, but it can still be swept by a
  Blasphemous Act: PASS
- The counters accumulate on a 1/1 base, so it grows without bound: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger and hexproof: `cards_morbid_and_ltb.rs`, `hexproof_filter.rs`
