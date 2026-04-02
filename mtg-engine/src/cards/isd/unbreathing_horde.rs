use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Unbreathing Horde — {2}{B} 0/0 Zombie.
/// This creature enters with a +1/+1 counter on it for each other Zombie you control
/// and each Zombie card in your graveyard.
/// If this creature would be dealt damage, prevent that damage and remove a +1/+1
/// counter from it.
///
/// "Enters with" is a replacement effect — counters must be counted and added as
/// part of entry, before ETB triggers. Per ruling: "If Unbreathing Horde enters
/// from a graveyard, it will count itself."
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
            oracle_text: "This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.\nIf this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::OnSelf },
            ],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));

        // Count Zombie cards in graveyard BEFORE moving to battlefield.
        // Per ruling: if entering from graveyard, Unbreathing Horde counts itself.
        let graveyard_zombies = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Zombie")
            })
            .count() as u32;

        // Count other Zombies on battlefield (before this one enters).
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

        // Move to battlefield.
        state.move_object(object_id, Zone::Battlefield);

        // Add counters as part of entering (replacement effect).
        let total = battlefield_zombies + graveyard_zombies;
        if total > 0 {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, total);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Unbreathing Horde enters with {} +1/+1 counters ({} battlefield + {} graveyard Zombies)",
                total, battlefield_zombies, graveyard_zombies));
    }
}
