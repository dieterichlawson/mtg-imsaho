# Audit: Elder of Laurels

## Reference (Scryfall)
- **Name:** Elder of Laurels
- **Cost:** {2}{G}
- **Type:** Creature -- Human Advisor
- **Oracle:** {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
- **P/T:** 2/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Advisor)
- Oracle text: CORRECT
- P/T: CORRECT (2/3)
- Activated ability cost: CORRECT ({3}{G})
- requires_tap: CORRECT (false)
- Target creature: CORRECT (TargetRequirement::Creature)
- +X/+X where X = creatures you control: CORRECT (counts creatures on battlefield under controller)

## Issues
None found.

---

# Audit: Elder of Laurels (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)
> Name: Elder of Laurels
> Mana Cost: {2}{G}
> Type Line: Creature — Human Advisor
> P/T: 2/3
> Oracle Text: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.

## Card Data
- **Name**: "Elder of Laurels" — correct
- **Mana cost**: {2}{G} — correct
- **Card types**: Creature — correct
- **Subtypes**: Human, Advisor — correct
- **Power/Toughness**: 2/3 — correct
- **Oracle text string**: matches verbatim

## Activated Ability
- **Cost**: {3}{G} — correct
- **requires_tap**: false — correct (oracle has no tap symbol)
- **sacrifice_cost**: None — correct
- **target_requirement**: `TargetRequirement::Creature` — correct ("Target creature")
- **once_per_turn**: false — correct (no restriction in oracle)
- **sorcery_speed_only**: false — correct
- **Zone restriction**: battlefield only — correct

## Effect (on_activate_ability)
- Counts creatures the controller controls on the battlefield at resolution time — matches ruling: "The number of creatures you control is counted as the ability resolves."
- Applies +X/+X as `until_end_of_turn_effects` with fixed power_mod and toughness_mod — matches "until end of turn" and ruling: "Once the ability has resolved, the bonus won't change if the number of creatures you control changes later in the turn."
- Creature detection uses `o.power.is_some()` as a proxy — acceptable heuristic.

## Tests
- `elder_of_laurels_card_data` — verifies P/T and subtypes. PASS.
- `elder_of_laurels_pumps_by_creature_count` — 3 creatures on battlefield, target 2/2 gets +3/+3 becoming 5/5. PASS.

## Verdict
**PASS** — No issues found. Card data, ability cost, targeting, creature count, and +X/+X effect all match oracle text and rulings.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
**Type line**: Creature — Human Advisor
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:54

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
**Type line**: Creature — Human Advisor
**Status**: PASS

### Code issues
None. All card data fields (name, cost, type, subtypes, P/T, oracle text) match oracle exactly. Activated ability definition (cost {3}{G}, no tap, no sacrifice, TargetRequirement::Creature, not once-per-turn, instant-speed, battlefield-only) is correct. The effect counts creatures at resolution via `objects_in_zone` + `power.is_some()` filter and stores the result as a fixed UntilEndOfTurnEffect, matching both rulings.

### Tricky interactions checked (min 3)
1. **Creature count at resolution, not activation**: The count is computed inside `on_activate_ability` (resolution time), not during ability generation. Matches ruling: "The number of creatures you control is counted as the ability resolves."
2. **Bonus locked in after resolution**: The X value is stored as fixed `power_mod`/`toughness_mod` i32 values in `UntilEndOfTurnEffect`. If creatures enter or leave after resolution, the bonus does not change. Matches ruling: "Once the ability has resolved, the bonus won't change if the number of creatures you control changes later in the turn."
3. **Can target any creature (including itself and opponents' creatures)**: `TargetRequirement::Creature` does not filter by controller, so the ability can target any creature on the battlefield, including Elder of Laurels itself or an opponent's creature. This matches the oracle text which says "Target creature" without restriction.
4. **Multiple activations stack**: Each activation creates a separate `UntilEndOfTurnEffect` entry, so activating the ability twice correctly applies two independent bonuses.

### Test coverage
- `elder_of_laurels_card_data`: Verifies P/T (2/3), mana value (3), subtypes (Human, Advisor).
- `elder_of_laurels_pumps_by_creature_count`: Sets up 3 creatures, activates ability on a 2/2, asserts it becomes 5/5 (both power and toughness).
