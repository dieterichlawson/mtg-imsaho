## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/57/frightful-delusion?utm_source=api
**Type line**: `Instant` — {2}{U}
**Oracle text**:
```
Counter target spell unless its controller pays {1}. That player discards a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You must target a spell in order to cast Frightful Delusion. You
  can't cast it without a legal target just to make a player discard a card":
  PASS
- CR 608.2g: the controller **may tap for the {1}** — having nothing floating is
  not the same as being unable to pay, which is the bug this card's tests were
  written for: PASS
- "That player discards a card" happens whether or not the spell was countered:
  PASS
- Countering uses `move_countered_spell` (CR 701.5a), not the resolving-spell
  cleanup path: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pay-or-counter choice and the unconditional discard: `resolution_time_checks.rs:auto_counter_when_controller_has_no_floating_mana_but_has_lands`, `:player_offered_choice_when_controller_has_floating_mana`, `:claiming_to_pay_without_the_mana_does_not_save_the_spell`
