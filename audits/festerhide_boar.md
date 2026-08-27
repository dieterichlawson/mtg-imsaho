## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/179/festerhide-boar?utm_source=api
**Type line**: `Creature — Boar` — {3}{G}, 3/3
**Oracle text**:
```
Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- Morbid on this card is a **replacement effect**, not a triggered ability:
  "enters with two +1/+1 counters on it if a creature died this turn" modifies
  how the permanent enters (CR 614.1c). Correctly not a trigger, so no
  `should_trigger` is needed here — unlike the morbid cards whose text is "at
  the beginning of ...".
- Shares `helpers::enters_with_counters` with Somberwald Spider, so the two
  cannot drift apart.

### Test coverage
`intervening_if.rs::morbid_enters_with_counters_only_when_a_creature_died`, both arms
