# Audit: Falkenrath Noble

## Reference (Scryfall)
- **Name:** Falkenrath Noble
- **Cost:** {3}{B}
- **Type:** Creature -- Vampire Noble
- **Oracle:** Flying. Whenever Falkenrath Noble or another creature dies, target player loses 1 life and you gain 1 life.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Vampire, Noble)
- Oracle text: CORRECT (uses "this creature" but Scryfall says "Falkenrath Noble" -- functionally equivalent)
- P/T: CORRECT (2/2)
- Keywords: CORRECT (Flying)
- Self-dies trigger: CORRECT (TriggerKind::SelfDies in on_dies)
- Any creature dies trigger: CORRECT (TriggerKind::AnyCreatureDies in on_any_creature_dies)
- Target player loses 1 life: CORRECT (auto-targets opponent)
- You gain 1 life: CORRECT

## Issues
None found.
