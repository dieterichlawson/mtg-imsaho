---
id: liliana_of_the_veil-01
status: closed-duplicate
card: Liliana of the Veil
card_file: mtg-engine/src/cards/isd/liliana_of_the_veil.rs
created: 2026-04-14T20:55:10Z
audit_run_id: 2026-04-14-liliana_of_the_veil-audit
audit_model: opus
audit_tokens: 11904
audit_duration: 290
duplicate_of: mtg-engine/tests/audit_bugs2.rs:528
duplicate_reason: Sequential-vs-simultaneous discard already has a test
---

## Audit Finding

**Oracle text:**
> +1: Each player discards a card.

**Code:**
> In `liliana_of_the_veil.rs:132-143`, when a player has 1 card, it is immediately moved to graveyard via `state.move_object(card_id, Zone::Graveyard, registry)` and the Discarded event is emitted before the next player chooses. Similarly, in `engine.rs:3039-3043`, the `ChooseCardFromHand` handler moves the chosen card to graveyard and emits Discarded before calling `on_discard_choice` to chain to the next player.

**Description:**
The +1 ability discards cards sequentially rather than simultaneously. Per the ruling, all players choose in APNAP order, then all chosen cards are discarded at the same time. The current implementation fully resolves each player's discard (moving to graveyard, emitting the Discarded event, potentially triggering graveyard-enters abilities) before the next player even gets to choose. This means: (1) earlier players' discards can trigger abilities that change game state before later players choose, and (2) a player choosing later can see what earlier players discarded (via graveyard observation), leaking information that should be hidden. This is a known bug with an existing test at `audit_bugs2.rs:528`.

**Engine path:**
- liliana_of_the_veil.rs:132-143 (auto-discard for single card)
- liliana_of_the_veil.rs:146-156 (present choice for multi-card)
- engine.rs:3037-3052 (ChooseCardFromHand handler discards immediately)
- liliana_of_the_veil.rs:223-226 (on_discard_choice chains to next player)

**Required check:** 8j (ruling 2 — simultaneous discard)

**Affected cards:**
- Liliana of the Veil
- Any future card with "each player discards" or similar simultaneous choice effects
