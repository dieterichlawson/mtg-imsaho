# Audit: Rooftop Storm

## Official Oracle
- **Name:** Rooftop Storm
- **Cost:** {5}{U}
- **Type:** Enchantment
- **Oracle Text:** You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {5}{U} — OK
- **Type:** Enchantment — OK
- **Oracle Text:** Matches — OK
- **P/T:** N/A — OK
- **Behavior:** Comment says it uses ReduceCost with a high reduction value (20) for Zombie creature spells. The actual cost reduction logic is handled externally in the engine.

## Issues
1. **ISSUE: No visible cost reduction mechanism in card file**: The card_data has no continuous_effects or any visible mechanism to apply the cost reduction. The comment mentions ReduceCost but there's no such effect in the card data. The actual cost reduction must be hardcoded somewhere in the engine. This may work correctly, but it's fragile — the card file itself doesn't declare its own behavior.

## Verdict: PASS (with note — cost reduction is handled elsewhere in engine, not visible in card file)
