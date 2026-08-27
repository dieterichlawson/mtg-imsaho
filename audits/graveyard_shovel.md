## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/225/graveyard-shovel?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
```

**Status**: ISSUE

### Code issues
See below.


- Three sites counted tokens as cards.
  - Oracle text says: `Target player exiles a card from their graveyard.`
  - Code did: `state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == *target_player)` — no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so a choice list built
    mid-resolution could offer one. The availability check and
    `is_valid_target` had the same gap. All three now ask `state.is_card`.

### Tricky interactions checked
- Ruling: "The targeted player chooses which card to exile when the ability
  resolves" — the `ResolutionChoice` is presented to the *targeted* player, not
  the Shovel's controller: PASS
- "If it's a creature card, **you** gain 2 life" — the life goes to the Shovel's
  controller, not the targeted player, and emits `LifeChanged`: PASS
- With exactly one card there is no choice to present: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not offered as a choice: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_offered_as_a_card_to_choose`
- Exile and life gain: `cards_lands_and_mana_sources.rs:graveyard_shovel_exiles_and_gains_life`, `graveyard_shovel.rs`
