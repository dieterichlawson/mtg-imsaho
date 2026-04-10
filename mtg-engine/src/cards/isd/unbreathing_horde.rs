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
///
/// Counter placement is implemented in on_enter_battlefield so it fires for both
/// the cast path (ETB trigger after on_resolve moves the card) and the reanimation
/// path (ETB trigger fires directly).
pub struct UnbreathingHorde;

impl UnbreathingHorde {
    fn add_zombie_counters(state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let bf_count = Self::count_zombies_on_battlefield(state, controller, object_id, registry);
        let gy_count = Self::count_zombies_in_graveyard(state, controller, registry);
        let total = bf_count + gy_count;
        if total > 0 {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, total);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Unbreathing Horde enters with {} +1/+1 counters ({} battlefield + {} graveyard Zombies)",
                total, bf_count, gy_count));
    }

    fn count_zombies_on_battlefield(state: &GameState, controller: crate::ids::PlayerId, exclude: ObjectId, registry: &CardRegistry) -> u32 {
        state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                && o.controller == controller
                && o.id != exclude
                && (registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Zombie"))
            })
            .count() as u32
    }

    fn count_zombies_in_graveyard(state: &GameState, controller: crate::ids::PlayerId, registry: &CardRegistry) -> u32 {
        state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                registry.card_data(o.card_id)
                    .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
                    .unwrap_or(false)
                    || o.subtypes.iter().any(|s| s == "Zombie")
            })
            .count() as u32
    }
}

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

    fn has_etb_handler(&self) -> bool { true }

    fn on_enter_battlefield(&self, state: &mut GameState, object_id: ObjectId, registry: &CardRegistry) {
        // Skip if counters were already added via on_resolve (cast path).
        let already_has_counters = state.get_object(object_id)
            .map(|o| *o.counters.get(&CounterType::PlusOnePlusOne).unwrap_or(&0) > 0)
            .unwrap_or(false);
        if already_has_counters {
            return;
        }
        Self::add_zombie_counters(state, object_id, registry);
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        // Count Zombies BEFORE entering (the Horde is still on the stack).
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        let bf_count = Self::count_zombies_on_battlefield(state, controller, object_id, registry);
        let gy_count = Self::count_zombies_in_graveyard(state, controller, registry);

        // Move to battlefield.
        state.move_object(object_id, Zone::Battlefield, registry);

        // Add counters after entering.
        let total = bf_count + gy_count;
        if total > 0 {
            state.add_counters(object_id, CounterType::PlusOnePlusOne, total);
        }
        state.log(crate::state::LogLevel::Event,
            format!("Unbreathing Horde enters with {} +1/+1 counters ({} battlefield + {} graveyard Zombies)",
                total, bf_count, gy_count));
    }
}
