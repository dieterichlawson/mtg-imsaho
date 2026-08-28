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

    /// CR 603.2: "whenever a **Vampire you control** deals combat damage to a
    /// player" is a condition on the event, answered as the damage is dealt.
    /// Asking it at resolution instead meant every creature's combat damage —
    /// an opponent's, a non-Vampire's — put a Heir trigger on the stack that
    /// then did nothing.
    ///
    /// CR 608.2g: "you" is the Heir's last known controller. Reading
    /// `o.controller` off the object gave the owner once the Heir had left the
    /// battlefield — the case where a Heir trades with a blocker in the same
    /// combat damage step, and would have compared the attacking Vampire
    /// against the wrong player.
    fn should_trigger_on_damage_to_player(&self, state: &GameState, self_id: ObjectId, source_id: ObjectId, _damaged_player: PlayerId, registry: &CardRegistry) -> bool {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        state.get_object(source_id).is_some_and(|o| o.controller == controller)
            && state.has_subtype(source_id, "Vampire", registry)
    }

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, _self_id: ObjectId, source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // Whether this triggered was settled above. What is left is CR 121.1:
        // the counter goes on the Vampire only if it is still on the
        // battlefield. Trading with a blocker in the same combat damage step
        // costs it the counter; CR 113.7a means the *Heir* trading does not.
        if state.get_object(source_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.add_counters(source_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}
