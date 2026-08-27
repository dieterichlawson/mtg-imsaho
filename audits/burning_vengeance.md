## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/133/burning-vengeance?utm_source=api
**Type line**: `Enchantment` — {2}{R}
**Oracle text**:
```
Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **you** cast a spell **from your graveyard**" — the caster must be
  the enchantment's controller, and the spell must have been cast from a
  graveyard, which in this set means flashback: PASS
- Ruling: "Burning Vengeance doesn't trigger when you **activate an ability** of
  a card in your graveyard" — only casting, not activating: PASS
- "deals 2 damage to **any target**", chosen when the trigger goes on the stack
  (CR 603.3d): PASS
- The trigger resolves before the spell that caused it, since it went on the
  stack on top: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Triggering on flashback casts: `cards_flashback.rs`, `trigger_dispatch.rs`
