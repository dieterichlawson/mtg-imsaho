use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

/// Divine Reckoning — {2}{W}{W} Sorcery.
/// Each player chooses a creature they control, then sacrifices the rest.
/// Flashback {5}{W}{W}.
pub struct DivineReckoning;

impl CardBehavior for DivineReckoning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Divine Reckoning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Sorcery],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "Each player chooses a creature they control. Destroy the rest.\nFlashback {5}{W}{W}".into(),
            keywords: vec![],
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::White),
            ])),
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let caster = state.get_object(object_id).map(|o| o.controller).unwrap_or(PlayerId(0));
        // Start with caster's choice, then opponent's.
        divine_reckoning_choose_for_player(state, object_id, caster, registry);
    }

    fn on_target_chosen(&self, state: &mut GameState, self_id: ObjectId, target: &Target, registry: &CardRegistry) {
        if let Target::Object(keeper_id) = target {
            // Determine which player's choice this was.
            let phase = state.get_object(self_id)
                .and_then(|o| o.card_state.get("phase").copied())
                .map(|id| id.0)
                .unwrap_or(0);

            let keeper_name = state.get_object(*keeper_id).map(|o| o.name.clone()).unwrap_or_default();
            let keeper_controller = state.get_object(*keeper_id).map(|o| o.controller).unwrap_or(PlayerId(0));
            state.log(LogLevel::Event, format!("p{} keeps {}", keeper_controller.0, keeper_name));

            // Store the keeper.
            if let Some(obj) = state.get_object_mut(self_id) {
                let key = format!("keeper_{}", phase);
                obj.card_state.insert(key, *keeper_id);
            }

            if phase == 0 {
                // Move to opponent's choice.
                let caster = state.get_object(self_id).map(|o| o.controller).unwrap_or(PlayerId(0));
                let opponent = state.opponent(caster);
                if let Some(obj) = state.get_object_mut(self_id) {
                    obj.card_state.insert("phase".into(), ObjectId(1));
                }
                divine_reckoning_choose_for_player(state, self_id, opponent, registry);
            } else {
                // Both players have chosen — destroy the rest.
                divine_reckoning_execute(state, self_id, registry);
            }
        }
    }
}

fn divine_reckoning_choose_for_player(state: &mut GameState, spell_id: ObjectId, player: PlayerId, registry: &CardRegistry) {
    let creatures: Vec<ObjectId> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.controller == player && o.power.is_some())
        .map(|o| o.id)
        .collect();

    if creatures.len() <= 1 {
        // 0 or 1 creatures — auto-keep (nothing to choose).
        if let Some(keeper) = creatures.first() {
            let keeper_name = state.get_object(*keeper).map(|o| o.name.clone()).unwrap_or_default();
            state.log(LogLevel::Event, format!("p{} keeps {}", player.0, keeper_name));
            let phase = state.get_object(spell_id)
                .and_then(|o| o.card_state.get("phase").copied())
                .map(|id| id.0)
                .unwrap_or(0);
            if let Some(obj) = state.get_object_mut(spell_id) {
                let key = format!("keeper_{}", phase);
                obj.card_state.insert(key, *keeper);
            }
        }

        // Check phase and advance.
        let phase = state.get_object(spell_id)
            .and_then(|o| o.card_state.get("phase").copied())
            .map(|id| id.0)
            .unwrap_or(0);

        if phase == 0 {
            let caster = state.get_object(spell_id).map(|o| o.controller).unwrap_or(PlayerId(0));
            let opponent = state.opponent(caster);
            if let Some(obj) = state.get_object_mut(spell_id) {
                obj.card_state.insert("phase".into(), ObjectId(1));
            }
            divine_reckoning_choose_for_player(state, spell_id, opponent, registry);
        } else {
            divine_reckoning_execute(state, spell_id, registry);
        }
        return;
    }

    // Present choice to player.
    let options: Vec<Target> = creatures.iter().map(|&id| Target::Object(id)).collect();
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player,
        source: spell_id,
        choice: ResolutionChoiceKind::ChooseTarget {
            description: format!("Divine Reckoning: p{}, choose a creature to keep", player.0),
            options,
            optional: false,
            effect: PendingEffect::CardCallbackWithTarget { source_id: spell_id },
        },
    });
}

fn divine_reckoning_execute(state: &mut GameState, spell_id: ObjectId, registry: &CardRegistry) {
    // Collect keepers from card_state.
    let keeper_0 = state.get_object(spell_id)
        .and_then(|o| o.card_state.get("keeper_0").copied());
    let keeper_1 = state.get_object(spell_id)
        .and_then(|o| o.card_state.get("keeper_1").copied());

    let keepers: Vec<ObjectId> = [keeper_0, keeper_1].iter().filter_map(|k| *k).collect();

    // Destroy all creatures not in the keepers list.
    let to_destroy: Vec<ObjectId> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && !keepers.contains(&o.id))
        .map(|o| o.id)
        .collect();

    for id in to_destroy {
        crate::destruction::try_destroy(state, id, registry);
    }

    state.move_spell_after_resolve(spell_id);
}
