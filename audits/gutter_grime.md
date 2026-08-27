## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/186/gutter-grime?utm_source=api
**Type line**: `Enchantment` — {4}{G}
**Oracle text**:
```
Whenever a nontoken creature you control dies, put a slime counter on this enchantment, then create a green Ooze creature token with "This token's power and toughness are each equal to the number of slime counters on Gutter Grime."
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a **nontoken** creature **you control** dies" — both filters present.
  The token check reads the *captured* `dead_is_token` rather than the object,
  and the comment says why: SBA 704.5d has already removed the dead token from
  `state.objects` by the time the trigger resolves, so the object is not there
  to ask.
- The Ooze token's P/T is linked to the slime-counter count on this Gutter Grime
  rather than fixed at creation, so every Ooze grows as more creatures die.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
