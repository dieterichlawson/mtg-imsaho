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


### Tricky interactions checked
- Ruling: "The damage dealt by Balefire Dragon's triggered ability **isn't combat
  damage**." It goes through `PendingEffect::DealDamage`, which calls
  `deal_damage` with `DamageKind::NonCombat` — so it emits
  `NonCombatDamageDealt`, does not feed lifelink as combat damage, and does not
  trigger combat-damage watchers: PASS
- "it deals **that much** damage" — the amount is the combat damage that was
  actually dealt, passed into the trigger: PASS
- "each creature **that player** controls" — not your own board: PASS
- CR 113.7a: killing the Dragon with the trigger on the stack does not save the
  defending player's board: PASS
- Protection and prevention apply, because it goes through the pipeline — this
  was one of the cards that used to write damage by hand: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The non-combat damage and protection: `inline_damage.rs`, `cards_burn_and_damage.rs`
