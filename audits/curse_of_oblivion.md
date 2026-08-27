## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/95/curse-of-oblivion?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {3}{B}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
```

**Status**: ISSUE

### Code issues
See below.


- It counted tokens as cards.
  - Oracle text says: `that player exiles two cards from their graveyard`
  - Code did: `state.objects_in_zone(Zone::Graveyard, cursed_player).iter().map(|o| Target::Object(o.id))` — no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so the choice list built
    mid-resolution could offer one. Both the first choice and the chained second
    now ask `state.is_card`.

### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- "**that player** exiles" — the cursed player chooses which cards, not the
  Curse's controller: PASS
- Two or fewer cards are exiled outright with no choice to present: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The upkeep scope and the choice chain: `curse_and_equip_scope.rs`, `cards_auras.rs`
