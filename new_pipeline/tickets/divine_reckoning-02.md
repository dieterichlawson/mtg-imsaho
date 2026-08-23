---
id: divine_reckoning-02
status: new
card: Divine Reckoning
audit_run_id: 2026-04-19-divine_reckoning-audit
audit_model: sonnet
audit_tokens: 33181
audit_duration: 977
---

## Audit Finding

**Oracle text:**
> A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.

**Code:**
>         // Clean up the spell first.
        state.move_spell_after_resolve(object_id, registry);

        if pending_players.is_empty() {
            ...
        } else {
            // Present choice to the first pending player.
            ...
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: first_player,
                source: object_id,

**Description:**
Divine Reckoning calls move_spell_after_resolve at line 70, before any player choices are presented via awaiting_action. The spell is moved to Zone::Graveyard (or Zone::Exile when cast with flashback) while it is still mid-resolution — no player has chosen which creature to keep and no destruction has occurred. The correct pattern, documented by the stack.rs comment at the resolve_top function (line 166: 'If the card set an awaiting_action, it's mid-resolution. Don't clean up yet — the ResolveChoice handler in submit_action will do that'), is to defer move_spell_after_resolve to inside the PendingEffect handler. Tribute to Hunger demonstrates the correct pattern: the spell stays in Zone::Stack during the awaiting_action phase and is moved to the graveyard at the end of the SacrificeAndGainLife handler (engine.rs:3662). Per CR 608.2, all resolution effects (including player choices and their downstream effects) complete before the spell leaves the stack. The premature move reverses this order: for flashback-cast Divine Reckoning the exile fires before the 'Destroy the rest' effect, and for normal casts the spell enters the graveyard before choices complete. No events are emitted on Stack→Graveyard/Exile transitions in the current engine so there is no currently-observable trigger impact, but the implementation violates the CR-mandated resolution order and would produce incorrect behavior in any engine that tracks spells leaving the stack as a game event.

**Engine path:** mtg-engine/src/cards/isd/divine_reckoning.rs:70

**Required check:** 8i

## Tests

### flashback_divine_reckoning_zone_is_stack_while_player_chooses
Scenario: Cast Divine Reckoning with flashback; while the player is making their creature choice the spell object should still be in Zone::Stack, and should only move to Zone::Exile after all unchosen creatures have been destroyed.

