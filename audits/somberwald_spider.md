## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/202/somberwald-spider?utm_source=api
**Type line**: `Creature — Spider` — {4}{G}, 2/4
**Oracle text**:
```
Reach (This creature can block creatures with flying.)
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

- Identical morbid replacement to Festerhide Boar, through the same helper.
  Reach is a keyword rather than a continuous effect, which is correct.

### Test coverage
`intervening_if.rs::morbid_enters_with_counters_only_when_a_creature_died`, both arms
