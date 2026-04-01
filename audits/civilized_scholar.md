## Audit — 2026-04-01

**Scryfall Oracle text**: (Front) {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap Civilized Scholar, then transform Civilized Scholar.
(Back — Homicidal Brute) At the beginning of your end step, if Homicidal Brute didn't attack this turn, transform Homicidal Brute.
**Scryfall type line**: (Front) Creature — Human Advisor // (Back) Creature — Human Mutant
**Status**: ISSUE

### Findings

1. **Front face P/T correct**: 0/1 matches Oracle.

2. **Back face P/T correct**: 5/1 matches Oracle.

3. **Discard is auto-chosen, not player-chosen (ISSUE)**: The implementation auto-picks a creature card from hand to discard (line 111-114), preferring creatures to trigger the transform. Oracle text says "then discard a card" — the player should choose which card to discard.

4. **Triggered ability TriggerKind mismatch (ISSUE)**: The triggered_abilities list includes `TriggerKind::Attacks` (for tracking attack state) and `TriggerKind::EndStep`. The Attacks trigger is used internally to mark state, which is a reasonable implementation approach. However, the back face's `triggered_abilities` vec is empty even though the end step trigger logic is on the front face's card data — this works because the same CardBehavior handles both faces.

5. **End step transform-back taps (ISSUE)**: Line 159 sets `obj.tapped = true` when transforming back. The Oracle text for Homicidal Brute does NOT say to tap it when transforming back. It just says "transform Homicidal Brute." The tapping is incorrect.

6. **Cloistered Youth P/T stored as (0,1) but oracle for front is 0/1**: Correct.

7. **Subtypes correct**: Human Advisor (front), Human Mutant (back).

8. **Tests**: No dedicated tests found.
