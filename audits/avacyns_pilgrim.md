## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/170/avacyns-pilgrim?utm_source=api
**Type line**: `Creature — Human Monk` — {G}, 1/1
**Oracle text**:
```
{T}: Add {W}.
```
**Status**: PASS

### Code issues
No issues found.

A single free `{T}: Add {W}` mana ability. The tap-cost conditions — battlefield, untapped, summoning sickness with the haste exception (CR 302.6) — are the engine's, applied centrally, and covered by `tap_cost_legality.rs`.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier D)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/170/avacyns-pilgrim?utm_source=api
**Type line**: `Creature — Human Monk` — {G}, 1/1
**Oracle text**:
```
{T}: Add {W}.
```

**Status**: PASS

### Code issues
No issues found.

"{T}: Add {W}" declared as a `ManaAbilityDef` with `requires_tap: true`,
`cost: ManaCost::free()`, `has_side_effects: false` — so it is a mana ability
(CR 605.1a): it does not use the stack and cannot be responded to. Human Monk,
both subtypes present.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_lands_and_mana_sources.rs` — taps for {W}; `mana_abilities.rs` covers the no-stack property.

## Audit — 2026-08-28 19:26

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Avacyn's Pilgrim"`, https://scryfall.com/card/isd/170/avacyns-pilgrim
**Oracle text**:
```
{T}: Add {W}.
```
**Type line**: Creature — Human Monk
**Mana cost**: {G}   **P/T**: 1/1
**Rulings**: none on Scryfall for this card.
**Status**: PASS (the submit path's refusal gained its test)

### Code issues
No issues found in `mtg-engine/src/cards/isd/avacyns_pilgrim.rs`.

`{G}`, `Creature`, `subtypes: ["Human", "Monk"]` — both — 1/1, oracle text verbatim, one
`ManaAbilityDef`: free, `requires_tap`, produces one white. Being a mana ability (CR 605.1a)
rather than an activated one is what makes it visible to the auto-tap planner — the Shimmering
Grotto lesson.

The summoning-sickness and tapped gates live in `available_mana_abilities`, which **both** the
offer path and `activate_mana_source` (the submit path) consult — one availability function, so
the two cannot disagree. A submitted activation for a source that cannot pay {T} finds no such
ability and is a no-op.

### Tricky interactions checked
- **Offered and produces {W}**: PASS.
- **Not offered while summoning-sick** (CR 302.6 — a creature's {T} cost): PASS.
- **Submitted anyway while sick or tapped: no phantom mana**: PASS, newly pinned — probed first,
  found correct, then tested. This is the session's offer/submit theme applied to mana.
- **A mana ability skips the stack**: engine-wide, pinned at Shimmering Grotto.
- **A Human for Human-watchers, a Monk for nothing in this pool**: prop duty, well exercised.
- **Green creature producing white**: nothing special — `produced` is independent of cost.

### Test coverage
- taps for white: `cards_lands_and_mana_sources.rs:209 avacyns_pilgrim_taps_for_white`
- not offered while sick: `cards_lands_and_mana_sources.rs:223 avacyns_pilgrim_cant_tap_with_summoning_sickness`
- submitted while sick or tapped produces nothing:
  `submitted_targets.rs:~300 a_mana_ability_submitted_for_a_source_that_cannot_pay_produces_nothing` (NEW)

Mutation-checked: making the submit path read the card's raw ability list instead of
`available_mana_abilities` fails the new test — which is the pin on the sharing itself.

### Changes made
- `submitted_targets.rs`: the submit-path test. No code change.
