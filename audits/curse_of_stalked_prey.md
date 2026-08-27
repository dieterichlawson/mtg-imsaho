## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/136/curse-of-stalked-prey?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The ability will trigger when **any** creature deals combat damage to
  the enchanted player, including one controlled by another opponent or even by
  the enchanted player (if combat damage gets redirected somehow)." The handler
  tests only that the damaged player is the cursed one — no controller
  restriction: PASS
- "**combat** damage", so `AnyCombatDamageToPlayer` and not the general damage
  trigger: PASS
- The counter goes on the creature that dealt the damage, and only while it is
  still on the battlefield: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The counter on the damaging creature: `cards_auras.rs`, `combat_rules.rs`
