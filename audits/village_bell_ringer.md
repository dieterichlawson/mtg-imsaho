## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/216/village-bell-ringer?utm_source=api
**Type line**: `Creature — Human Scout` — {2}{W}, 1/4
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```

**Status**: PASS

### Code issues
No issues found.

- "untap **all creatures you control**" — filters to the controller's creatures
  and to tapped ones; no targeting, matching "all" rather than "target".
- Flash is a keyword on the card, so instant-speed casting is the engine's, not
  a hand-rolled timing exception.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/216/village-bell-ringer?utm_source=api
**Type line**: `Creature — Human Scout` — {2}{W}, 1/4
**Oracle text**:
```
Flash (You may cast this spell any time you could cast an instant.)
When this creature enters, untap all creatures you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, **untap all creatures you control**" — all of
  them, no targeting, so hexproof is irrelevant: PASS
- Untapping an attacking creature does not remove it from combat (CR 506.4c),
  which is the point of the card with flash: PASS
- Flash: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mass untap: `cards_complex_creatures.rs`, `combat_rules.rs`
