## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/49/curiosity?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "'You' refers to the controller of Curiosity, which may be different
  from the controller of the enchanted creature. 'An opponent' refers to an
  opponent of Curiosity's controller." `should_trigger_on_damage_to_player`
  tests `damaged_player != aura.controller`, and the draw goes to the Aura's
  controller: PASS
- Ruling: "If you control Curiosity and it's enchanting an opponent's creature,
  you won't draw a card when that creature deals damage to you": PASS
- Ruling: "Any damage dealt by the enchanted creature to an opponent will cause
  Curiosity to trigger, **not just combat damage**" — `AnyDamageToPlayer`: PASS
- Ruling: "Curiosity doesn't trigger if the enchanted creature deals damage to a
  planeswalker" — the trigger kind is damage to a *player*: PASS
- Ruling: "You draw one card **each time** the enchanted creature deals damage to
  an opponent, no matter how much damage it deals" — one draw per damage event,
  not per point: PASS
- CR 603.2: both halves of "whenever enchanted creature deals damage to an
  opponent" are part of the triggering event and are read at dispatch, so the
  ability does not go on the stack every time any permanent damages any player:
  PASS
- CR 113.7a: destroying Curiosity in response to its own trigger does not
  counter it — the draw still happens: PASS
- "you **may** draw" — a YesNo choice: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The opponent test and the may-draw: `cards_auras.rs`, `trigger_dispatch.rs`
