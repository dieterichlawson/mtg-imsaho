---
id: mindshrieker-01
status: fixed
card: Mindshrieker
audit_run_id: 2026-04-19-mindshrieker-audit
audit_model: sonnet
audit_tokens: 18802
audit_duration: 418
fixed_sha: 8c9312f
fixed_at: 2026-08-24T00:53:36Z
test_file: mtg-engine/tests/token_is_not_a_card.rs
fix_note: Milling routed through engine::mill_one so CreatureCardMilled is emitted.
---

## Audit Finding

**Oracle text:**
> {2}: Target player mills a card.

**Code:**
> let milled_card_id = {
    let player_state = state.get_player_mut(*player_id);
    if player_state.library_order.is_empty() {
        return;
    }
    player_state.library_order.remove(0)
};
state.move_object(milled_card_id, Zone::Graveyard, registry);

**Description:**
Mindshrieker's `on_activate_ability` (mindshrieker.rs:54-63) mills a card by directly calling `library_order.remove(0)` and `state.move_object(Zone::Graveyard)` without emitting `GameEvent::CreatureCardMilled`. The canonical `mill_cards()` helper in engine.rs (lines 4313-4325) performs the same library removal and move_object call, but also checks whether the milled card is a creature and conditionally pushes `GameEvent::CreatureCardMilled { object, milled_player }` onto `state.events`. When Mindshrieker mills a creature card, this event is never emitted, so any card watching for `TriggerKind::CreatureCardMilled` — specifically Undead Alchemist, which exiles the milled creature and creates a 2/2 Zombie token — will silently fail to trigger. The fix is to replace the inline mill code with a call to `mill_cards(state, *player_id, 1, registry)`, capturing the top card ID before the call (to preserve the mana-value lookup afterward).

**Engine path:** mtg-engine/src/cards/isd/mindshrieker.rs:54-63

**Required check:** 5

**Affected cards:**
- Trepanation Blade

## Tests

### mindshrieker_milled_creature_triggers_undead_alchemist
Scenario: Mindshrieker activates its ability targeting an opponent whose library's top card is a creature while Undead Alchemist is on the battlefield under the controller's control; the creature should be exiled and a 2/2 Zombie token should be created.

