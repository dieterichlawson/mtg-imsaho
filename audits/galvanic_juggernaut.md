## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/222/galvanic-juggernaut?utm_source=api
**Type line**: `Artifact Creature — Juggernaut` — {4}, 5/5
**Oracle text**:
```
This creature attacks each combat if able.
This creature doesn't untap during your untap step.
Whenever another creature dies, untap this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature dies, untap this creature" — `AnyCreatureDies`
  (self-excluded), and the untap is conditional on it being tapped and on the
  battlefield.
- The other two clauses are static: "attacks each combat if able" and "doesn't
  untap during your untap step".

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
