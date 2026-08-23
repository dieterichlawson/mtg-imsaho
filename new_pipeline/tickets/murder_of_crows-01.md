---
id: murder_of_crows-01
status: new
card: Murder of Crows
audit_run_id: 2026-04-19-murder_of_crows-audit
audit_model: sonnet
audit_tokens: 14529
audit_duration: 313
---

## Audit Finding

**Oracle text:**
> you may draw a card. If you do, discard a card.

**Code:**
> draw_cards(state, controller, 1, registry);
        let hand: Vec<_> = state.objects_in_zone(Zone::Hand, controller)
            .iter().map(|o| o.id).collect();
        if hand.len() == 1 {
            state.move_object(hand[0], Zone::Graveyard, registry);
            state.events.push(GameEvent::Discarded { player: controller, object: hand[0] });
            state.log(LogLevel::Event, "Drew and discarded a card".to_string());
        } else if !hand.is_empty() {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseCardFromHand {
                    description: "Murder of Crows: choose a card to discard".into(),
                    player: controller,
                    cards: hand,
                },
            });
        }

**Description:**
The oracle text says "If you do, discard a card" — the discard is conditional on the draw succeeding. However, `draw_cards` returns `()` (void); it does not report whether a card was actually drawn. After calling `draw_cards`, the handler immediately enumerates the current hand and forces a discard if it is non-empty. If the library is empty, `draw_cards` silently fails (no card enters hand), but the player may already have cards in hand from before the trigger. In that case `!hand.is_empty()` is still true and the engine presents a discard prompt — or auto-discards the single card — even though the draw never happened. A player with a non-empty hand but an empty library who chooses "yes" would be forced to discard a card they did not draw, violating the "If you do" conditional.

**Engine path:** mtg-engine/src/cards/isd/murder_of_crows.rs:65

**Required check:** 8j

## Tests

### murder_of_crows_empty_library_no_forced_discard
Scenario: Murder of Crows is on the battlefield, player has 1 card in hand, library is empty; when another creature dies and the player chooses 'yes', no discard should occur because the draw failed.

### murder_of_crows_nonempty_hand_empty_library_no_prompt
Scenario: Murder of Crows is on the battlefield, player has 3 cards in hand, library is empty; when another creature dies and the player chooses 'yes', the ChooseCardFromHand prompt should NOT appear because no card was drawn.

