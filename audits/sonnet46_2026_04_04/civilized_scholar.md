## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
**Back face oracle text**: At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
**Type line**: Creature — Human Advisor // Creature — Human Mutant
**P/T**: 0/1 // 5/1
**Status**: ISSUE

### Code issues

- **Stale `attacked_this_turn` flag causes Homicidal Brute to skip transform-back** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 166–191)
  - Oracle text says: `"At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it."` — "this turn" means the current controller's turn; the condition must reflect only whether the creature attacked in the turn currently ending.
  - Code does: `on_end_step` guards with `if !is_transformed || state.active_player != controller { return; }` and places the `card_state.remove("attacked_this_turn")` clear **after** that guard. When the end step fires while the creature is in front-face (Civilized Scholar) form, the function returns early without clearing the flag. Sequence that triggers the bug: (1) Turn N — Civilized Scholar declares as attacker, `on_attacks` inserts `"attacked_this_turn"` into `card_state`; (2) Turn N end step — `is_transformed=false`, `on_end_step` returns before the clear; (3) Turn N+1 — player activates `{T}`, discards a creature, Civilized Scholar transforms into Homicidal Brute; (4) Turn N+1 end step — `is_transformed=true`, `on_end_step` reads stale flag, sees `attacked=true`, incorrectly skips the tap-and-transform-back, then clears the flag. Homicidal Brute should have transformed back (it did not attack in Turn N+1) but does not.

- **EndStep trigger registered on front face, causing spurious stack entry when not transformed** (`mtg-engine/src/cards/isd/civilized_scholar.rs`, lines 38–48)
  - Oracle text says: Civilized Scholar's (front face) oracle text is `"{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it."` — no end-step triggered ability on the front face. The end-step trigger belongs exclusively to Homicidal Brute (back face).
  - Code does: `card_data().triggered_abilities` lists `TriggerKind::EndStep` on the front face (`triggered_abilities: vec![TriggeredAbilityDef { kind: TriggerKind::Attacks, ... }, TriggeredAbilityDef { kind: TriggerKind::EndStep, description: "transform back if didn't attack".into() }]`). The back face `back_face_data().triggered_abilities` is `vec![]`. In `triggers.rs`, `trigger_description` checks front-face triggers first (line 314) and returns a non-empty description for `EndStep` even when `is_transformed=false`. Because the description is non-empty (line 612 check), a `PendingTrigger::EndStepTrigger` is queued every end step regardless of transformation state. The `on_end_step` handler correctly guards with `if !is_transformed { return; }` so there is no game-state effect, but the trigger still appears on the stack when the card is in front-face form, which is incorrect per oracle text.

### Tricky interactions checked

