## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/88/bitterheart-witch?utm_source=api
**Type line**: `Creature — Human Shaman` — {4}{B}, 1/2
**Oracle text**:
```
Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
```

**Status**: ISSUE

### Code issues
See below.


- The "target player" was asked for at resolution rather than when the trigger
  went on the stack.
  - Oracle text says: `put it onto the battlefield attached to target player`
  - Code did: `target_requirement: None` on the `SelfDies` trigger, and a
    hand-built player list presented after the search
    (`fn present_player_choice(...)`)
  - CR 603.3d: a triggered ability's targets are chosen as it is put on the
    stack. Asking at resolution meant an opponent responding to the trigger
    could not know whom it would hit, and CR 608.2b never re-checked the choice.
    It also made this card filter hexproof players itself, rather than the
    engine doing it once for everything that targets a player. The trigger now
    declares `TargetRequirement::PlayerOnly` and the handler reads the target it
    was given.

### Tricky interactions checked
- Ruling: "The Curse must be legally able to enchant the player. For example, if
  the player has protection from red, you couldn't put a red Curse onto the
  battlefield this way." CR 303.4h, applied when the Curse would enter — the
  target was chosen before the search, so this cannot be a choice filter: PASS
- The ward arriving between targeting and resolution still stops the Curse: PASS
- "you **may** search" — declining is offered and does nothing: PASS
- A search that finds nothing still shuffles; a search that is declined does not:
  PASS
- Targeting yourself is legal — "target player", not "target opponent": PASS
- Deathtouch, and the trigger is on death (`SelfDies`), so it works from the
  graveyard using last known information: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Targeting at trigger time: `cards_complex_creatures.rs:bitterheart_witch_targets_its_player_when_the_trigger_goes_on_the_stack`
- Finding and attaching, to an opponent and to yourself: `cards_complex_creatures.rs:bitterheart_witch_finds_curse_on_death`, `:bitterheart_witch_can_attach_curse_to_self`
- Declining: `cards_complex_creatures.rs:bitterheart_witch_decline_search`
- Protection and CR 303.4h: `player_protection.rs`
- Hexproof filtered by the engine: `hexproof_filter.rs:bug_bitterheart_witch_hexproof_not_filtered`
