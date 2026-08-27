## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/163/skirsdag-cultist?utm_source=api
**Type line**: `Creature — Human Shaman` — {2}{R}{R}, 2/2
**Oracle text**:
```
{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Sacrifice a creature" is a cost, so it is paid on activation and the creature
  is gone while the ability is on the stack: PASS
- "any target" — creature, player or planeswalker: PASS
- The damage is non-combat and emits `NonCombatDamageDealt`: PASS
- The player chooses which creature to sacrifice (`legal_actions` enumerates one
  action per fodder choice): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Damage to a creature and to a player: `cards_sacrifice_and_additional_costs.rs:skirsdag_cultist_deals_2_damage_to_creature`, `:skirsdag_cultist_deals_2_damage_to_player`
- Explicit sacrifice choice: `sacrifice_choice.rs:skirsdag_cultist_explicit_sacrifice`
