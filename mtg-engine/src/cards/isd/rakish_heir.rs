use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Rakish Heir — {2}{R} 2/2 Vampire.
/// Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on that Vampire.
pub struct RakishHeir;

impl CardBehavior for RakishHeir {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rakish Heir".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Vampire".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCombatDamageToPlayer,
                    description: "put a +1/+1 counter on that Vampire".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, registry: &CardRegistry) {
        // Whenever a Vampire YOU control deals combat damage to a player.
        // CR 113.7a: the Heir trading with a blocker in the same combat damage
        // step does not counter this — the Vampire still gets its counter.
        // CR 608.2g: "you" is the Heir's last known controller. Reading
        // `o.controller` off the object gave the owner once the Heir had left
        // the battlefield, which is exactly the case the comment above is
        // about — a Heir that traded with a blocker in the same damage step
        // would have compared the attacking Vampire against the wrong player.
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // The Vampire that dealt the damage has to be one you control, and
        // still be there for the counter to go on it.
        let source_is_yours = state.get_object(source_id)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == controller);
        if source_is_yours && state.has_subtype(source_id, "Vampire", registry) {
            // Put a +1/+1 counter on THAT Vampire (the source).
            state.add_counters(source_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}
