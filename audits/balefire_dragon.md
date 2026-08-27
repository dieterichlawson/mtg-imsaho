## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/129/balefire-dragon?utm_source=api
**Type line**: `Creature — Dragon` — {5}{R}{R}, 6/6
**Oracle text**:
```
Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature** deals combat damage to a player" — `CombatDamageToPlayer`,
  the self variant. (`AnyCombatDamageToPlayer` is the other one, for a trigger
  watching some *other* creature — Rakish Heir uses that.)
- "it deals **that much** damage to each creature that player controls" — the
  amount is the combat damage dealt, and the affected creatures are the damaged
  player's, not everyone's.
- CR 113.7a: killing the Dragon in response does not save the board; the ability
  is independent of its source once on the stack.
- All four counter-adders check the creature is still on the battlefield before
  adding, so an ability resolving after its source died does nothing rather than
  putting a counter on a permanent that is not there.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_combat_damage_triggers.rs` — including a table-driven coverage check that every card with this trigger shape in the set is exercised.
