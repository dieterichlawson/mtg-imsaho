## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/55/forbidden-alchemy?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: PASS

### Code issues
No issues found.

- "**Look at** the top four cards" — no reveal, and the code does not emit one.
- One card is auto-selected when only one is available, since there is no choice
  to present; two or more prompts the player.
- The rest go to the graveyard, not back on the library.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/55/forbidden-alchemy?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.
Flashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Look at the top four cards of your library. Put one of them into your hand and
  **the rest into your graveyard**." The rest are a library-to-graveyard move, so
  they emit `CreatureCardMilled` — they used to be moved by hand in the shared
  `ChooseFromRevealed` handler: PASS
- The choice is the caster's, and with only one card revealed it is taken
  automatically rather than presenting a choice of one: PASS
- The spell stays on the stack while the choice is pending, and the engine moves
  it once the chain completes (CR 608.2m): PASS
- Flashback {6}{B}: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill event for the discarded cards: `multi_target_and_mill.rs:forbidden_alchemy_emits_creature_card_milled_for_the_rest`
