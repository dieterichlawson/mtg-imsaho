## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/180/full-moons-rise?utm_source=api
**Type line**: `Enchantment` — {1}{G}
**Oracle text**:
```
Werewolf creatures you control get +1/+0 and have trample.
Sacrifice this enchantment: Regenerate all Werewolf creatures you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Werewolf** creatures you control get +1/+0 and have trample" — a static
  ability, so it covers Werewolves that arrive later, and it follows a
  transformed Werewolf on both faces: PASS
- "**Sacrifice this enchantment**: Regenerate all Werewolf creatures you
  control" — the sacrifice is a cost, so the Rise is gone while the ability is
  on the stack, and the shields still land: PASS
- The shields are given on resolution, so the set of Werewolves is read then
  (CR 611.2c): PASS
- The static +1/+0 ends when the Rise is sacrificed; the shields do not: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The static buff and the regeneration: `activated_no_stack.rs:full_moons_rise_shields_on_resolution`, `werewolf_cards.rs`
