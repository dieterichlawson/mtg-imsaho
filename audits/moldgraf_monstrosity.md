## Audit — 2026-04-01

**Scryfall Oracle text**: Trample\nWhen Moldgraf Monstrosity dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Scryfall type line**: Creature — Insect
**Status**: PASS

- Name: Correct ("Moldgraf Monstrosity")
- Cost: {4}{G}{G}{G} - Correct
- Type: Creature — Insect - Correct
- P/T: 8/8 - Correct
- Keywords: Trample - Correct
- Triggered ability (SelfDies): Correctly exiles itself, then returns up to two random creature cards from graveyard to battlefield. Uses shuffle + take(2) for randomness.
- Correctly filters out the Monstrosity itself (now in exile) and non-creature cards (checks power.is_some()).
- Correctly sets controller on returned creatures.
- Tests: tier15_cards.rs has test `moldgraf_monstrosity_returns_creatures_on_death`.

No issues found.
