## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/52/deranged-assistant?utm_source=api
**Type line**: `Creature — Human Wizard` — {1}{U}, 1/1
**Oracle text**:
```
{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{T}, **Mill a card**: Add {C}" — a mana ability with a side effect, which is
  why it is declared `has_side_effects`: PASS
- The mill goes through the pipeline, so a creature card among it emits
  `CreatureCardMilled`: PASS
- A mana ability does not use the stack (CR 605.1a), so the mill happens
  immediately: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mana and the mill: `cards_lands_and_mana_sources.rs`, `mana_ability_offers.rs`
