## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/67/mindshrieker?utm_source=api
**Type line**: `Creature — Spirit Bird` — {1}{U}, 1/1
**Oracle text**:
```
Flying
{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- The mill goes through `mill_one`, so a creature card among the milled emits
  `CreatureCardMilled` (Undead Alchemist is in this set): PASS
- The +X/+X is applied only while Mindshrieker is still on the battlefield: PASS
- X is the milled card's mana value, read after the move: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill emits CreatureCardMilled: `token_is_not_a_card.rs`
- Pump from the milled card's mana value: `cards_activated_abilities.rs`
