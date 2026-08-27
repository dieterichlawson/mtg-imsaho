## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/213/geist-of-saint-traft?utm_source=api
**Type line**: `Legendary Creature — Spirit Cleric` — {1}{W}{U}, 2/2
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
```

**Status**: ISSUE

### Code issues
See below.


- Its `on_resolve` override was `state.move_object(object_id, Zone::Battlefield,
  registry)` plus `obj.is_legendary = true` — exactly the trait default, written
  out again. Deleted; a guard now fails the build on a card that moves itself
  onto the battlefield.

### Tricky interactions checked
- "create a 4/4 white Angel creature token with flying **that's tapped and
  attacking**" — it does not trigger attack triggers, because it was never
  declared as an attacker (CR 508.4): PASS
- "**Exile** that token at end of combat" — a delayed trigger, and exiling a
  token means it ceases to exist either way: PASS
- Hexproof stops opponents targeting the Geist but not blocking it, and not
  board wipes: PASS
- Legendary: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Angel token and its exile: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
- Hexproof: `hexproof_filter.rs`
