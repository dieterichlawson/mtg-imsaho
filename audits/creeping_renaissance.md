## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/174/creeping-renaissance?utm_source=api
**Type line**: `Sorcery` — {3}{G}{G}
**Oracle text**:
```
Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Status**: ISSUE

### Code issues
See below.

- Oracle text says: `Return all cards of the chosen type from your graveyard to your hand.`
- The resolution handler lives in the engine (`choices.rs`, the `ChooseCardType`
  arm) and filtered the graveyard by `has_card_type` alone.
- CR 109.1: a token is not a card, and a token sits in a graveyard until the next
  state-based action check. Same defect as the eight card-side ones fixed earlier
  in this audit, one layer down — the first sweep only looked at `src/cards/isd/`.
- Fixed: the filter now goes through `state.is_card`.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/174/creeping-renaissance?utm_source=api
**Type line**: `Sorcery` — {3}{G}{G}
**Oracle text**:
```
Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.
Flashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.

Ruling: "The permanent types are artifact, creature, enchantment, land, and planeswalker."
`on_resolve` offers exactly those five and nothing else. The chosen type is
applied in `engine/actions/choices.rs:129` over
`objects_in_zone(Zone::Graveyard, controller)` — which filters by *owner*, so
"your graveyard" is right — and filters `state.is_card(*id)` so a token in the
graveyard is not returned (CR 109.1). "Return **all** cards", not a choice: no
subset prompt, every match moves.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_graveyard_interaction.rs` — chosen-type return and the token exclusion.
