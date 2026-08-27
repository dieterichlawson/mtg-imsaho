## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/157/rage-thrower?utm_source=api
**Type line**: `Creature — Human Shaman` — {5}{R}, 4/2
**Oracle text**:
```
Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature dies" — `AnyCreatureDies`, and the engine's
  collection filters `o.id != dead_id`, so the source is excluded exactly as
  "another" requires.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/157/rage-thrower?utm_source=api
**Type line**: `Creature — Human Shaman` — {5}{R}, 4/2
**Oracle text**:
```
Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Rage Thrower **dies at the same time as another creature**, its
  ability will trigger." The other creature's death is the event, and CR 603.6d
  lets the trigger resolve from the graveyard: PASS
- "Whenever **another** creature dies" — declared as `AnyCreatureDies` only, with
  no `SelfDies`, so its own death alone does not trigger it: PASS
- "deals 2 damage to **target player or planeswalker**" — not any target, so it
  cannot be pointed at a creature: PASS
- Damage through the pipeline: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger and the target restriction: `cards_morbid_and_ltb.rs`, `damage_helper.rs`
