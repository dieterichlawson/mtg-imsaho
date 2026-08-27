## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/199/parallel-lives?utm_source=api
**Type line**: `Enchantment` — {3}{G}
**Oracle text**:
```
If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- "under **your** control" — gated on the token controller matching this
  permanent's controller, so an opponent's token-maker is unaffected even when
  it would put tokens under their own control.
- Doubles the count rather than creating a second batch, which is what "creates
  twice that many of those tokens instead" means: one event, modified.

### Test coverage
`token_copy.rs` / `cards_death_triggers_and_tokens.rs` exercise token creation; the doubling itself is NOT TESTED
