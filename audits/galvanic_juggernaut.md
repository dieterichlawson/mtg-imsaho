# Audit: Galvanic Juggernaut

## Reference (Scryfall)
- **Name:** Galvanic Juggernaut
- **Cost:** {4}
- **Type:** Artifact Creature -- Juggernaut
- **Oracle:** Galvanic Juggernaut attacks each combat if able. Galvanic Juggernaut doesn't untap during your untap step. Whenever another creature dies, untap Galvanic Juggernaut.
- **P/T:** 5/5

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({4})
- Type: CORRECT (Artifact, Creature)
- Subtypes: CORRECT (Juggernaut)
- Oracle text: CORRECT
- P/T: CORRECT (5/5)
- Attacks each combat if able: CORRECT (ForceAttack, scope: OnSelf)
- Doesn't untap during untap step: CORRECT (PreventUntap, scope: OnSelf)
- Whenever another creature dies, untap: CORRECT (TriggerKind::AnyCreatureDies, on_any_creature_dies sets tapped=false)

## Issues
None found.
