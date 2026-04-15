---
id: murder_of_crows-02
status: new
card: Murder of Crows
card_file: mtg-engine/src/cards/isd/murder_of_crows.rs
created: 2026-04-15T03:45:47Z
audit_run_id: 2026-04-14-murder_of_crows-audit
audit_model: opus
audit_tokens: 17841
audit_duration: 401
---

## Audit Finding

**Oracle text:**
> you may draw a card. If you do, discard a card.

**Code:**
> `murder_of_crows.rs:65-82` — after `draw_cards(state, controller, 1, registry)`, the code unconditionally checks `hand.len()` and proceeds to discard if the hand is non-empty, without verifying that the draw actually added a card.

**Description:**
The "if you do" clause makes the discard contingent on having actually drawn a card. The `draw_cards` function (engine.rs:4227) returns void and does not indicate whether any cards were drawn. The handler proceeds to the discard step based solely on whether the hand is non-empty. If the library is empty and the player already had cards in hand, the player is forced to discard even though no card was drawn, violating the "if you do" semantics. Per CR 121.1, drawing a card means "putting the top card of their library into their hand" — if the library is empty, no card is drawn and the condition is not met. The fix requires either tracking hand size before/after the draw call, or modifying `draw_cards` to return the number of cards actually drawn.

**Engine path:**
- mtg-engine/src/cards/isd/murder_of_crows.rs:65-82
- mtg-engine/src/engine.rs:4227 (draw_cards returns void)

**Required check:** Step 6 ("if you do" / "may" analysis)

**Affected cards:**
- Murder of Crows
- Any other card using "draw a card. If you do, [X]" pattern with `draw_cards`

## Tests

### murder_of_crows_empty_library_no_discard
Source ticket: (new)
Implementation: (not yet written)
Scenario: Place Murder of Crows on the battlefield controlled by P0. Give P0 an empty library and 2 cards in hand. Trigger the ability (another creature dies), player chooses yes. Assert that no discard prompt is presented and hand size remains 2, because the draw failed ("if you do" condition not met).

