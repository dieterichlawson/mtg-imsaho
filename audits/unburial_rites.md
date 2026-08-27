## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/122/unburial-rites?utm_source=api
**Type line**: `Sorcery` — {4}{B}
**Oracle text**:
```
Return target creature card from your graveyard to the battlefield.
Flashback {3}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Return target creature **card** from **your** graveyard to the battlefield" —
  a card (CR 109.1, now enforced in the engine's graveyard enumeration), from
  the caster's own graveyard: PASS
- The creature returns under the *caster's* control, not its owner's — the card
  says "to the battlefield" with no owner clause: PASS
- Its enters-the-battlefield triggers fire: PASS
- The spell stays on the stack while the choice chain runs, and the engine moves
  it afterwards (CR 608.2m): PASS
- Flashback {3}{W} is a different colour from the {4}{B} front cost, which is
  the card's whole design: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The reanimation and the flashback: `cards_flashback.rs`, `spell_cleanup.rs`
