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
## Audit — 2026-08-27 (Tier B)

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


### Tricky interactions checked
- "**As long as this creature is in your graveyard**" — a static ability that
  works from the graveyard, so the `replace_event` is gated on the Departed
  being there rather than on the battlefield: PASS
- Ruling: "The effect is **cumulative**. Human creatures you control will enter
  with a +1/+1 counter **for each** Dearly Departed in your graveyard." Each
  copy is its own object with its own replacement, so two in the graveyard add
  two counters without any special-casing: PASS
- "each **Human** creature **you control**" — both filters, and `has_subtype`
  covers a Human token: PASS
- "enters with an **additional** +1/+1 counter" is a replacement effect
  (CR 614.1c), applied as the creature enters — so an ETB watcher sees the
  counter already there: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The graveyard-static counter: `cards_morbid_and_ltb.rs`, `enters_tapped.rs`
