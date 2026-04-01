use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Creeping Renaissance — {3}{G}{G} Sorcery.
/// Choose a permanent type. Return all cards of the chosen type from your graveyard
/// to your hand. Flashback {5}{G}{G}.
pub struct CreepingRenaissance;

/// Encode permanent type choices as player IDs for the choice system.
const TYPE_CREATURE: PlayerId = PlayerId(0);
const TYPE_ARTIFACT: PlayerId = PlayerId(1);
const TYPE_ENCHANTMENT: PlayerId = PlayerId(2);
const TYPE_LAND: PlayerId = PlayerId(3);
const TYPE_PLANESWALKER: PlayerId = PlayerId(4);

impl CardBehavior for CreepingRenaissance {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Creeping Renaissance".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.\nFlashback {5}{G}{G}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));

        // Build list of permanent types that have cards in the graveyard.
        let graveyard = state.objects_in_zone(Zone::Graveyard, controller);
        let mut options: Vec<Target> = Vec::new();
        let type_checks: [(PlayerId, CardType, &str); 5] = [
            (TYPE_CREATURE, CardType::Creature, "Creature"),
            (TYPE_ARTIFACT, CardType::Artifact, "Artifact"),
            (TYPE_ENCHANTMENT, CardType::Enchantment, "Enchantment"),
            (TYPE_LAND, CardType::Land, "Land"),
            (TYPE_PLANESWALKER, CardType::Planeswalker, "Planeswalker"),
        ];
        for &(sentinel, ref card_type, _label) in &type_checks {
            let has_type = graveyard.iter().any(|o| {
                o.card_types.contains(card_type)
                || registry.card_data(o.card_id)
                    .map(|d| d.card_types.contains(card_type))
                    .unwrap_or(false)
            });
            if has_type {
                options.push(Target::Player(sentinel));
            }
        }

        if options.len() == 1 {
            // Only one type has cards — auto-select.
            if let Target::Player(sentinel) = options[0] {
                creeping_renaissance_return(state, object_id, sentinel, registry);
            }
        } else if options.len() > 1 {
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: object_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Creeping Renaissance: choose a permanent type (Creature=p0, Artifact=p1, Enchantment=p2, Land=p3, Planeswalker=p4)".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::CardCallbackWithTarget { source_id: object_id },
                },
            });
        } else {
            state.log(crate::state::LogLevel::Event,
                "Creeping Renaissance: no permanent cards in graveyard".into());
            state.move_spell_after_resolve(object_id);
        }
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, registry: &CardRegistry) {
        if let Target::Player(sentinel) = target {
            creeping_renaissance_return(state, self_id, *sentinel, registry);
        }
    }
}

fn creeping_renaissance_return(state: &mut GameState, spell_id: ObjectId, sentinel: PlayerId, registry: &CardRegistry) {
    let card_type = match sentinel {
        TYPE_CREATURE => CardType::Creature,
        TYPE_ARTIFACT => CardType::Artifact,
        TYPE_ENCHANTMENT => CardType::Enchantment,
        TYPE_LAND => CardType::Land,
        TYPE_PLANESWALKER => CardType::Planeswalker,
        _ => CardType::Creature,
    };
    let type_name = match sentinel {
        TYPE_CREATURE => "creature",
        TYPE_ARTIFACT => "artifact",
        TYPE_ENCHANTMENT => "enchantment",
        TYPE_LAND => "land",
        TYPE_PLANESWALKER => "planeswalker",
        _ => "creature",
    };
    let controller = state.get_object(spell_id).map(|o| o.controller).unwrap_or(PlayerId(0));
    let to_return: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
        .iter()
        .filter(|o| {
            o.card_types.contains(&card_type)
            || registry.card_data(o.card_id)
                .map(|d| d.card_types.contains(&card_type))
                .unwrap_or(false)
        })
        .map(|o| o.id)
        .collect();

    let count = to_return.len();
    for id in to_return {
        state.move_object(id, Zone::Hand);
    }
    state.log(crate::state::LogLevel::Event,
        format!("Creeping Renaissance returned {} {} cards from graveyard to hand", count, type_name));
    state.move_spell_after_resolve(spell_id);
}