- **"This turn" scoping across faces**: If a creature attacks as Civilized Scholar (front face) and later transforms into Homicidal Brute within the same turn via the activated ability — this cannot happen in practice because attacking taps the creature and the ability also requires tapping (`requires_tap: true`), so there is no mechanism to attack and use the ability in the same turn. Across turns, the stale-flag bug (Issue 1 above) applies: FAIL.
- **Ruling: attacked-as-either-face counts**: The ruling says "the creature attacked that turn, even if it had its other face up at the time." The Attacks trigger (front-face `TriggerKind::Attacks`) fires for both faces because `trigger_description` is called with `is_transformed=false` in the `AttackersDeclared` handler (triggers.rs line 684), and the front-face trigger is always found. `on_attacks` sets `attacked_this_turn` regardless of face. Correct for this ruling — but only when the flag is not stale (see stale-flag bug above): CONDITIONAL PASS (correct in isolation, broken by stale-flag bug across turns).
- **"May" / "you may" optionality**: The activated ability has no optional component. The draw is mandatory, the discard is mandatory. With exactly one card in hand after drawing, the code auto-discards (lines 107–130). With multiple cards, it presents a `ChooseCardFromHand` choice. The player cannot decline to discard. Per oracle text: "Draw a card, then discard a card" — both are mandatory. PASS.
- **Untap-then-transform atomicity (ruling: no priority in between)**: The ruling states "You don't have priority between untapping Civilized Scholar and transforming it." Both untap and transform happen synchronously within the same `on_activate_ability` / `on_discard_choice` call; no `awaiting_action` is set between them. PASS.
- **Discard check: `is_creature` via `o.power.is_some()`**: Both the single-card path (line 113) and the multi-card path (`on_discard_choice` line 147) check `state.get_object(id).map(|o| o.power.is_some())`. After moving to graveyard, `get_object` returns the object (not zone-filtered) with its original `power` field intact (move_object does not clear non-battlefield fields). Standard creatures have `power = Some(i32)`; non-creatures have `power = None` (set at `create_object` from `card_data.power`). PASS.
- **Homicidal Brute end-step trigger fires only on controller's turn**: `on_end_step` checks `state.active_player != controller` and returns early if it's not the controller's end step. PASS.
- **End-step trigger: intervening-if clause ("if this creature didn't attack this turn")**: The check at `on_end_step` (line 175–177) reads the flag at resolution time. The same condition is not re-checked at trigger collection time (no intervening-if at collection), but this is acceptable for MTG rules since "at the beginning of your end step, if..." is an intervening-if that should be checked both at trigger event and at resolution. The code only checks at resolution (during `on_end_step`). Functionally correct for normal cases, but combined with the stale-flag bug, resolution-time check produces wrong result: CONDITIONAL PASS.
- **EndStep trigger fires during opponent's turn for Homicidal Brute**: The `StepStarted { EndStep }` handler collects triggers for ALL permanents on the battlefield. For an opponent's end step, a Homicidal Brute controlled by the non-active player would also have its trigger collected. `on_end_step` returns early via `state.active_player != controller`. PASS.
- **Transform-back when can't attack (ruling: "You'll tap and transform Homicidal Brute even if it couldn't attack")**: `on_end_step` has no guard for whether the creature could legally attack; it only checks `attacked_this_turn`. If Homicidal Brute was tapped, summoning-sick, or pacified, it would still transform back (the flag would be absent). PASS.
- **`attacked_this_turn` cleared on zone change**: `move_object` does not clear `card_state`. If the creature leaves the battlefield and re-enters, it is a new object with fresh `card_state`. Correct. PASS.
- **`should_transform` returns false**: The standard werewolf-transform upkeep check calls `should_transform`; returning false correctly prevents Civilized Scholar from being caught by the werewolf transform logic. PASS.
- **`dynamic_pt` returns (5,1) for Homicidal Brute**: Returns `Some((5, 1))` when `is_transformed=true`, `None` otherwise (uses base 0/1). Matches oracle P/T. PASS.
- **Single-card-in-hand auto-discard path**: When only one card is in hand, `on_activate_ability` auto-discards without presenting a choice (lines 110–130). The card is moved to graveyard, a `Discarded` event is emitted, and the creature/non-creature check is applied immediately. PASS.

### Test coverage

- Discard creature → transforms: `tier15_cards.rs:2760` — TESTED (`civilized_scholar_discard_creature_transforms`)
- Discard non-creature → no transform, stays tapped: `tier15_cards.rs:2801` — TESTED (`civilized_scholar_discard_noncreature_no_transform`)
- Homicidal Brute end-step transform-back (didn't attack): NOT TESTED
- Homicidal Brute end-step no transform-back (did attack): NOT TESTED
- Stale `attacked_this_turn` flag across turns (attack as front face, transform next turn, end step): NOT TESTED
- Single-card-in-hand auto-discard path: NOT TESTED (both tests have multiple cards in hand)
- EndStep trigger fires spuriously in front-face form: NOT TESTED
- Attacked-as-either-face counts correctly: NOT TESTED
- End-step only fires on controller's turn: NOT TESTED
- Ruling: "can't attack but still taps/transforms": NOT TESTED
