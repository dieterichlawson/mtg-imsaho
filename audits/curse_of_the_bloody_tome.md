## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/50/curse-of-the-bloody-tome?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {2}{U}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player mills two cards.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly` and the Curse subtype:
  PASS
- Ruling: "If the enchanted player has only one card in their library, they put
  that card into their graveyard" — `mill_cards` stops at an empty library
  rather than making the player lose: PASS
- The mill goes through the pipeline, so a creature card among the two emits
  `CreatureCardMilled`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep mill: `cards_auras.rs`, `curse_and_equip_scope.rs`
