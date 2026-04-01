## Audit — 2026-04-01

**Scryfall Oracle text**: Orchard Spirit can't be blocked except by creatures with flying or reach.
**Scryfall type line**: Creature — Spirit
**Status**: PASS

- Name: Correct ("Orchard Spirit")
- Cost: {2}{G} - Correct
- Type: Creature — Spirit - Correct
- P/T: 2/2 - Correct
- Block restriction: Can only be blocked by creatures with flying or reach. Implemented via BlockRestriction with CreatureFilter::Or([HasKeyword(Flying), HasKeyword(Reach)]) and OnSelf scope. Correct.
- Tests: tier5_cards.rs has `orchard_spirit_not_blocked_by_ground`, `orchard_spirit_blocked_by_flyer`, and `orchard_spirit_blocked_by_reach`. Good coverage.

No issues found.
