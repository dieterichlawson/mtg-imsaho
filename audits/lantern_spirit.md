## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/62/lantern-spirit?utm_source=api
**Type line**: `Creature — Spirit` — {2}{U}, 2/1
**Oracle text**:
```
Flying
{U}: Return this creature to its owner's hand.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{U}: Return **this creature** to **its owner's** hand" — the owner, so a
  stolen Lantern Spirit returns to its owner: PASS
- The return happens on resolution, so the Spirit can be removed in response and
  the ability then does nothing: PASS
- Returning it to hand while it is attacking removes it from combat: PASS
- Flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The self-bounce: `activated_abilities.rs:lantern_spirit_returns_itself_to_hand`
