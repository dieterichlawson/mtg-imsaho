## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/207/tree-of-redemption?utm_source=api
**Type line**: `Creature — Plant` — {3}{G}, 0/13
**Oracle text**:
```
Defender
{T}: Exchange your life total with this creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{T}: **Exchange** your life total with this creature's toughness" — both
  halves, and the exchange goes through `change_life` so LifeChanged is emitted
  like any other life change: PASS
- The ability does nothing if the Tree is no longer on the battlefield when it
  resolves — destroyed or bounced in response — because the exchange is with
  *this creature's* toughness (CR 608.2): PASS
- Defender, 0/13: PASS
- The toughness it takes on is the life total it gave away, so a subsequent
  exchange is not the printed 13: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The exchange, and being removed in response: `activated_no_stack.rs:tree_of_redemption_exchanges_on_resolution`, `token_is_not_a_card.rs:tree_destroyed_in_response_no_exchange`, `:tree_bounced_in_response_no_exchange`, `:tree_on_the_battlefield_exchanges_normally`
