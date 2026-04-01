## Audit — 2026-04-01

**Scryfall Oracle text (front)**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Scryfall Oracle text (back)**: At the beginning of your end step, if Homicidal Brute didn't attack this turn, tap Homicidal Brute, then transform it.
**Scryfall type line**: Creature — Human Advisor // Creature — Human Mutant
**Status**: ISSUE

1. **Discard auto-picks creature card** (`mtg-engine/src/cards/civilized_scholar.rs`, lines 110-115): The activated ability auto-selects a creature card to discard. Oracle says "draw a card, then discard a card" — the player should choose which card to discard. Auto-picking a creature biases the transform trigger.
2. **Triggered abilities declaration mismatch** (`mtg-engine/src/cards/civilized_scholar.rs`, line 39): The `Attacks` TriggerKind is declared, but `on_attacks` is only used for internal state tracking (marking that the creature attacked), not as a real triggered ability that goes on the stack. This is a minor structural concern.
