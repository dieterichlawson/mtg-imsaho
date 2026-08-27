## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/218/cellar-door?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "the **bottom** card of their library" — `library_order[len - 1]`, and the
  bottom is what `mill_cards` cannot express, which is why this goes through
  `mill_one` directly: PASS
- "If it's a creature card, **you** create" — the token goes to Cellar Door's
  controller, not the milled player: PASS
- The Zombie token carries its subtype via `create_token_with_subtypes`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Milling from the bottom still emits CreatureCardMilled: `multi_target_and_mill.rs:cellar_door_emits_creature_card_milled`
- The Zombie is created for a creature card: `cards_complex_creatures.rs:cellar_door_creates_zombie_when_milling_creature`
