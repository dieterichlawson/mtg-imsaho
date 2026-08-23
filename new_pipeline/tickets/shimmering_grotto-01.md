---
id: shimmering_grotto-01
status: new
card: Shimmering Grotto
audit_run_id: 2026-04-19-shimmering_grotto-audit
audit_model: sonnet
audit_tokens: 19750
audit_duration: 417
---

## Audit Finding

**Oracle text:**
> {1}, {T}: Add one mana of any color.

**Code:**
> fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
    ...
    vec![
        ActivatedAbilityDef {
            ability_index: 1,
            description: "{1}, {T}: Add {W}".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
            requires_tap: true,
            ...
        },
        // ... four more entries for U/B/R/G
    ]
}

**Description:**
The {1},{T}: Add one mana of any color ability qualifies as a mana ability under CR 605.1a — it is an activated ability that could put mana into a player's pool when it resolves, has no target, and is not a loyalty ability. The implementation exposes it via `activated_abilities()` as five separate `ActivatedAbilityDef` entries (ability_index 1–5, one per color) rather than as `ManaAbilityDef` entries returned from `mana_abilities()`. The `gather_mana_sources` function in engine.rs (line 76) only calls `behavior.mana_abilities()` to build the pool of available sources for auto-tap plans; `activated_abilities()` is never consulted for this purpose. As a result, Shimmering Grotto's color-producing ability is excluded from every `CastSpell` tap plan. The AI and tap-plan optimizer have no awareness that the land can produce colored mana: in a position where Shimmering Grotto is the only source of a needed color (e.g., three Plains + Grotto trying to cast {2}{G}), no `CastSpell` action is generated for that spell even though the cost is theoretically payable. A human player can work around the omission by manually activating the ability as a standalone `ActivateAbility` action before selecting the spell, but this two-step sequence is invisible to the planner. The comment in the card file acknowledges the limitation ('Simplified: the {1},{T} ability adds {G} (arbitrary choice since the engine doesn't have a "choose a color" mechanism for mana abilities)') but the workaround chosen — multiple `ActivatedAbilityDef` entries — does not restore the missing tap-plan integration.

**Engine path:** mtg-engine/src/engine.rs:76

**Required check:** 8c

## Tests

### grotto_color_ability_funds_spell_in_tap_plan
Scenario: Player controls three Plains and an untapped Shimmering Grotto; a {2}{G} spell is in hand. After tapping the Plains (generic) and using Grotto's {1},{T} ability for {G}, the engine should generate a CastSpell action for the green spell — verifying the color-producing ability is integrated into the tap plan rather than requiring a separate manual pre-activation step.

