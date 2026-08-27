## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/100/falkenrath-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {3}{B}, 2/2
**Oracle text**:
```
Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature or another** creature dies" — declares *both*
  `SelfDies` and `AnyCreatureDies`, which is the correct pair given
  `AnyCreatureDies` excludes the source. One or the other alone would silently
  drop half the wording.
- "target player loses 1 life and **you** gain 1 life" — the drain is not
  symmetric between two players; the gain is the controller's.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
