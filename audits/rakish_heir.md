## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/158/rakish-heir?utm_source=api
**Type line**: `Creature — Vampire` — {2}{R}, 2/2
**Oracle text**:
```
Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "a Vampire **you control**" — the controller check is present, so an
  opponent's Vampire connecting gives nothing: PASS
- "a Vampire", not "another", so the Heir's own combat damage grows it: PASS
- The counter goes on the Vampire that dealt the damage, not on the Heir: PASS
- CR 113.7a: the Heir trading with a blocker in the same combat damage step does
  not counter the trigger: PASS
- `has_subtype`, so a creature Olivia Voldaren turned into a Vampire counts: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The controller filter and the counter placement: `combat_rules.rs`, `subtype.rs`
