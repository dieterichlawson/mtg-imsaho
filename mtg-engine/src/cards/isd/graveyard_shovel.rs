use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone};

/// Graveyard Shovel — {2} Artifact.
/// {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
pub struct GraveyardShovel;

impl CardBehavior for GraveyardShovel {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Graveyard Shovel".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
            ])),
            card_types: vec![CardType::Artifact],
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.".into(),
            keywords: vec![],
            flashback_cost: None, continuous_effects: vec![], additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone != Zone::Battlefield || obj.tapped {
            return vec![];
        }
        // Only available if at least one player has cards in their graveyard.
        let any_graveyard = state.objects.values().any(|o| o.zone == Zone::Graveyard);
        if !any_graveyard {
            return vec![];
        }
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{2}, {T}: Target player exiles a card from their graveyard, gain 2 life if creature".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::PlayerOnly),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Player(pid) => {
                // Only valid if the targeted player has at least one card in their graveyard.
                state.objects.values().any(|o| o.zone == Zone::Graveyard && o.owner == *pid)
            }
            Target::Object(_) => false,
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        if let Some(Target::Player(target_player)) = targets.first() {
            // Collect all cards in the targeted player's graveyard.
            let gy_cards: Vec<Target> = state.objects.values()
                .filter(|o| o.zone == Zone::Graveyard && o.owner == *target_player)
                .map(|o| Target::Object(o.id))
                .collect();

            if gy_cards.is_empty() {
                return;
            }

            if gy_cards.len() == 1 {
                // Only one card — auto-exile it.
                if let Target::Object(card_id) = &gy_cards[0] {
                    let is_creature = state.get_object(*card_id)
                        .is_some_and(|o| {
                            state.face_data(o.id, registry)
                                .map_or(o.power.is_some(), |d| d.card_types.iter().any(|ct| matches!(ct, CardType::Creature)))
                        });
                    let name = state.get_object(*card_id).map(|o| o.name.clone()).unwrap_or_default();
                    state.move_object(*card_id, Zone::Exile, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Graveyard Shovel: p{} exiled {} from graveyard", target_player.0, name));

                    if is_creature {
                        let old_life = state.get_player(controller).life;
                        let new_life = old_life + 2;
                        state.get_player_mut(controller).life = new_life;
                        state.events.push(crate::events::GameEvent::LifeChanged {
                            player: controller,
                            old: old_life,
                            new_life,
                        });
                        state.log(crate::state::LogLevel::Event,
                            format!("Graveyard Shovel: p{} gained 2 life (creature exiled)", controller.0));
                    }
                }
            } else {
                // Multiple cards — targeted player chooses which to exile.
                state.awaiting_action = Some(crate::state::AwaitingAction::ResolutionChoice {
                    player: *target_player,
                    source: object_id,
                    choice: crate::state::ResolutionChoiceKind::ChooseTarget {
                        description: "Graveyard Shovel: choose a card from your graveyard to exile".to_string(),
                        options: gy_cards,
                        optional: false,
                        effect: crate::state::PendingEffect::CardEffect { source_id: object_id, key: String::new() },
                    },
                });
            }
        }
    }

    /// "{2}, {T}: Target player exiles a card from their graveyard. If a
    /// creature card is exiled this way, you gain 2 life." The 2 life and the
    /// creature condition are this card's text.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let was_creature = state.is_creature(*id, registry);
        let name = state.obj_name(*id);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Graveyard Shovel: exiled {name} from graveyard"));

        if was_creature {
            let controller = crate::cards::helpers::controller_of(state, source_id);
            state.gain_life(controller, 2);
            state.log(crate::state::LogLevel::Event,
                format!("Graveyard Shovel: p{} gained 2 life (creature exiled)", controller.0));
        }
    }
}
