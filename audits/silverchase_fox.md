## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/31/silverchase-fox?utm_source=api
**Type line**: `Creature — Fox` — {1}{W}, 2/2
**Oracle text**:
```
{1}{W}, Sacrifice this creature: Exile target enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{W}, **Sacrifice this creature**: Exile target enchantment" — the
  sacrifice is a cost, paid on activation, so the Fox is in the graveyard while
  the ability is on the stack: PASS
- **Exile**, not destroy, so indestructible does not save the enchantment and it
  does not reach a graveyard: PASS
- "target enchantment" includes an Aura or a Curse: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The sacrifice cost and the exile: `cards_sacrifice_and_additional_costs.rs`, `sacrifice_choice.rs`
