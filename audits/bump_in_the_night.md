## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/92/bump-in-the-night?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: ISSUE

### Code issues
See below.


- The life change was written out by hand — read `life`, write `life`, push
  `LifeChanged` — rather than going through `GameState::change_life`, whose own
  doc says why it exists: "Every caller used to hand-roll this ... which meant a
  site that forgot the event silently broke any 'whenever you gain life'
  watcher." Twelve cards were still hand-rolling it. Collapsed onto the helper,
  with a guard to keep it that way.

### Tricky interactions checked
- "Target **opponent** loses 3 life" — `is_valid_target` rejects the caster, so
  you cannot point it at yourself: PASS
- Life **loss**, not damage: it bypasses protection, prevention and damage
  triggers, which is why it does not go through `deal_damage`: PASS
- Flashback {5}{R} is a different colour from the {B} front cost, and the card
  is exiled after the flashback resolution (CR 702.33a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The life loss and the opponent-only restriction: `cards_burn_and_damage.rs`
- Flashback from the graveyard and exile: `cards_flashback.rs`
