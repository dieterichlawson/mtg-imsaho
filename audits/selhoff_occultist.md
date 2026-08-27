## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/73/selhoff-occultist?utm_source=api
**Type line**: `Creature — Human Rogue` — {2}{U}, 2/3
**Oracle text**:
```
Whenever this creature or another creature dies, target player mills a card.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature or another** creature dies" — both kinds declared,
  same as Falkenrath Noble.
- "target player mills a card" — targeted, so the target is locked when the
  trigger goes on the stack (CR 603.3d).

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
