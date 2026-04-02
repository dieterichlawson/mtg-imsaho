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
