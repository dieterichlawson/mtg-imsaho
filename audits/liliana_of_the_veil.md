## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/105/liliana-of-the-veil?utm_source=api
**Type line**: `Legendary Planeswalker — Liliana` — {1}{B}{B}
**Oracle text**:
```
+1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "When Liliana's first ability resolves, first the player whose turn it
  is chooses a card in hand without revealing it, then each other player in turn
  order does the same. **Then all the chosen cards are discarded at the same
  time.**" The choices are queued and collected before anything leaves a hand,
  so a discard trigger (Murder of Crows is in this set) cannot fire while another
  player is still choosing: PASS
- Ruling: "You can activate Liliana's first ability even if some or all players
  will be unable to discard a card" — a player with an empty hand is skipped
  rather than blocking the ability: PASS
- "−2: **Target player** sacrifices a creature" — the targeted player chooses
  which, and sacrifice bypasses indestructible: PASS
- Ruling: "A pile can be empty. If the player chooses an empty pile, no
  permanents will be sacrificed": PASS
- Starting loyalty 3, and the −6 is not activatable below six: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The simultaneous discard: `cards_planeswalkers.rs`, `simultaneous_events.rs`
- The −2 sacrifice choice: `sacrifice_choice.rs`
