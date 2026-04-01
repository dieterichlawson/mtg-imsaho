## Audit — 2026-04-01

**Scryfall Oracle text**: When Pitchburn Devils dies, it deals 3 damage to any target.
**Scryfall type line**: Creature — Devil
**Status**: PASS

- Name: Correct ("Pitchburn Devils")
- Cost: {4}{R} - Correct
- Type: Creature — Devil - Correct
- P/T: 3/3 - Correct
- Trigger: SelfDies - Correct
- Effect: Deals 3 damage to any target (creatures + players). Uses helpers::any_targets and presents target choice. Mandatory (not "you may"). Correct.
- Tests: tier3_cards.rs has `pitchburn_devils_deals_3_on_death`, card_mechanics.rs has `pitchburn_devils_choice_with_targets`.

No issues found.
## Audit — 2026-04-01

**Scryfall Oracle text**: When Pitchburn Devils dies, it deals 3 damage to any target.
**Scryfall type line**: Creature — Devil
**Status**: PASS

No issues found. Death trigger correctly presents target choice for "any target" (creatures + players). Uses SelfDies trigger kind.
