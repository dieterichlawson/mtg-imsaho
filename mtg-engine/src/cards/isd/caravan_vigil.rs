use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, LogLevel, ResolutionChoiceKind};
use crate::types::{Zone, ManaCost, ManaSymbol, Color, CardType, Supertype};

/// Caravan Vigil — {G} Sorcery.
/// Search your library for a basic land card, reveal it, put it into your hand,
/// then shuffle. Morbid — You may put that card onto the battlefield
/// instead of putting it into your hand if a creature died this turn.
pub struct CaravanVigil;

impl CaravanVigil {
    /// After a basic land has been selected, handle morbid or put into hand.
    fn finish_search(state: &mut GameState, spell_id: ObjectId, land_id: ObjectId, controller: crate::ids::PlayerId, registry: &CardRegistry) {

        let land_name = state.obj_name(land_id);

        // The land stays in the library — listed, and in its place — until it
        // actually leaves it. Every path below moves it, and `move_object`
        // takes it out of the order then.
        if state.creature_died_this_turn {
            // Morbid: "You may put that card onto the battlefield instead."
            if let Some(obj) = state.get_object_mut(spell_id) {
                obj.card_state.insert("morbid_land".into(), land_id);
            }
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: spell_id,
                choice: ResolutionChoiceKind::YesNo {
                    description: format!(
                        "Caravan Vigil (morbid): put {land_name} onto the battlefield? (No = put into hand)"
                    ),
                    source_card: spell_id,
                },
            });
            // Don't move spell yet or shuffle — on_yes_no_choice will handle that.
        } else {
            state.move_object(land_id, Zone::Hand, registry);
            state.log(LogLevel::Event,
                format!("Caravan Vigil: {land_name} put into hand"));

            crate::cards::helpers::shuffle_library(state, controller);

        }
    }
}

impl CardBehavior for CaravanVigil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Caravan Vigil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nMorbid — You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {

        let controller = crate::cards::helpers::controller_of(state, object_id);

        // Search library for all basic land cards.
        let basic_lands: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&obj_id| {
                state.has_card_type(obj_id, CardType::Land, registry)
                    && state.face_data(obj_id, registry)
                        .is_some_and(|d| d.supertypes.iter().any(|st| matches!(st, Supertype::Basic)))
            })
            .copied()
            .collect();

        // Morbid — "you may put that card onto the battlefield instead" — is
        // this card's own text and is asked AFTER the land is chosen, which is
        // why the choice routes back here rather than through the shared
        // `search_library`, whose finisher moves the card straight to a fixed
        // destination and would never offer morbid at all.
        //
        // CR 701.19b: a player searching a hidden zone "isn't required to find
        // some or all of those cards even if they're present in that zone".
        // The search is mandatory; taking a card is not. So the list is offered
        // as an optional choice even when exactly one basic land qualifies —
        // this used to take that land for the player — and declining still
        // shuffles, because the search happened (CR 701.19a).
        if basic_lands.is_empty() {
            // Debug on purpose: a player who searched and came back with
            // nothing need not say whether there was anything to find, and an
            // Event-level line would say it for them.
            state.log(LogLevel::Debug, "Caravan Vigil: no basic land in library".into());
            crate::cards::helpers::shuffle_library(state, controller);
            return;
        }
        let options: Vec<Target> = basic_lands.iter().copied().map(Target::Object).collect();
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Caravan Vigil: choose a basic land card (or none)".into(),
                options,
                optional: true,
                effect: crate::state::PendingEffect::CardEffect {
                    source_id: object_id,
                    key: String::new(),
                },
            },
        });
    }

    /// The chosen land. Declining is handled by `on_declined_choice` — the
    /// search still happened, so the library is still shuffled.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(land_id) = target else { return };
        let controller = crate::cards::helpers::controller_of(state, source_id);
        Self::finish_search(state, source_id, *land_id, controller, registry);
    }

    /// CR 701.19a: the search happened whether or not a card was found, so the
    /// shuffle happens too.
    fn on_declined_choice(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        state.log(LogLevel::Event, "Caravan Vigil: found nothing".into());
        crate::cards::helpers::shuffle_library(state, controller);
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        let Some(land_id) = state.get_object(self_id).and_then(|o| o.card_state.get("morbid_land").copied()) else {
            return;
        };
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let land_name = state.obj_name(land_id);

        if yes {
            state.move_object(land_id, Zone::Battlefield, registry);
            state.log(LogLevel::Event,
                format!("Caravan Vigil (morbid): {land_name} enters the battlefield"));
        } else {
            state.move_object(land_id, Zone::Hand, registry);
            state.log(LogLevel::Event,
                format!("Caravan Vigil: {land_name} put into hand"));
        }

        // Shuffle library.
        crate::cards::helpers::shuffle_library(state, controller);

    }
}
