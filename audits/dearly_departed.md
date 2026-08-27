## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/9/dearly-departed?utm_source=api
**Type line**: `Creature — Spirit` — {4}{W}{W}, 5/5
**Oracle text**:
```
Flying
As long as this creature is in your graveyard, each Human creature you control enters with an additional +1/+1 counter on it.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- "As long as this creature is in **your graveyard**" — gated on
  `o.zone == Zone::Graveyard`, so it does nothing from anywhere else.
- The card in a graveyard has an owner, not a controller, and the code compares
  the entering creature's *controller* against the Departed's *owner*. That is
  the right pairing for "each Human creature you control".
- Human check goes through `state.has_subtype`, so a token or a creature granted
  the type is included.

### Test coverage
`enters_tapped_replacement.rs` / `enter_trigger_conditions.rs` cover the ETB-counter replacement path generally
