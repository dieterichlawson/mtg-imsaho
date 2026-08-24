use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ContinuousEffect, EffectScope, Zone, CounterType};

/// Unbreathing Horde — {2}{B} 0/0 Zombie.
/// This creature enters with a +1/+1 counter on it for each other Zombie you control
/// and each Zombie card in your graveyard.
/// If this creature would be dealt damage, prevent that damage and remove a +1/+1
/// counter from it.
///
/// "Enters with" is a replacement effect (CR 614.1c). Per Scryfall ruling:
/// "If Unbreathing Horde enters from a graveyard, it will count itself."
/// The `entering_with_counters` callback is called BEFORE the zone change,
/// so graveyard counts naturally include the Horde when entering from GY.
pub struct UnbreathingHorde;

impl UnbreathingHorde {
    fn is_zombie(state: &GameState, id: ObjectId, registry: &CardRegistry) -> bool {
        state.has_subtype(id, "Zombie", registry)
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
            subtypes: vec!["Zombie".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.\nIf this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.".into(),
            continuous_effects: vec![
                ContinuousEffect::PreventDamageRemoveCounter { scope: EffectScope::OnSelf },
            ],
            ..Default::default()
        }
    }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        crate::cards::helpers::enters_with_counters(self_id, event, || {
        let controller = state.get_object(self_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Count other Zombies on the battlefield.
        let bf_count = u32::try_from(state.objects.values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                && o.controller == controller
                && o.id != self_id
                && Self::is_zombie(state, o.id, registry)
            })
            .count()).unwrap_or(u32::MAX);

        // Count Zombie cards in graveyard. Because this callback is called
        // BEFORE the zone change, the Horde is still in its original zone.
        // If entering from graveyard, the Horde naturally appears in this
        // count (per the Scryfall ruling: "it will count itself").
        let gy_count = u32::try_from(state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            // "each Zombie CARD" — CR 109.1: a token is not a card. The
            // battlefield count above says just "Zombie", so it correctly
            // includes tokens; this one must not.
            .filter(|o| o.id != self_id && state.is_card(o.id) && Self::is_zombie(state, o.id, registry))
            .count()).unwrap_or(u32::MAX);

        // Per ruling: count self when entering from graveyard.
        let self_in_gy = state.get_object(self_id)
            .is_some_and(|o| o.zone == Zone::Graveyard);
        let self_count = u32::from(self_in_gy);

        let total = bf_count + gy_count + self_count;
        if total > 0 {
            vec![(CounterType::PlusOnePlusOne, total)]
        } else {
            vec![]
        }
        })
    }
}
