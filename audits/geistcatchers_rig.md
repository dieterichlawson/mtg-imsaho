## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/223/geistcatchers-rig?utm_source=api
**Type line**: `Artifact Creature — Construct` — {6}, 4/5
**Oracle text**:
```
When this creature enters, you may have it deal 4 damage to target creature with flying.
```

**Status**: PASS

### Code issues
No issues found.

- "**you may** have it deal 4 damage to **target** creature with flying" — both
  halves handled: the target is locked when the trigger goes on the stack
  (CR 603.3d) and only the may-decision is presented at resolution, through
  `present_optional_target_choice` offering the locked target rather than a fresh
  pick. Re-picking at resolution is the tempting shortcut and would let a player
  dodge a removal spell aimed at the original target.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
