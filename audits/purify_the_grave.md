## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/27/purify-the-grave?utm_source=api
**Type line**: `Instant` — {W}
**Oracle text**:
```
Exile target card from a graveyard.
Flashback {W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: ISSUE

### Code issues
See below.


- The engine's graveyard target enumeration offered tokens.
  - Oracle text says: `Exile target card from a graveyard.`
  - Code did: `state.objects.values().filter(|o| o.zone == Zone::Graveyard)` in
    `engine/targeting.rs`, with no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so an enumeration taken in between
    can see one. This is the *engine's* variant list rather than this card's
    code — six `TargetRequirement` variants name a "card" in a graveyard or in
    exile and none of them asked, so the fix is shared by everything that uses
    them, and `stack.rs`'s resolution-time re-check now asks too.

### Tricky interactions checked
- "from **a** graveyard" — any graveyard, not only your own: PASS
- Flashback {W}, the same cost as the front face, and the card is exiled after
  the flashback resolution: PASS
- Exiling the card the spell itself would go to is fine — the spell is on the
  stack while it resolves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not offered: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_a_targetable_card`
- Exile from either graveyard, and flashback: `cards_flashback.rs`
