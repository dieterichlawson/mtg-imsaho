## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/171/boneyard-wurm?utm_source=api
**Oracle text**:
```
Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.`)
- Code did: filtered the graveyard by creature-ness alone, with no card/token distinction.
- CR 109.1: a token is not a card. CR 111.7 removes a token from a graveyard as
  a state-based action, so between the moment it dies and the next SBA check it
  really is sitting there — the same window a dies-trigger sees. Measured
  directly on Boneyard Wurm: 2/2 with one creature card and one just-died token
  in the yard, 1/1 the instant SBAs ran. The oracle's answer is 1/1 throughout.
- Fixed: the graveyard filter now goes through `state.is_card`.

### How this was found
A sweep for cards whose oracle says "cards in a graveyard" against code that
never distinguishes tokens. Thirteen cards matched; five already guarded
(Gnaw to the Bone, Moorland Haunt, Past in Flames, Runechanter's Pike,
Splinterfright) and eight did not.

Splinterfright and Boneyard Wurm are the instructive pair — near-identical
text, adjacent in the set. `token_is_not_a_card.rs::cda_does_not_count_tokens_in_graveyard`
covered Splinterfright, which is why Splinterfright alone had the guard.

### Test coverage
`token_is_not_a_card.rs::a_token_in_a_graveyard_is_not_a_creature_card` —
**added by this audit**, covers Boneyard Wurm and Splinterfright together and
fails against the unfixed code.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/171/boneyard-wurm?utm_source=api
**Type line**: `Creature — Wurm` — {1}{G}, */*
**Oracle text**:
```
Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
```

**Status**: ISSUE (fixed)

### Code issues
See below.

Ruling: "The ability that defines Boneyard Wurm's power and toughness works in
all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it
will count itself." The `dynamic_pt` has no zone guard and no self-exclusion, so
both halves hold — and `is_card` keeps tokens out of the count (CR 109.1).

But it reads its own `controller` to pick whose graveyard to count, and that
was stale off the battlefield:

- CR 108.4 says: a card has a controller only while it represents a permanent
  or a spell; elsewhere its owner is treated as its controller.
- `state.rs::move_object` did: cleared tapped, counters, attachments, damage and
  the transform flag when a permanent left the battlefield — but left
  `controller` set to whoever last controlled the permanent.

So a Boneyard Wurm stolen by an opponent and then killed sat in its owner's
graveyard still reading the thief as its controller, and counted the thief's
creature cards instead of its owner's. Fixed in the engine, not the card:
`move_object` now assigns `obj.controller = obj.owner` on leaving the
battlefield. Last known information for death triggers is unaffected — that is
captured in `pre_move_controller` before the move.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`control_change.rs::a_card_leaving_the_battlefield_stops_having_a_controller` — mutation-checked. `cards_evasion_and_graveyard_pt.rs` covers the count itself.

## Audit — 2026-08-28 19:28

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Boneyard Wurm"`, https://scryfall.com/card/isd/171/boneyard-wurm
**Oracle text**:
```
Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
```
**Type line**: Creature — Wurm
**Mana cost**: {1}{G}   **P/T**: */*
**Rulings** (1, 2018-12-07): "The ability that defines Boneyard Wurm's power and toughness works
in all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it will count
itself."
**Status**: PASS (the count's two qualifying words gained their rows)

### Code issues
No issues found in `mtg-engine/src/cards/isd/boneyard_wurm.rs`.

`{1}{G}`, `Creature`, `subtypes: ["Wurm"]`, the `Some(0)/Some(0)` CDA sentinel for a printed
*/*, oracle text verbatim. `dynamic_pt` counts creature cards (`is_card` — CR 109.1, no tokens;
`is_creature` through the characteristics layer) in the controller's graveyard, live.

The ruling holds by construction: `effective_power` consults `dynamic_pt` with no zone filter,
so the CDA works in every zone (CR 604.3); a Wurm in the graveyard is in the set it counts, so
it counts itself; and off the battlefield the object's `controller` field has reverted to its
owner, which is what "your" means for an object with no controller (CR 109.5).

### Tricky interactions checked
- **0/0 with an empty graveyard; N/N with N creature cards**: PASS.
- **A land in your graveyard, and a creature card in the opponent's, count for nothing**: PASS,
  newly pinned.
- **The ruling — counts itself from the graveyard**: PASS, and directly asserted twice in the
  Corpse Lunge tests ("in the graveyard the Wurm counts itself").
- **In exile it stops counting the graveyard it left**: PASS — the Corpse Lunge pair, including
  the lone-Wurm 0/0 whose 0 damage is no damage (CR 120.8).
- **Read at resolution, not carried from cast**: PASS — the two-latecomers Corpse Lunge test.
- **A 0/0 Wurm dies at SBA (CR 704.5f)**: cast with an empty graveyard, it dies unless
  something died first. Pipeline behaviour; the counts above are the card's part.
- **Last known toughness at death**: `death_event` captures `effective_toughness` pre-move.

### Test coverage
- battlefield count with both exclusions:
  `cards_upkeep_triggers_and_curses.rs:32 boneyard_wurm_pt_equals_creatures_in_graveyard` (extended)
- the all-zones ruling, three ways: `cards_sacrifice_and_additional_costs.rs:757`, `:785`, `:810`
  (the Corpse Lunge family)
- the CDA sentinel convention: `card_data_invariants.rs:1673`

Mutation-checked: counting any card fails the extended test; counting every graveyard fails it.

### Changes made
- `cards_upkeep_triggers_and_curses.rs`: the land and opponent's-card rows. No code change.
