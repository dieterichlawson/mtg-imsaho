## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/65/makeshift-mauler?utm_source=api
**Type line**: `Creature — Zombie Horror` — {3}{U}, 4/5
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
```
**Status**: ISSUE

### Code issues
See below.

- Code did: an `on_resolve` whose entire body was
  `state.move_object(object_id, Zone::Battlefield, registry)`.
- Putting a resolving permanent spell onto the battlefield is the engine's job,
  and it does it: with the method deleted, casting the card still lands it on
  the battlefield. Verified by probe before removing anything.
- Fixed: `on_resolve` removed. The additional cost ("exile a creature card from
  your graveyard") is paid at cast time by the engine's cost pipeline, which is
  where an additional cost belongs (CR 601.2f).

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/65/makeshift-mauler?utm_source=api
**Type line**: `Creature — Zombie Horror` — {3}{U}, 4/5
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
```

**Status**: PASS

### Code issues
No issues found.

`ExileCreaturesFromGraveyard(1)`. Note the type line is `Creature — Zombie
Horror` and both subtypes are present, though the oracle text carries no
ability beyond the additional cost. 4/5, no keywords.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — the shared fixed-count exile tests.
