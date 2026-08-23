---
id: back_from_the_brink-01
status: new
card: Back from the Brink
audit_run_id: 2026-04-19-back_from_the_brink-audit
audit_model: sonnet
audit_tokens: 35564
audit_duration: 870
---

## Audit Finding

**Oracle text:**
> If the exiled creature card has {X} in its mana cost, X is considered to be zero.

**Code:**
> let mana_cost = registry.card_data(creature.card_id)
    .and_then(|d| d.cost.clone())
    .unwrap_or_else(|| ManaCost::new(vec![]));

**Description:**
When a creature card in the graveyard has {X} in its mana cost, `activated_abilities()` builds the `ActivatedAbilityDef` using the raw registry cost without filtering out `ManaSymbol::X`. The engine activation path in `engine.rs` checks `has_x_cost = ab.cost.symbols.iter().any(|s| matches!(s, ManaSymbol::X))`, finds it true, pays only the non-X portion of the cost, then enters the X-funding flow — prompting the player to choose how much additional mana to spend on X. The player can legally fund X with any remaining mana. Per the ruling, X must be treated as zero; no additional mana should be spendable. The fix is to filter `ManaSymbol::X` out of the creature's mana cost when constructing the `ActivatedAbilityDef` cost in `activated_abilities()`.

**Engine path:** mtg-engine/src/cards/isd/back_from_the_brink.rs:61

**Required check:** 8j

## Tests

### x_cost_creature_activation_costs_only_non_x_portion
Scenario: A creature with {X}{G} in its mana cost is in the graveyard; activating Back from the Brink should cost only {G} with no X-funding prompt and no way to pay additional mana for X.

