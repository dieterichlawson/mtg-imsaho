## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/137/curse-of-the-nightly-hunt?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{R}
**Oracle text**:
```
Enchant player
Creatures enchanted player controls attack each combat if able.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The enchanted player still chooses which player or planeswalker each
  creature they control attacks" — `ForceAttack` is an attack *requirement*, not
  a choice of defender: PASS
- Ruling: "If, during the enchanted player's declare attackers step, a creature
  they control is tapped, is affected by a spell or ability that says it can't
  attack, or hasn't been under that player's control continuously since the turn
  began (and doesn't have haste), then it doesn't attack." A requirement cannot
  force an illegal attack (CR 508.1d): PASS
- A static ability, so it covers creatures that arrive after it resolved: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The attack requirement and its exceptions: `combat_requirements.rs`, `curse_and_equip_scope.rs`
