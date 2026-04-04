## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flash
When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Creature — Human Wizard
**Status**: ISSUE

### Code issues

- **`until_end_of_turn_flashback` is never cleared at end of turn** (`mtg-engine/src/engine.rs:3006–3061`)
  - Oracle text says: `gains flashback until end of turn`
  - Code does: The Cleanup step clears `until_end_of_turn_effects`, `until_end_of_turn_keywords`, `until_end_of_turn_cant_block`, `until_end_of_turn_protection`, and `until_end_of_turn_removed_keywords`, but **`until_end_of_turn_flashback` is absent from this list and is never cleared**. The flashback grant therefore persists across multiple turns indefinitely.

- **Snapcaster Mage incorrectly excludes cards with innate flashback from eligible targets** (`mtg-engine/src/cards/isd/snapcaster_mage.rs:48–53`)
  - Oracle text says: `target instant or sorcery card in your graveyard` (no restriction on whether the card already has flashback)
  - Code does: `.filter(|o| { registry.card_data(o.card_id).map(|d| { (d.card_types.contains(&CardType::Instant) || d.card_types.contains(&CardType::Sorcery)) && d.flashback_cost.is_none() }).unwrap_or(false) })` — the `&& d.flashback_cost.is_none()` clause excludes any instant or sorcery card that already has a printed flashback cost. Per the oracle text and the ruling "If a card has multiple instances of flashback, you may choose any of its flashback costs to pay," there is no such restriction. Targeting a card with innate flashback (e.g., Think Twice, Geistflame) would grant it a second flashback instance at its mana cost, which is a meaningful interaction (e.g., casting Think Twice for {1}{U} instead of the printed flashback {2}{U}).

### Tricky interactions checked

- **"until end of turn" expiry**: FAIL — `until_end_of_turn_flashback` is initialized in `GameState::new` but is never cleared at cleanup. All other until-EOT vecs are cleared in `Step::Cleanup` handling in `engine.rs:3021–3025`; `until_end_of_turn_flashback` is absent from that list. Confirmed no other clear call exists (`flashback.clear()` returns no matches in the entire `src/` tree).
- **ETB trigger dispatch fires correctly**: PASS — `collect_triggers` pushes `PendingTrigger::EnteredBattlefield` for all cards with a registry entry when `GameEvent::EnteredBattlefield` is seen. `resolve_next_trigger` verifies the object is still on the battlefield, then calls `on_enter_battlefield`. Snapcaster Mage is on the battlefield at resolution time, so the handler runs correctly.
- **"target" — mandatory choice presented when multiple eligible cards**: PASS — when `eligible.len() > 1`, the code calls `present_target_choice` with `optional: false`, which presents a `ChooseTarget` `ResolutionChoice` to the player. When `eligible.len() == 1`, it auto-applies (correct for a mandatory targeted ability with exactly one legal target).
- **"target" — requires a legal target to trigger (no target, no effect)**: PASS — `on_enter_battlefield` returns early (`if eligible.is_empty() { return; }`) when there are no eligible cards, effectively doing nothing. The trigger fired but found no valid targets. This is correct.
- **"your graveyard" — owner vs. controller**: PASS — the filter uses `o.owner == controller` where `controller` is Snapcaster Mage's controller. Since graveyards are tracked by ownership, and "your graveyard" refers to the zone belonging to the controller of Snapcaster Mage, this is correct.
- **Flashback cost set to mana cost**: PASS — both the single-target path (direct push in `snapcaster_mage.rs:66–69`) and the multi-target path (via `apply_pending_effect` → `GrantFlashback` in `engine.rs:2615–2626`) retrieve the cost from `registry.card_data(...).and_then(|d| d.cost.clone())`, which is the card's mana cost. This matches "The flashback cost is equal to its mana cost."
- **Flashback spell exiled after resolution**: PASS — the engine sets `cast_with_flashback = true` on any spell cast from the graveyard (`engine.rs:1636–1638`), and `move_spell_after_resolve` checks this flag to send it to exile instead of the graveyard (`state.rs:1132–1141`). Applies to any instant/sorcery cast using the granted flashback.
- **Cards without innate flashback but already granted dynamic flashback excluded from second grant**: This is technically a minor restriction that goes beyond the oracle text (two Snapcasters could legally target the same card), but the practical impact is negligible since a second grant adds no benefit. Not flagged as a separate issue.
- **"may" — the player may cast using flashback (optional)**: PASS — the flashback grant does not force a cast; it only makes the card castable from the graveyard via `legal_actions`. The player freely chooses whether to cast it.

### Test coverage

- **Basic ETB grant (Snapcaster targets Lightning Bolt)**: `mtg-engine/tests/tier14_cards.rs:393` — `snapcaster_mage_grants_flashback` — TESTED
- **"until end of turn" expiry (flashback should be gone next turn)**: NOT TESTED — no test verifies `until_end_of_turn_flashback` is empty after the Cleanup step. This is the failing behavior.
- **Snapcaster can target a card with innate flashback (e.g., Geistflame)**: NOT TESTED
- **Snapcaster with empty graveyard (ETB trigger does nothing)**: NOT TESTED
- **Multiple eligible cards in graveyard — player presented choice**: NOT TESTED
- **Granted flashback spell is exiled (not graveyard) after casting**: NOT TESTED directly for Snapcaster-granted flashback (only tested for innate flashback cards like Geistflame in `flashback.rs`)
- **Flashback cost equals mana cost (not some other cost)**: NOT TESTED
