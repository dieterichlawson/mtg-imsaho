## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another Human you control enters, put a +1/+1 counter on this creature.
**Type line**: Creature — Human Soldier
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Another" condition (Champion doesn't trigger on itself): PASS - Engine filter `o.id != *object` in triggers.rs:369 correctly excludes the entering creature from watching its own ETB event
- "You control" targeting (only triggers on own Humans): PASS - Code checks `entered_controller != controller` and returns early if they don't match
- Subtype checking for tokens vs cards: PASS - Code checks both `registry.card_data()` and `obj.subtypes` to handle regular cards and tokens correctly
- Source removal before trigger resolves: PASS - Code checks Champion is still on battlefield (`o.zone == Zone::Battlefield`) before adding counters
- Multiple simultaneous Human entries (Gather the Townsfolk scenario): PASS - Trigger dispatch processes each ETB event separately, triggering once per Human
- Human type changes after ETB: PASS - Trigger checks Human status at ETB time, not continuously

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Champion gets counter when another Human enters: `tier6_cards.rs:87` 
- Non-Human creatures don't trigger Champion: `tier6_cards.rs:108`
- Opponent's Humans don't trigger Champion: `tier6_cards.rs:128`
- Multiple Champions triggering simultaneously: NOT TESTED
- Champion being destroyed before trigger resolves: NOT TESTED  
- Multiple Humans entering simultaneously: NOT TESTED
- Champion not triggering on itself: NOT TESTED (but verified correct via engine code)