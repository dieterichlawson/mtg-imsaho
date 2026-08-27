## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/32/slayer-of-the-wicked?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 3/2
**Oracle text**:
```
When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
```

**Status**: PASS

### Code issues
No issues found.

- "**you may** destroy **target** Vampire, Werewolf, or Zombie" — same locked-target
  plus optional-decision shape as Geistcatcher's Rig.
- Destroys through `PendingEffect::Destroy`, so indestructible and regeneration
  apply; the oracle says destroy, not exile or sacrifice.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/32/slayer-of-the-wicked?utm_source=api
**Type line**: `Creature — Human Soldier` — {3}{W}, 3/2
**Oracle text**:
```
When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature enters, **destroy target Vampire, Werewolf, or Zombie**" —
  all three subtypes, and the trigger declares its target so hexproof is
  filtered by the engine: PASS
- `has_subtype` covers granted subtypes, so a creature Olivia Voldaren turned
  into a Vampire is a legal target: PASS
- `try_destroy`, so indestructible survives: PASS
- No legal target means the trigger is not put on the stack: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The subtype filter and hexproof: `hexproof_filter.rs:an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`, `subtype.rs`
