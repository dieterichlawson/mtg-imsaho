---
id: cellar_door-01
status: new
card: Cellar Door
audit_run_id: 2026-04-19-cellar_door-audit
audit_model: sonnet
audit_tokens: 29047
audit_duration: 534
---

## Audit Finding

**Oracle text:**
> {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.

**Code:**
> state.move_object(milled_id, Zone::Graveyard, registry);

            // Check if it was a creature.
            let is_creature = state.get_object(milled_id)
                .is_some_and(|o| {
                    registry.card_data(o.card_id)
                        .map_or(o.power.is_some(), |d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                });

            if is_creature {
                state.create_token_with_subtypes(
                    "Zombie", controller, 2, 2,
                    vec![Color::Black],
                    vec![CardType::Creature],
                    vec![],
                    vec!["Zombie".into()],
                    registry,
                );

**Description:**
After moving the milled card to the graveyard and confirming it is a creature, on_activate_ability creates the Zombie token but never pushes a GameEvent::CreatureCardMilled event to state.events. The engine's centralized mill_cards() function (engine.rs:4319-4325) always emits this event when it mills a creature card, enabling Undead Alchemist's TriggerKind::CreatureCardMilled triggered ability ('Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token') to fire via the collect_triggers dispatcher in triggers.rs:1127-1158. Because Cellar Door mills from the bottom rather than the top, it cannot reuse mill_cards() and must emit the event manually — but does not. The result: when Cellar Door mills a creature card, Undead Alchemist's trigger is never created and the creature is neither exiled nor replaced by an additional Zombie token.

**Engine path:** mtg-engine/src/cards/isd/cellar_door.rs:67

**Affected cards:**
- Mindshrieker
- Heretic's Punishment

## Tests

### cellar_door_triggers_undead_alchemist_on_creature_mill
Scenario: Undead Alchemist is on the battlefield under Player A's control; Player A activates Cellar Door targeting Player B whose bottom library card is a creature; Undead Alchemist's triggered ability should fire, exiling the milled creature and creating a 2/2 black Zombie token — but currently does not because CreatureCardMilled is never emitted.

