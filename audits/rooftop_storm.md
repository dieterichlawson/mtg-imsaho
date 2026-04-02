# Audit: Rooftop Storm

**Date:** 2026-04-02
**Auditor:** Claude (Opus 4.6)

## Official Oracle Text (Scryfall, cached 2026-04-01)

- **Name:** Rooftop Storm
- **Cost:** {5}{U}
- **Type:** Enchantment
- **Oracle Text:** "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast."
- **P/T:** N/A

### Official Rulings
1. [2011-09-22] You must still pay any mandatory additional costs, such as exiling a creature card from your graveyard for Makeshift Mauler.
2. [2011-09-22] The mana cost and mana value of the spell are unchanged. Rooftop Storm only changes what you pay.

## Card Data Review

| Field            | Expected      | Implemented   | Status |
|------------------|---------------|---------------|--------|
| Name             | Rooftop Storm | Rooftop Storm | OK     |
| Mana Cost        | {5}{U}        | {5}{U}        | OK     |
| Card Types       | Enchantment   | Enchantment   | OK     |
| Supertypes       | (none)        | (none)        | OK     |
| Subtypes         | (none)        | (none)        | OK     |
| P/T              | N/A           | N/A           | OK     |
| Oracle Text      | (matches)     | (matches)     | OK     |
| Keywords         | (none)        | (none)        | OK     |
| continuous_effects | (see below) | empty `vec![]`| SEE ISSUES |

## Implementation Analysis

### Card File (`mtg-engine/src/cards/isd/rooftop_storm.rs`)

The card file declares only static card data with **no behavioral logic**. The `continuous_effects` vector is empty. The doc comment misleadingly says:

> "Implementation: Uses ReduceCost with a high reduction value for Zombie creature spells."

This is **factually incorrect** -- the card does not use `ReduceCost` at all. Instead, the behavior is hardcoded in the engine.

### Engine Handling (`mtg-engine/src/engine.rs`, lines 82-90)

The actual logic is a special case in `effective_spell_cost()`:

```rust
// Check for Rooftop Storm: Zombie creature spells cost {0}.
if is_creature && subtypes.iter().any(|s| s == "Zombie") {
    let has_rooftop_storm = state.objects.values().any(|o| {
        o.zone == Zone::Battlefield && o.controller == caster && o.name == "Rooftop Storm"
    });
    if has_rooftop_storm {
        return ManaCost::free();
    }
}
```

### Tests (`mtg-engine/tests/tier14_cards.rs`)

Two tests exist:
1. `rooftop_storm_makes_zombies_free` -- Zombie creature (Walking Corpse) is castable with no mana when Rooftop Storm is on the battlefield.
2. `rooftop_storm_no_free_non_zombies` -- Non-Zombie creature (Grizzly Bears) is not made free.

## Issues Found

### ISSUE 1 (Semantic / Medium): Alternative cost implemented as unconditional free cast

**Oracle text:** "You **may** pay {0} **rather than** pay the mana cost..."

This is an **alternative cost** per rule 601.2b. The current implementation unconditionally returns `ManaCost::free()` when Rooftop Storm is on the battlefield and the spell is a Zombie creature. This means:

- The player has **no choice** -- the cost is always {0}. The oracle says "you may," meaning the player should be able to choose between paying the normal mana cost or paying {0}. In practice this rarely matters (why would you pay more?), but it could matter with effects like Trinisphere, cost-increase effects, or Mana Drain scenarios where a player wants to pay mana intentionally.
- The implementation returns `ManaCost::free()` **before** any additional-cost or cost-increase logic has a chance to run on the base cost. Per ruling #1 and rule 601.2e, the total cost is the alternative cost ({0}) **plus** all additional costs and cost increases, **minus** all cost reductions. The current code returns early, bypassing later cost modification steps.

### ISSUE 2 (Structural / Low): Behavior is hardcoded in engine, not declared on the card

The card file declares `continuous_effects: vec![]`. The behavior is a special-case block in `effective_spell_cost()` that checks for the card by name string `"Rooftop Storm"`. This is:

- **Fragile** -- if the card name string changes or a variant card is printed, the check breaks.
- **Non-discoverable** -- reading the card file gives no indication of what it does. The doc comment claims `ReduceCost` is used, which is false.
- **Inconsistent** -- other cost-modifying cards (like Heartless Summoning) use the `ContinuousEffect::ReduceCost` system properly through `continuous_effects`.

The type system already has `SpellFilter::CreatureWithSubtype(String)` which could be used, but `ReduceCost` is a cost reduction, not an alternative cost. A proper fix would require adding an `AlternativeCost` variant to `ContinuousEffect`.

### ISSUE 3 (Correctness / Low): Misleading doc comment

The doc comment on the struct says:

> "Uses ReduceCost with a high reduction value for Zombie creature spells."

The implementation does **not** use `ReduceCost`. It uses a hardcoded name check in the engine that returns `ManaCost::free()`.

### ISSUE 4 (Coverage Gap / Low): No test for non-creature Zombie spells

