# Audit: Harvest Pyre

## Oracle Reference (Scryfall)
- Cost: {1}{R}
- Type: Instant
- Oracle: "As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard.
  Harvest Pyre deals damage to target creature equal to the number of cards exiled this way."

## Implementation: harvest_pyre.rs

## Issues Found

1. **ISSUE: Always exiles ALL graveyard cards instead of player choosing** - Oracle says "exile any number of cards from your graveyard." The implementation exiles all cards (line 44-47, comment on line 43 acknowledges this: "we exile all cards for maximum damage"). The player should choose how many to exile. This matters strategically (e.g., keeping flashback cards in graveyard, keeping cards for Gnaw to the Bone).

2. **ISSUE: Missing damaged_by tracking** - Line 59-62 marks damage on the creature (`obj.damage_marked += count`) and emits NonCombatDamageDealt, but does NOT push to `obj.damaged_by`. This means effects that check what dealt damage to a creature (e.g., for death triggers like Falkenrath Noble tracking) won't know Harvest Pyre was the source.

3. **ISSUE: Additional cost not enforced at cast time** - The additional_cost field is None (line 29). The exile happens during resolution (on_resolve), not as a cost to cast. Per rules, additional costs are paid during casting, which means the cards should be exiled before the spell resolves. If the spell is countered, the cards should still be exiled.

Otherwise correct: cost ({1}{R}), type (Instant), target requirement (Creature), oracle text.

## Verdict: ISSUES FOUND (3 issues)
