# Audit: Evil Twin

## Reference (Scryfall)
- **Name:** Evil Twin
- **Cost:** {2}{U}{B}
- **Type:** Creature -- Shapeshifter
- **Oracle:** You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
- **P/T:** 0/0

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U}{B})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Shapeshifter)
- Oracle text: CORRECT
- P/T: CORRECT (0/0)
- Copies a creature on ETB: CORRECT
- Gains destroy ability: CORRECT (activated ability with {U}{B}, tap cost)
- Destroy ability requires tap: CORRECT (requires_tap: true)
- Targets creature with same name: CORRECT (checks target_name == my_name)
- Uses try_destroy: CORRECT (destroy, not sacrifice)

## Issues
None found.