Oracle text specifies "Zombie **creature** spells." There is no test verifying that non-creature Zombie spells (e.g., a Tribal - Zombie spell like Nameless Inversion, if it were in the card registry) are NOT made free. The engine logic does check `is_creature`, so this is likely correct, but test coverage is missing.

### ISSUE 5 (Coverage Gap / Low): No test for "may" / optionality

There is no test verifying that the player can choose to pay the full cost instead of {0}. This ties into Issue 1.

## Summary

| #  | Severity | Category       | Description                                              |
|----|----------|----------------|----------------------------------------------------------|
| 1  | Medium   | Semantic       | Alternative cost modeled as unconditional; no player choice; bypasses additional costs/increases |
| 2  | Low      | Structural     | Hardcoded engine special-case instead of card-declared effect |
| 3  | Low      | Documentation  | Doc comment claims ReduceCost but that is not what happens |
| 4  | Low      | Test Coverage  | No test for non-creature Zombie spells                   |
| 5  | Low      | Test Coverage  | No test for optionality of the alternative cost          |

## Verdict: FAIL (1 medium issue, 4 low issues)

The most significant problem is Issue 1: the alternative cost is modeled as an unconditional replacement that short-circuits before additional costs or cost increases can be applied. While this works for the simple case, it would produce incorrect results with commander tax, Trinisphere, or mandatory additional costs on Zombie creature spells (e.g., Makeshift Mauler's exile requirement is a different system, but mana-based additional costs would be skipped).

Sources:
- [Rooftop Storm Scryfall](https://scryfall.com/card/isd/71/rooftop-storm)
- [Rooftop Storm does not apply to non-creature spells - Magic Rules Tips](https://blogs.magicjudges.org/rulestips/2011/09/rooftop-storm-does-not-apply-to-non-creature-spells/)
- [MTG Salvation Rooftop Storm rulings](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/784301-rooftop-storm)
- [Commander tax interaction - TappedOut](https://tappedout.net/mtg-questions/reducing-commander-costs-rooftop-storm/)

---

# Re-Audit: Rooftop Storm

**Date:** 2026-04-02
**Auditor:** Claude (Opus 4.6)
**Reason:** Re-audit after fixes were applied to address previous audit findings.

## Official Oracle Text (Scryfall, cached 2026-04-01)

- **Name:** Rooftop Storm
- **Cost:** {5}{U}
- **Type:** Enchantment
- **Oracle Text:** "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast."

### Official Rulings
1. [2011-09-22] You must still pay any mandatory additional costs, such as exiling a creature card from your graveyard for Makeshift Mauler.
2. [2011-09-22] The mana cost and mana value of the spell are unchanged. Rooftop Storm only changes what you pay.

## Card Data Review

| Field            | Expected      | Implemented   | Status |
|------------------|---------------|---------------|--------|
| Name             | Rooftop Storm | Rooftop Storm | OK     |
| Mana Cost        | {5}{U}        | Generic(5), Colored(Blue) | OK |
| Card Types       | Enchantment   | Enchantment   | OK     |
| Supertypes       | (none)        | (none)        | OK     |
| Subtypes         | (none)        | (none)        | OK     |
| P/T              | N/A           | None/None     | OK     |
| Oracle Text      | "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast." | "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast." | OK |
| Keywords         | (none)        | (none)        | OK     |
| continuous_effects | (none used) | empty `vec![]`| OK (behavior in engine) |

## Previous Issues -- Status After Fixes

### ISSUE 1 (was Medium): Alternative cost / player choice / additional costs

**Previous finding:** No player choice; unconditional free cast; bypasses additional costs.

**Current status: PARTIALLY FIXED.**

The alternative cost mechanic is now properly modeled with an `alternative_cost` field on `CastSpell` actions. The action generation code at `engine.rs:618-656` correctly:
- Generates both normal-cost and alternative-cost (`ManaCost::free()`) actions when the player can afford the normal cost (player choice preserved).
- Replaces all actions with alternative-cost versions when the player cannot afford the normal cost.
- The `alternative_cost` field at resolution time (line 1425) only replaces the mana payment; additional costs (sacrifice at line 1471, exile at lines following) are still enforced separately. This aligns with ruling #1.

**Remaining problem:** The `effective_spell_cost` function (`engine.rs:101-109`) still contains a Rooftop Storm special case that returns `ManaCost::free()` unconditionally for Zombie creature spells:

```rust
// engine.rs:101-109
if is_creature && subtypes.iter().any(|s| s == "Zombie") {
    let has_rooftop_storm = state.objects.values().any(|o| {
        o.zone == Zone::Battlefield && o.controller == caster && o.name == "Rooftop Storm"
    });
    if has_rooftop_storm {
        return ManaCost::free();
    }
}
```

This means even the "normal cost" path computes to {0}. When the player is offered a choice between "normal" and "alternative cost {0}", both options result in paying {0}. The player is never actually able to pay the full mana cost. In practice this rarely matters (paying more is almost never desired), but it is technically incorrect per the oracle text's "you **may**" wording and could matter with cost-increase effects or Trinisphere.

Additionally, the `rooftop_storm_applies` check in `legal_actions` (line 522) and the `effective_spell_cost` Rooftop Storm block (line 101) are now redundant with each other and with the `alternative_cost` action generation at lines 618-656. The `effective_spell_cost` block should be removed so that the normal-cost path computes the true normal cost.

### ISSUE 2 (was Low): Hardcoded engine special-case

**Current status: UNCHANGED.** The card file still declares `continuous_effects: vec![]` and the behavior is implemented via name-string checks (`o.name == "Rooftop Storm"`) in three locations in the engine:
1. `rooftop_storm_applies()` function (line 41)
2. `effective_spell_cost()` special case (line 101)
3. Action generation block (line 622)

A dedicated `rooftop_storm_applies()` helper function was added, which is an improvement in structure, but the fundamental approach is still hardcoded by card name in the engine rather than declared on the card.

### ISSUE 3 (was Low): Misleading doc comment

**Current status: UNCHANGED.** The doc comment on `RooftopStorm` (rooftop_storm.rs:7-10) still reads:

> "Implementation: Uses ReduceCost with a high reduction value for Zombie creature spells."

The implementation does **not** use `ReduceCost`. It uses name-based checks in the engine and an `alternative_cost` field on `CastSpell`. The doc comment is factually wrong.

### ISSUE 4 (was Low): No test for non-creature Zombie spells

**Current status: UNCHANGED.** No new test was added. The engine correctly checks `CardType::Creature` in all three Rooftop Storm code paths, so this is likely correct but untested.

### ISSUE 5 (was Low): No test for optionality

**Current status: PARTIALLY ADDRESSED.** The action generation code now creates both normal and alternative cost actions when the player can pay, which is the right approach. However, as noted in Issue 1 above, the `effective_spell_cost` block makes both paths compute to {0}, so a test for meaningful optionality would currently fail. No new test was added.

## New Issues Found

### ISSUE 6 (Low): Redundant Rooftop Storm logic in `effective_spell_cost`

The `effective_spell_cost` function at lines 101-109 still returns `ManaCost::free()` for Zombie creatures when Rooftop Storm is on the battlefield. This is now redundant with the `alternative_cost` mechanism and actively harmful because it eliminates the normal-cost option. The block should be removed so that:
- `effective_spell_cost` returns the true effective cost (with other reductions applied but NOT Rooftop Storm's alternative).
- The alternative cost path via `alternative_cost: Some(ManaCost::free())` provides the free option.
- The player genuinely has a choice between the two.

## Tests

Two existing tests, both passing:
1. `rooftop_storm_makes_zombies_free` -- Walking Corpse castable with no mana when Rooftop Storm is on battlefield. PASS.
2. `rooftop_storm_no_free_non_zombies` -- Grizzly Bears not made free by Rooftop Storm. PASS.

No tests for: non-creature Zombie spells, optionality of the alternative cost, interaction with additional costs on Zombie creatures.

## Summary

| #  | Severity | Category       | Description                                              | Status vs Previous |
|----|----------|----------------|----------------------------------------------------------|--------------------|
| 1  | Low      | Semantic       | `effective_spell_cost` still unconditionally returns free, eliminating true player choice | Was Medium, downgraded: alternative cost mechanic added but residual code undermines optionality |
| 2  | Low      | Structural     | Hardcoded engine special-case instead of card-declared effect | Unchanged |
| 3  | Low      | Documentation  | Doc comment claims ReduceCost but that is not what happens | Unchanged |
| 4  | Low      | Test Coverage  | No test for non-creature Zombie spells                   | Unchanged |
| 5  | Low      | Test Coverage  | No test for optionality of the alternative cost          | Unchanged |
| 6  | Low      | Redundancy     | `effective_spell_cost` Rooftop Storm block is now redundant and undermines the new alternative-cost mechanism | New |

## Verdict: CONDITIONAL PASS (6 low issues, 0 medium/high)

The core alternative cost mechanic has been properly implemented via the `alternative_cost` field on `CastSpell` actions. The action generation correctly offers both normal and free options, and additional costs are preserved during resolution. The remaining issues are:

- The redundant `effective_spell_cost` block (Issue 6/1) that makes the "normal cost" path also free, undermining the optionality. This is low-severity because paying {0} instead of full cost is almost always the correct choice anyway.
- Structural and documentation debt (Issues 2, 3) that do not affect correctness.
- Missing edge-case test coverage (Issues 4, 5).

The card functions correctly for standard gameplay scenarios. The implementation is a significant improvement over the previous audit's findings.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {5}{U}, Enchantment type, oracle text matches exactly. The alternative cost effect is correctly implemented in the engine (`rooftop_storm_applies()` in engine.rs) rather than in the card file, which is the right pattern for a static ability that modifies casting costs. Engine checks that the spell is a Zombie creature and that the caster controls a Rooftop Storm, then offers an alternative {0} cost CastSpell action. Additional costs are preserved per rulings.
