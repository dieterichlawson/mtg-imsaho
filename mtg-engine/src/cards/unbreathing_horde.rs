use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Unbreathing Horde — {2}{B} 0/0 Zombie.
/// Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each
/// other Zombie you control and each Zombie card in your graveyard.
/// If Unbreathing Horde would be dealt damage, prevent that damage and remove a
/// +1/+1 counter from it.
///
/// Damage prevention is handled by `GameState::mark_damage_on_creature` which
/// checks for Unbreathing Horde by name and applies the replacement effect.
pub struct UnbreathingHorde;

impl CardBehavior for UnbreathingHorde {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Unbreathing Horde".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "Unbreathing Horde enters the battlefield with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.\nIf Unbreathing Horde would be dealt damage, prevent that damage and remove a +1/+1 counter from it.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::EntersBattlefield,
                    description: "enters with +1/+1 counters for Zombies".into(),
                },
            ],
        }
    }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = match state.get_object(object_id) {
            Some(o) => o.controller,
            None => return,
        };

        // Count other Zombies on battlefield under our control.
        let battlefield_zombies = state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                && o.controller == controller
                && o.id != object_id
                && (registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Zombie"))
            })
            .count() as u32;

        // Count Zombie cards in graveyard.
        let graveyard_zombies = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Zombie")
            })
            .count() as u32;

        let total = battlefield_zombies + graveyard_zombies;
        if total > 0 {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, total);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Unbreathing Horde enters with {} +1/+1 counters ({} battlefield + {} graveyard Zombies)",
                total, battlefield_zombies, graveyard_zombies));
    }
}
