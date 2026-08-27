## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/224/ghoulcallers-bell?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{T}: Each player mills a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Each** player mills a card" — no targeting, so it cannot be responded to by
  making a player untargetable, and it hits its own controller too: PASS
- The mill goes through `mill_cards`, so a creature card among them emits
  `CreatureCardMilled`: PASS
- A player with an empty library mills nothing rather than losing: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both players mill: `cards_lands_and_mana_sources.rs:ghoulcallers_bell_mills_both_players`
