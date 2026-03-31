# Audit: Endless Ranks of the Dead

## Reference (Scryfall)
- **Name:** Endless Ranks of the Dead
- **Cost:** {2}{B}{B}
- **Type:** Enchantment
- **Oracle:** At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{B}{B})
- Type: CORRECT (Enchantment)
- Oracle text: CORRECT
- Triggered ability: CORRECT (TriggerKind::Upkeep)
- Only triggers on controller's upkeep: CORRECT (checks state.active_player == controller)
- Counts Zombies you control: CORRECT
- X = half rounded down: CORRECT (zombie_count / 2)
- Creates 2/2 black Zombie tokens: CORRECT
- Token subtypes: CORRECT (Zombie)
- P/T: CORRECT (N/A)

## Issues
None found.
