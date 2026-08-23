---
id: liliana_of_the_veil-01
status: new
card: Liliana of the Veil
audit_run_id: 2026-04-19-liliana_of_the_veil-audit
audit_model: sonnet
audit_tokens: 28126
audit_duration: 882
---

## Audit Finding

**Oracle text:**
> first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at the same time.

**Code:**
> state.move_object(card_id, Zone::Graveyard, registry);
                    state.events.push(crate::events::GameEvent::Discarded {
                        player: first_player,
                        object: card_id,
                    });
                    ...
                    // Chain to next player.
                    Self::chain_next_discard(state, self_id, registry);

**Description:**
The +1 ability discards each player's card immediately as their choice resolves, rather than collecting all choices first and then discarding all cards simultaneously. Two paths both violate the ruling. (1) Auto-discard path (liliana_of_the_veil.rs:136): when the first player has exactly one card in hand, that card is moved to Zone::Graveyard and a Discarded event is pushed before chain_next_discard prompts the second player. (2) Multi-card choice path (engine.rs:3047): when the first player has multiple cards and submits a ResolveChoice, the engine immediately moves the chosen card to Zone::Graveyard (line 3047) and pushes GameEvent::Discarded (line 3048), then calls on_discard_choice which chains to prompt the second player. In both cases, the game loop's collect_triggers call (engine.rs:4770) runs before the awaiting_action prompt for the second player is dispatched (engine.rs:4790). Any trigger watching for discards (e.g. Murder of Crows, graveyard-order effects) therefore sees the first discard before the second player has made their choice. The oracle ruling requires all choices to be locked in before any cards change zones.

**Engine path:** mtg-engine/src/cards/isd/liliana_of_the_veil.rs:132

**Required check:** 8j

**Affected cards:**
- Liliana of the Veil

## Tests

### discard_trigger_fires_between_choices
Scenario: Murder of Crows is in play; both players have multiple cards; after player A chooses and Liliana +1 chains to player B, verify that Murder of Crows' discard trigger is already on the stack while B is still being prompted — demonstrating the discard is not simultaneous.

### auto_discard_visible_before_second_player_chooses
Scenario: Player A has exactly 1 card in hand; player B has multiple cards; activate Liliana +1; verify that A's card is already in the graveyard (and a Discarded event has fired) at the moment B is presented with their ChooseCardFromHand prompt — both discards should instead occur simultaneously.

