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
