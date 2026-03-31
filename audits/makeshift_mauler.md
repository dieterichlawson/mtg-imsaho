# Audit: Makeshift Mauler

## Oracle (Official)
- **Name:** Makeshift Mauler
- **Cost:** {3}{U}
- **Type:** Creature — Zombie Horror
- **Oracle:** As an additional cost to cast this spell, exile a creature card from your graveyard.
- **P/T:** 4/5

## Implementation
- Name: "Makeshift Mauler" -- CORRECT
- Cost: {3}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Zombie", "Horror"] -- CORRECT
- P/T: 4/5 -- CORRECT
- Oracle text matches -- CORRECT
- Additional cost: ExileCreaturesFromGraveyard(1) -- CORRECT
- On resolve: exiles a creature card from graveyard, then enters battlefield -- CORRECT

## Issues
None.

## Verdict: PASS
