use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Rakish Heir — {2}{R} 2/2 Vampire.
/// Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on that Vampire.
///
/// TODO: The group trigger for OTHER Vampires you control is not yet implemented.
/// Currently only triggers for Rakish Heir itself (it is a Vampire).
/// A full implementation would need an `on_any_creature_combat_damage_to_player` watcher hook
/// so Rakish Heir can observe combat damage events from all Vampires you control.
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
            supertypes: vec![],
            subtypes: vec!["Vampire".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on that Vampire.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::CombatDamageToPlayer,
                    description: "put a +1/+1 counter on this Vampire".into(),
                },
            ],
        }
    }

    fn on_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // Rakish Heir is a Vampire, so it triggers for its own combat damage.
        // TODO: Also trigger for other Vampires you control (needs watcher hook).
        if state.get_object(self_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            state.add_counters(self_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}
