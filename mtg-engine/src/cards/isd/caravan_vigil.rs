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

        // Remove from library order.
        state.get_player_mut(controller).library_order.retain(|&id| id != land_id);

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

            state.move_spell_after_resolve(spell_id, registry);
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

        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Search library for all basic land cards.
        let basic_lands: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&obj_id| {
                state.has_card_type(obj_id, CardType::Land, registry)
                    && state.face_data(obj_id, registry)
                        .is_some_and(|d| d.supertypes.iter().any(|st| matches!(st, Supertype::Basic)))
            })
            .copied()
            .collect();

        // The search is general; morbid — "you may put that card onto the
        // battlefield instead" — is this card's text and is asked AFTER the
        // land is chosen, so the choice routes back here rather than letting
        // the engine finish the search. Previously the multi-land branch went
        // through the engine's finisher, which put the land in hand and never
        // offered morbid at all.
        match basic_lands.len() {
            0 => {
                state.log(LogLevel::Event, "Caravan Vigil: no basic land found in library".into());
                crate::cards::helpers::shuffle_library(state, controller);
                state.move_spell_after_resolve(object_id, registry);
            }
            1 => Self::finish_search(state, object_id, basic_lands[0], controller, registry),
            _ => {
                let options: Vec<Target> = basic_lands.iter().copied().map(Target::Object).collect();
                state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                    player: controller,
                    source: object_id,
                    choice: ResolutionChoiceKind::ChooseTarget {
                        description: "Caravan Vigil: choose a basic land card".into(),
                        options,
                        optional: false,
                        effect: crate::state::PendingEffect::CardEffect {
                            source_id: object_id,
                            key: String::new(),
                        },
                    },
                });
            }
        }
    }

    /// The chosen land — hand it to the same finisher the single-land case
    /// uses, so morbid is offered either way.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(land_id) = target else { return };
        let controller = crate::cards::helpers::controller_of(state, source_id);
        Self::finish_search(state, source_id, *land_id, controller, registry);
    }

    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        let Some(land_id) = state.get_object(self_id).and_then(|o| o.card_state.get("morbid_land").copied()) else {
            state.move_spell_after_resolve(self_id, registry);
            return;
        };
        let controller = state.get_object(self_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
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

        state.move_spell_after_resolve(self_id, registry);
    }
}
