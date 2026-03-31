# Audit: Doom Blade

## Reference (Scryfall)
- **Name:** Doom Blade
- **Cost:** {1}{B}
- **Type:** Instant
- **Oracle:** Destroy target nonblack creature.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{B})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Target requirement: CORRECT (CreatureWithFilter(Nonblack))
- is_valid_target checks nonblack: CORRECT (!o.colors.contains(&Color::Black))
- Destroys target: CORRECT (uses resolve_destroy)
- P/T: CORRECT (N/A)

## Issues
None found.
