## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/98/disciple-of-griselbrand?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/1
**Oracle text**:
```
{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The amount of life you gain is equal to the toughness of the creature
  **as it last existed on the battlefield**, not its toughness in the
  graveyard." Read from the `CreatureDied` event's `last_known_toughness`, which
  `death_event` builds before the zone change (CR 608.2g): PASS
- "Sacrifice a creature" can be the Disciple itself: PASS
- A negative toughness gains 0 life, not negative life: PASS
- The life gain emits `LifeChanged`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Life equal to the sacrificed creature's toughness: `cards_sacrifice_and_additional_costs.rs:disciple_of_griselbrand_gains_life`
- Sacrificing itself: `sacrifice_choice.rs:disciple_of_griselbrand_can_sacrifice_itself`
- The player picks the fodder: `sacrifice_choice.rs:disciple_of_griselbrand_player_picks_highest_toughness_sacrifice`
