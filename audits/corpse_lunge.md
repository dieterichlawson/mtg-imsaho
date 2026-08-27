## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/93/corpse-lunge?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As an additional cost to cast this spell**, exile a creature card from your
  graveyard" — paid on casting, so the card is already in exile while the spell
  is on the stack and countering the spell does not give it back: PASS
- "damage equal to the **exiled card's** power" — snapshotted at cast into
  `card_state`, because the card is in exile by resolution: PASS
- CR 109.1: "a creature **card** from your graveyard", so a token is not one:
  PASS
- Damage through `deal_damage`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The additional cost and the snapshotted power: `cards_additional_costs.rs`, `cards_burn_and_damage.rs`
