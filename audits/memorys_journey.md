## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/66/memorys-journey?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Target player shuffles up to three target cards from their graveyard into their library.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target player shuffles **up to three target cards from their graveyard**" —
  two target slots, the second nested `UpToTargets` inside `TwoTargets`, which
  is the shape that made the card uncastable when `valid_targets_for_req` had no
  `UpToTargets` branch: PASS
- "from **their** graveyard" — only the targeted player's, enforced at
  announcement rather than silently discarded at resolution (CR 601.2c): PASS
- CR 109.1 now keeps tokens out of the graveyard enumeration engine-side: PASS
- Castable with zero card targets: PASS
- Flashback {G} is a different colour from the {1}{U} front cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Castability, the up-to slot, and the per-player filter: `multi_target_and_mill.rs:memorys_journey_is_castable`, `:memorys_journey_can_be_cast_with_no_card_targets`, `:memorys_journey_only_offers_the_targeted_players_graveyard`
