## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/80/stitched-drake?utm_source=api
**Type line**: `Creature — Zombie Drake` — {1}{U}{U}, 3/4
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
```
**Status**: ISSUE

### Code issues
See below.

Same dead `on_resolve`; removed.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/80/stitched-drake?utm_source=api
**Type line**: `Creature — Zombie Drake` — {1}{U}{U}, 3/4
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
```

**Status**: PASS

### Code issues
No issues found.

Rulings: "exactly one creature card", and "players can only respond once ...
all its costs have been paid". `AdditionalCost::ExileCreaturesFromGraveyard(1)`
is a fixed count paid during the cast. Data-only card otherwise — 3/4 Zombie
Drake with flying, both subtypes.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — the exile happens at cast; `card_data_invariants.rs` covers the printed characteristics.

## Audit — 2026-08-28 20:05

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Flying
**Type line**: Creature — Zombie Drake
**P/T**: 3/4
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/stitched_drake.rs` matches: {1}{U}{U}, Zombie Drake (both subtypes), 3/4, Flying, `AdditionalCost::ExileCreaturesFromGraveyard(1)`. Behavior-free otherwise — the cost machinery is the shared path audited in depth for Skaab Ruinator (217/249).

### Tricky interactions checked
- Ruling: "exactly one creature card; you cannot cast it without exiling ... and you cannot exile additional": the prompt is min=1 max=1; `additional_cost_is_payable` refuses a submitted cast with the wrong count, duplicates, non-creatures, or another player's graveyard (CR 601.2h). PASS
- Ruling: "Players can only respond once ... costs have been paid": the exile happens in `cast_spell` before priority; nothing can save the exiled card. Same mechanism tested for Altar's Reap/Infernal Plunge. PASS
- Cast from hand only (no graveyard-cast ability), so the self-exile question doesn't arise; the shared `o.id != spell` guard is harmless here. PASS
- The player chooses which card to exile — not auto-picked (Bug F fix): prompt lists every eligible creature card. PASS

### Test coverage
- Choice is offered, both candidates listed, exactly one required: `mtg-engine/tests/auto_pick.rs` `bug_f_stitched_drake_enumerates_exile_choices`
- Resolve path (exiled card recorded, spell resolves): shared machinery — `cards_complex_creatures.rs` (Skaab Ruinator), `cards_sacrifice_and_additional_costs.rs`
- Submit-side validation: `submitted_targets.rs` `an_exile_cost_cannot_be_paid_from_an_opponents_graveyard` (engine-generic)

Mutation check: replacing the additional cost with `None` (import sink kept) fails `bug_f_stitched_drake_enumerates_exile_choices` — no exile prompt appears. Bites.
