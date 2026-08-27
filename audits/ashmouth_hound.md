## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/128/ashmouth-hound?utm_source=api
**Type line**: `Creature — Elemental Dog` — {1}{R}, 2/1
**Oracle text**:
```
Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "blocks **or becomes blocked by** a creature" — both directions, two declared
  triggers: PASS
- The damage is dealt to *that* creature, the one in the blocking relationship,
  not to every blocker: PASS
- Damage through `deal_damage`, so protection and prevention apply: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both directions: `combat_rules.rs`, `cards_complex_creatures.rs`
