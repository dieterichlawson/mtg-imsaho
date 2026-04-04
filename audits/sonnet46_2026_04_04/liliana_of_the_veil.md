## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: +1: Each player discards a card.
−2: Target player sacrifices a creature.
−6: Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice.
**Type line**: Legendary Planeswalker — Liliana
**Status**: ISSUE

### Code issues

- **+1 ability: Player 1's discard resolves before Player 2 even makes their choice, violating the "all at the same time" ruling.**
  - File: `mtg-engine/src/cards/isd/liliana_of_the_veil.rs` lines 136–144 (auto-discard path) and `mtg-engine/src/engine.rs` lines 2012–2022 (choice path)
  - Oracle text ruling says: `"first the player whose turn it is chooses a card in hand without revealing it, then each other player in turn order does the same. Then all the chosen cards are discarded at the same time."`
  - Code does: In the auto-discard path (single card), `state.move_object(card_id, Zone::Graveyard)` and `state.events.push(GameEvent::Discarded { ... })` execute at lines 136–140 for the first player, and then `chain_next_discard` is called at line 144 to handle the second player — so P1's discard is set up only *after* P0's card has already moved to the graveyard. In the multi-card path, `new_state.move_object(*discard_id, Zone::Graveyard)` and the `Discarded` event fire at engine.rs lines 2012–2016 the moment P0 submits their choice, before `on_discard_choice` → `chain_next_discard` presents any choice to P1. In both paths the first player's card is fully discarded (zone changed, event fired) before the second player makes any selection, contradicting the ruling that all chosen cards are discarded simultaneously.

### Tricky interactions checked

- **Simultaneous discard (ruling: "all the chosen cards are discarded at the same time")**: FAIL — see issue above; cards are discarded one at a time as each player chooses, not all together after all players have chosen.
- **Active player chooses first ("first the player whose turn it is chooses")**: PASS — `state.active_player` is pushed first into `players_to_discard` (lines 80–87), before iterating remaining players.
- **Turn order for subsequent players**: PASS — after the active player, the loop iterates `PlayerId(0..n)`, skipping the active player and lost players, in ascending turn order (lines 89–98).
- **Empty hand players skipped (ruling: "even if some or all players will be unable to discard")**: PASS — players with no cards in hand are omitted from `players_to_discard` and, inside `chain_next_discard`, a player with an empty hand at chain time is logged and skipped (lines 268–273), with ability still resolving.
- **Both hands empty**: PASS — if `players_to_discard` is empty the code returns early after logging (lines 100–104) with no effect and no `awaiting_action`.
- **Single-card auto-discard**: PASS — when `hand.len() == 1` and the sacrifice is mandatory, the engine auto-applies with no choice presented.
- **−2: Target player (not controller) chooses the creature to sacrifice**: PASS — `present_target_choice` is called with `controller = target_player` (line 174), so it is the targeted player who selects which creature to sacrifice.
- **−2: Sacrifice (not destroy)**: PASS — `PendingEffect::SacrificeCreature` is used (lines 179–183); `apply_pending_effect` calls `crate::destruction::sacrifice` (engine.rs line 2424).
- **−2: Mandatory choice (false for optional)**: PASS — `optional: false` at line 184; with a single creature, `present_target_choice` auto-applies (helpers.rs lines 129–133).
- **−2: No creatures — no effect**: PASS — `creatures_controlled_by` returning empty causes `present_target_choice` to return immediately (helpers.rs line 126–128); logged at lines 168–170.
- **−2: "Target player" includes self**: PASS — `TargetRequirement::PlayerOnly` generates actions for all non-lost players including the caster (engine.rs lines 883–897); test `liliana_minus_two_can_target_self` confirms.
- **−6: Liliana's controller divides piles, target player chooses which to sacrifice**: PASS — `DividePermanentsIntoPiles` is presented to `controller` (line 207); after division the engine presents `ChoosePile` to `*target_player` (engine.rs line 2109). Ruling: "you put each permanent the player controls into one of the two piles [controller does this]; That player sacrifices all permanents in the pile of their choice [target player chooses]."
- **−6: Empty pile allowed (ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed")**: PASS — the controller can submit an empty subset for pile 1 (all permanents fall into pile 2), and the engine's for-loop over `chosen_pile` simply doesn't execute on an empty vec (engine.rs lines 2129–2136).
- **−6: All permanents (not just creatures) are subject to pile division**: PASS — `objects_in_zone(Zone::Battlefield, target_player)` returns all objects on the battlefield controlled by the target player, regardless of card type (state.rs lines 600–608).
- **−6: No permanents — no action**: PASS — `permanents.is_empty()` guard at lines 199–202 returns early with a log message.
- **"target player" on −2/−6 respects hexproof**: PASS — `can_target_player` is called inside `valid_targets_for_req` for `PlayerOnly` (engine.rs lines 884–896; helpers path calls `state.player_has_hexproof`).

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:

- Starting loyalty of 3: `tier15_cards.rs:1814` (`liliana_enters_with_loyalty`) — TESTED
- +1 active player chooses first: `tier15_cards.rs:1830` (`liliana_plus_one_each_player_discards_with_choice`) — TESTED
- +1 all chosen cards discarded at the same time (ruling): NOT TESTED — the existing test at line 1867–1868 asserts P0's card is in the graveyard *before* P1 makes any choice, which validates the incorrect sequential behavior rather than the required simultaneous behavior.
- +1 single card auto-discard: `tier15_cards.rs:1893` (`liliana_plus_one_single_card_auto_discards`) — TESTED
- +1 empty hand skipped: `tier15_cards.rs:1918` (`liliana_plus_one_empty_hand_skipped`) — TESTED
- +1 both hands empty: `tier15_cards.rs:1942` (`liliana_plus_one_both_empty_hands`) — TESTED
- −2 target player chooses creature to sacrifice: `tier15_cards.rs:1961` (`liliana_minus_two_target_player_sacrifices_creature`) — TESTED
- −2 single creature auto-sacrifices: `tier15_cards.rs:2000` (`liliana_minus_two_single_creature_auto_sacrifices`) — TESTED
- −2 no creatures, no effect: `tier15_cards.rs:2021` (`liliana_minus_two_no_creatures`) — TESTED
- −2 can target self: `tier15_cards.rs:2040` (`liliana_minus_two_can_target_self`) — TESTED
- −6 controller divides / target player chooses pile: `tier15_cards.rs:2061` (`liliana_minus_six_pile_division_and_choice`) — TESTED
- −6 empty pile allowed: `tier15_cards.rs:2119` (`liliana_minus_six_empty_pile_allowed`) — TESTED
- −6 all permanents in one pile: `tier15_cards.rs:2149` (`liliana_minus_six_all_in_one_pile`) — TESTED
- −6 no permanents: `tier15_cards.rs:2182` (`liliana_minus_six_no_permanents`) — TESTED
- −6 can target self: `tier15_cards.rs:2200` (`liliana_minus_six_can_target_self`) — TESTED
