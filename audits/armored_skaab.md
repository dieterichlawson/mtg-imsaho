## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/43/armored-skaab?utm_source=api
**Type line**: `Creature — Zombie Warrior` — {2}{U}, 1/4
**Oracle text**:
```
When this creature enters, mill four cards.
```

**Status**: PASS

### Code issues
No issues found.

'mill four cards' goes through `engine::mill_cards`, so the milled cards emit the events that mill-watchers in this set (Undead Alchemist, Selhoff Occultist) rely on.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
