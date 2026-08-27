## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/78/snapcaster-mage?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 2/1
**Oracle text**:
```
Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **instant or sorcery** card in **your** graveyard" —
  `GraveyardCardOwnedByCaster` plus the card's own type filter, and CR 109.1 now
  keeps tokens out of that enumeration engine-side: PASS
- "The flashback cost is equal to its **mana cost**" — the mana cost, not the
  mana value, so colours are preserved: PASS
- "gains flashback **until end of turn**", so it lapses if unused: PASS
- Flash, so it can be cast at instant speed to give an instant flashback in
  response: PASS
- Casting the granted flashback exiles the card (CR 702.33a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Granting flashback and the exile after: `cards_flashback.rs`
