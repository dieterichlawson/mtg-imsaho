## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/39/unruly-mob?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
Whenever another creature you control dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature **you control** dies" — self-excluded by the
  trigger kind, controller-filtered in the handler.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/39/unruly-mob?utm_source=api
**Type line**: `Creature — Human` — {1}{W}, 1/1
**Oracle text**:
```
Whenever another creature you control dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **another** creature **you control** dies" — both the self-exclusion
  and the controller check: PASS
- It counts tokens dying, since a token is a creature: PASS
- CR 603.6d: the Mob dying alongside another creature still gets its trigger,
  though the counter lands on a creature that is gone: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger: `cards_morbid_and_ltb.rs`
