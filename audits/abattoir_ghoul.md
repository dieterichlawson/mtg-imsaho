## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever a creature **dealt damage by this creature this turn** dies, you gain
  life equal to that creature's **toughness**" — reads the captured
  `dead_damaged_by` and `dead_toughness`, which is last-known information
  (CR 603.6d, cited in the code). A dead creature's toughness cannot be read off
  the object afterwards.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/85/abattoir-ghoul?utm_source=api
**Type line**: `Creature — Zombie` — {3}{B}, 3/2
**Oracle text**:
```
First strike
Whenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You'll gain life equal to the creature's **last known toughness before
  it died**. For example, if Abattoir Ghoul deals 3 first-strike damage to a 7/7
  creature and then you give the creature -5/-5 before the regular combat damage
  step, you'll gain 2 life." Both the toughness *and* the `damaged_by` list are
  captured before the zone change clears them (CR 608.2g): PASS
- "a creature **dealt damage by this creature this turn**" — the check is
  `dead_damaged_by.contains(&self_id)`, so a creature that died to something else
  gives nothing: PASS
- CR 603.6d: the trigger resolves even if the Ghoul died in the same combat
  damage step: PASS
- The life gain goes through `change_life`: PASS
- A negative last-known toughness gains 0, not negative life: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Last-known toughness and the damaged-by check: `cards_morbid_and_ltb.rs`, `combat_rules.rs`
