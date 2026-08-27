## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/117/skirsdag-high-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/2
**Oracle text**:
```
Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — ... **Activate only if a creature died this turn**" is an activation
  restriction, not an intervening-if: the ability is simply not offered: PASS
- "{T}, **Tap two untapped creatures you control**" — a cost the
  `ActivatedAbilityDef` cannot express, so it is paid in `pay_activation_cost`
  (CR 601.2h) and the two creatures stay tapped even if the ability is countered:
  PASS
- The two tapped creatures may be summoning sick — tapping as a cost is not the
  {T} symbol (CR 302.6): PASS
- The 5/5 Demon token carries its subtype and flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The cost paid at activation and the token on resolution: `activated_no_stack.rs:skirsdag_high_priests_tap_cost_is_paid_at_activation`, `:skirsdag_high_priest_makes_its_demon_on_resolution`, `:skirsdag_summoning_sick_creature_can_be_tapped`
