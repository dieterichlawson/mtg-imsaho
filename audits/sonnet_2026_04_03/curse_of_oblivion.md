## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant player
At the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "That player" choice ownership: pass (cursed player makes the choice, not curse controller)
- "Two cards" when <2 available: pass (automatically exiles all available cards per ruling)
- Upkeep timing filter: pass (only triggers during cursed player's upkeep, not controller's)
- Sequential card selection: pass (correctly presents second choice after first exile)
- Enchant player targeting: pass (TargetRequirement::PlayerOnly enforces player targeting)
- Curse attachment: pass (uses resolve_curse helper to attach to target player)
- Trigger creation: pass (TriggerKind::Upkeep declared, StepStarted{Upkeep} creates UpkeepTrigger)
- Trigger resolution: pass (UpkeepTrigger calls on_upkeep method correctly)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Auto-exile when ≤2 cards: `mtg-engine/tests/tier7_cards.rs:299`
- "That player" choice ownership: NOT TESTED
- Upkeep timing (only cursed player's turn): NOT TESTED  
- Sequential choice for >2 cards: NOT TESTED
- Enchant player targeting: NOT TESTED
- No cards in graveyard: NOT TESTED