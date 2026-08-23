---
id: charmbreaker_devils-01
status: fixed
card: Charmbreaker Devils
audit_run_id: 2026-04-19-charmbreaker_devils-audit
audit_model: sonnet
audit_tokens: 19584
audit_duration: 382
fixed_at: 2026-08-23T17:06:20Z
fix_note: verified fixed on master by inspection: should_trigger_on_spell_cast implemented and consulted at dispatch (triggers.rs:945); regression coverage present
---

## Audit Finding

**Oracle text:**
> Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.

**Code:**
> GameEvent::SpellCast { player: caster, object: spell_id } => {
    // Dispatch SpellCast triggers for ALL spell types (not just instant/sorcery).
    // Individual card handlers can filter by spell type if needed.
    {
        let watchers: Vec<(ObjectId, CardId, PlayerId)> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| (o.id, o.card_id, o.controller))
            .collect();
        for (watcher_id, watcher_card_id, watcher_controller) in watchers {
            if registry.get(watcher_card_id).is_some() {
                let desc = trigger_description(registry, watcher_card_id, &crate::cards::TriggerKind::SpellCast, false);
                if !desc.is_empty() {
                    let trigger = PendingTrigger::SpellCastWatch { ... };
                    if watcher_controller == active_player {
                        ap_triggers.push(trigger);
                    } else {
                        nap_triggers.push(trigger);
                    }
                }
            }
        }
    }
}

**Description:**
The SpellCast trigger dispatch in `collect_triggers` (triggers.rs ~923-950) creates a `SpellCastWatch` trigger for every battlefield permanent that declares a `TriggerKind::SpellCast` ability, for every `GameEvent::SpellCast` event — with no pre-filtering by caster identity or spell type. For Charmbreaker Devils, whose oracle text says 'Whenever **you** cast an **instant or sorcery** spell', this means the trigger is pushed onto the stack (via `process_pending_trigger_pushes`) even when (a) the opponent casts any spell, or (b) the controller casts a non-instant/sorcery spell. The filtering is deferred entirely to `on_spell_cast` in the card file, which returns early in those cases — but the trigger has already appeared on the stack, giving players a spurious priority window to respond. Per CR 603.2, a triggered ability should only go on the stack when its trigger event (including its condition) actually occurs. This differs from the existing step-trigger insight: SpellCast is not a step trigger, but the same unconditioned dispatch pattern applies.

**Engine path:** mtg-engine/src/triggers.rs:923

**Required check:** 8b

**Affected cards:**
- Burning Vengeance

## Tests

### opponent_cast_creature_no_spurious_trigger
Scenario: Opponent casts a creature spell while Charmbreaker Devils is on the battlefield; verify that no Charmbreaker Devils trigger appears on the stack.

### controller_cast_creature_no_spurious_trigger
Scenario: Controller casts a creature spell; verify that no Charmbreaker Devils trigger appears on the stack.

### controller_cast_instant_trigger_fires
Scenario: Controller casts an instant; verify that exactly one Charmbreaker Devils trigger appears on the stack and resolves to grant +4/+0 until end of turn.

