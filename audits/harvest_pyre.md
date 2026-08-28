## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/146/harvest-pyre?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

**Status**: PASS

### Code issues
No issues found.

- "As an additional cost to cast this spell, exile X cards from your graveyard.
  Harvest Pyre deals X damage to target creature." — X is fixed by the
  additional cost paid at cast time (CR 601.2f), not chosen again at
  resolution.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`fizzle.rs` (CR 608.2b, including the new hexproof-in-response case), `cards_removal_and_bounce.rs`, `multi_target_and_mill.rs`.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/146/harvest-pyre?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As an additional cost to cast this spell, exile X cards from your
  graveyard**" — paid on casting, so the cards are already in exile while the
  spell is on the stack and countering it does not give them back: PASS
- X is set by how many were exiled, not by an {X} in the mana cost — so there is
  no X-funding prompt: PASS
- "deals X damage to **target creature**", not any target: PASS
- CR 109.1: "X **cards** from your graveyard", so tokens are not payable: PASS
- Damage through `deal_damage`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The additional cost and the scaling damage: `cards_additional_costs.rs`, `cards_burn_and_damage.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/146/harvest-pyre?utm_source=api
**Type line**: `Instant` — {1}{R}
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), via `scripts/oracle_lookup.py`
**Oracle text**:
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```
**Type line**: `Instant` — {1}{R}
**Status**: ISSUE (fixed) — two pieces of dead or unnamed engine contract; the behaviour was correct

### Rulings
None on Scryfall.

### Code issues

- `mtg-engine/src/cards/isd/harvest_pyre.rs:34` and `mtg-engine/src/engine/costs.rs:347` — an engine/card contract spelled as a bare string on both sides.
  - `cards/mod.rs` documents it: `The count is stored on the spell object's card_state as "exile_count".`
  - The engine wrote `obj.card_state.insert("exile_count".into(), ...)` and the card read `o.card_state.get("exile_count")`.
  - This is the second `card_state` key that crosses between engine and card, and the doc I wrote on the first one during the Gutter Grime audit claimed it was *the* one. Both are now named constants — `PT_DEFINED_BY` and the new `EXILE_COUNT` — and that doc says two.

- `mtg-engine/src/cards/isd/harvest_pyre.rs:37` — a card restating a rule the pipeline already applies.
  - Code did: `if count > 0 { ... apply_pending_effect(...) }`
  - `damage::deal_damage` opens with `if amount == 0 { return; }` — CR 120.8, "if a source would deal 0 damage, it does not deal damage at all". The card's guard was dead code, and dropping it changed nothing in the whole workspace. Removed, with the rule cited where X is read.

Everything else is right: `{1}{R}`, Instant, oracle text verbatim, `AdditionalCost::ExileXFromGraveyard` (the general mechanism, with the engine generating the choice prompt rather than the card), `TargetRequirement::Creature`, and damage through `apply_pending_effect` into the one damage pipeline.

### Tricky interactions checked

- X is fixed while casting, not at resolution (CR 601.2b): PASS. The count is recorded when the cost is paid and read back unchanged.
- X = 0 is a legal cast: PASS, and tested — `auto_pick.rs:841` asserts the prompt offers `min = 0`.
- X = 0 deals no damage: PASS, and now for the pipeline's reason rather than the card's.
- "**your** graveyard": PASS, tested with cards in both graveyards.
- The exiled cards really leave: PASS, tested.
- The player chooses which cards, rather than the engine auto-selecting: PASS, `auto_pick.rs:796` — this was a fixed bug and the test is its regression.
- One cast action per target rather than one per subset of the graveyard: PASS, `cards_sacrifice_and_additional_costs.rs:848`.
- "target **creature**", not any target: PASS. Widening to `AnyTarget` fails two tests.
- Damage reaches protection and replacement effects: PASS, `inline_damage.rs:211` puts an Unbreathing Horde in the way.
- Mana value stays 2 whatever X is: X is not `{X}` in the mana cost, so nothing about the cost varies.

### Test coverage

This card is unusually well covered already; the table at `cards_sacrifice_and_additional_costs.rs:787` walks four cases in one place.

- X exiled, X damage, for X = 4/2/0 out of 4, and "your graveyard" only: `cards_sacrifice_and_additional_costs.rs:787` `harvest_pyre_exiles_x_of_your_own_cards_and_deals_x`
- The exile choice is the player's: `auto_pick.rs:796` `bug_harvest_pyre_auto_selects_exile`
- One cast action per target: `cards_sacrifice_and_additional_costs.rs:848`
- The maximum X is exposed to the caster: `cards_sacrifice_and_additional_costs.rs:893`
- Casting for the maximum through the prompt: `cards_sacrifice_and_additional_costs.rs:962`
- Damage goes through the pipeline: `inline_damage.rs:211` `harvest_pyres_chosen_x_still_goes_through_the_pipeline`
- CR 120.8, zero damage is not damage: `damage_pipeline.rs:172` `zero_damage_is_not_damage`, added this audit
- Mana cost, type line, and that it declares no flashback: `card_data_invariants.rs` (added earlier in this run)

### Mutation checking

| Mutation | Before | After |
| --- | --- | --- |
| M1 always deal 1 | 2 tests FAILED | (unchanged) |
| M2 deal `count + 1` | 2 tests FAILED | (unchanged) |
| M3 damage even when X is 0 | did not compile (`count >= 0` on a `u32` is a denied warning) | redone as M6 |
| M5 `TargetRequirement::Creature` -> `AnyTarget` | 2 tests FAILED | (unchanged) |
| M6 drop the card's `count > 0` guard | passed whole workspace — **the guard was dead code** | still passes; the rule now lives only in `deal_damage` |
| M7 remove `deal_damage`'s `amount == 0` guard | passed whole workspace | `zero_damage_is_not_damage` FAILED |

M3 is recorded as a failed attempt rather than a result: `count >= 0` on a `u32` is "comparison is useless due to type limits", which `warnings = "deny"` rejects, so nothing ran. It was redone as M6.

M6 and M7 together are the finding. The card guarded X = 0 and the pipeline guarded it too, so neither guard was observable on its own; removing the card's copy left the rule stated once, in `damage.rs`, where the new test now pins it.

Sources restored from `/tmp/hp.bak` and `/tmp/dmg.bak` after each.

### Suite

`cargo test --workspace --no-fail-fast` exit 0, 1490 passing (was 1489). `cargo check --workspace --all-targets` clean, zero warnings.
