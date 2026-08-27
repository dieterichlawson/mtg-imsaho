## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/84/undead-alchemist?utm_source=api
**Type line**: `Creature — Zombie` — {3}{U}, 4/2
**Oracle text**:
```
If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.
Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**If** a Zombie you control **would** deal combat damage to a player,
  **instead** that player mills that many cards" — a replacement effect, so no
  damage is dealt and damage triggers do not fire: PASS
- "a Zombie **you control**", including Zombie tokens: PASS
- "Whenever a creature card is put into an **opponent's** graveyard from their
  library" — the opponent filter is the collector's, which is why every mill in
  the set now emits `CreatureCardMilled` and lets the collector decide: PASS
- "exile that card **and** create a 2/2 black Zombie" — both, once per card: PASS
- CR 109.1: a creature *card*, so its own Zombie tokens dying do not feed it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage replacement and the mill trigger: `token_is_not_a_card.rs:mindshrieker_milled_creature_triggers_undead_alchemist`, `multi_target_and_mill.rs`
