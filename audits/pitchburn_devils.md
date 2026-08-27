## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/156/pitchburn-devils?utm_source=api
**Type line**: `Creature — Devil` — {4}{R}, 3/3
**Oracle text**:
```
When this creature dies, it deals 3 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.

'it deals 3 damage to **any target**' — targeted and locked at trigger time; 'any target' includes planeswalkers, and the damage goes through the pipeline so CR 120.3c loyalty removal applies.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_source_independence.rs` (a dies trigger resolving after its source is gone).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/156/pitchburn-devils?utm_source=api
**Type line**: `Creature — Devil` — {4}{R}, 3/3
**Oracle text**:
```
When this creature dies, it deals 3 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this creature dies, **it** deals 3 damage to any target" — the source is
  the Devils, from the graveyard, using last known information (CR 608.2g): PASS
- The target is chosen when the death trigger goes on the stack (CR 603.3d), and
  "any target" includes a planeswalker: PASS
- Damage through the pipeline: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death damage and planeswalker targeting: `damage_helper.rs:an_ability_that_picks_any_target_on_resolution_offers_a_planeswalker`, `cards_morbid_and_ltb.rs`
