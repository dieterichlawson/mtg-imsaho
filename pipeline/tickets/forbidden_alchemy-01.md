---
id: forbidden_alchemy-01
status: new
card: Forbidden Alchemy
card_file: mtg-engine/src/cards/isd/forbidden_alchemy.rs
created: 2026-04-15T03:44:55Z
audit_run_id: 2026-04-14-forbidden_alchemy-audit
audit_model: opus
audit_tokens: 16178
audit_duration: 349
---

## Audit Finding

**Oracle text:**
> Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.

**Code:**
> `state.log(LogLevel::Event, format!("Forbidden Alchemy revealed: {}", names.join(", ")));` (forbidden_alchemy.rs:53)
>
> `ResolutionChoiceKind::ChooseFromRevealed { ... }` (forbidden_alchemy.rs:57)
>
> In view.rs:246, the `ChooseFromRevealed` variant populates `revealed_names` in GameView for ALL players:
> `ResolutionChoiceKind::ChooseFromRevealed { revealed, .. } => revealed.clone(),`
>
> In view.rs:279-282, Event-level log messages are included in `display_log` visible to all players:
> `display_log: state.game_log.iter().filter(|e| e.level >= crate::state::LogLevel::Info).map(|e| e.message.clone()).collect(),`

**Description:**
The oracle text says "Look at" the top four cards, not "Reveal." Per CR 701.16a, "look at" means only the controller sees the cards — the opponent should not learn their identities. The implementation leaks all four card names to the opponent through two paths: (1) the Event-level log message at forbidden_alchemy.rs:53 says "Forbidden Alchemy revealed: [card1], [card2], [card3], [card4]", which persists in the game log visible to all players via display_log in GameView; and (2) the engine's `ChooseFromRevealed` resolution kind populates the `revealed_names` map in GameView (view.rs:246, 286) for all players, not just the controller. Since three of the four cards go to the graveyard (a public zone), the opponent can cross-reference the logged/revealed names against the graveyard to deduce which card went to the controller's hand. This violates the privacy guarantee of "look at" and gives the opponent information they should not have.

**Engine path:**
- forbidden_alchemy.rs:53 (Event-level log with all card names)
- forbidden_alchemy.rs:54-62 (uses ChooseFromRevealed)
- view.rs:246 (ChooseFromRevealed populates revealed_names for all players)
- view.rs:279-282 (Event-level log included in display_log for all players)
- view.rs:286 (revealed_names included in GameView)

**Required check:** 8j (rulings + oracle text fidelity)

**Affected cards:**
- Forbidden Alchemy
- Any other card using "Look at" with ChooseFromRevealed (engine-wide: the engine has no "private look" mechanism distinct from "reveal")

## Tests

### look_at_not_revealed_to_opponent
Source ticket: (new)
Implementation: (not yet written)
Scenario: Cast Forbidden Alchemy. Build GameView for the opponent (not the controller). Verify that the opponent's GameView does not contain the names of the looked-at cards in revealed_names or display_log. Currently fails because both paths leak the names.

