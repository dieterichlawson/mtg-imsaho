# Audit: Festerhide Boar

## Reference (Scryfall)
- **Name:** Festerhide Boar
- **Cost:** {3}{G}
- **Type:** Creature -- Boar
- **Oracle:** Trample. Morbid -- Festerhide Boar enters the battlefield with two +1/+1 counters on it if a creature died this turn.
- **P/T:** 3/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Boar)
- Oracle text: CORRECT
- P/T: CORRECT (3/3)
- Keywords: CORRECT (Trample)
- Morbid check: CORRECT (checks creature_died_this_turn)
- Two +1/+1 counters: CORRECT

## Issues
**ISSUE: Morbid is a static/replacement ability, not a triggered ability.** The oracle says "enters the battlefield WITH two +1/+1 counters" -- this is a replacement effect that modifies how the creature enters, not a triggered ability that fires after entering. The implementation uses on_enter_battlefield (ETB trigger) and declares TriggerKind::EntersBattlefield in triggered_abilities. While functionally similar, the triggered_abilities metadata is misleading. The actual on_enter_battlefield hook is fine functionally.
