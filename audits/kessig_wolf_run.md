## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/243/kessig-wolf-run?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{X}{R}{G}, {T}: Target creature gets **+X/+0**" — X is the amount funded, and
  CR 107.3e means X is 0 in a cost paid other than by casting only for costs
  that are not announced; here X is announced, so the funding prompt is correct:
  PASS
- X = 0 is a legal activation: trample with no pump: PASS
- Trample until end of turn: PASS
- The pump lands on resolution, not on activation (CR 602.2a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both X values and the trample: `cards_lands_and_mana_sources.rs:x_equals_0_gives_trample_only`, `:x_equals_3_gives_plus_3`, `cards_rule_modifiers.rs:kessig_wolf_run_grants_power_and_trample`
