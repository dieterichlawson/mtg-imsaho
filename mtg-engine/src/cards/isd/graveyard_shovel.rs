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
            oracle_text: "{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // No zone-or-tapped guard here: `legal_actions` enumerates only
        // battlefield permanents its player controls and rejects a
        // `requires_tap` ability on a tapped one, and it also applies the
        // summoning-sickness rule this never did (CR 302.6 — irrelevant to a
        // land, but a card should not be the place that decides).
        // "Target player" is the whole targeting requirement — the printed
        // text has no "with a card in their graveyard" clause, so the
        // ability is activatable with every graveyard empty (even just to
        // tap the Shovel) and resolution does as much as it can, which may
        // be nothing (CR 601.2c, 608.2; issue #126).
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

    fn is_valid_target(&self, _state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        // Any player is a legal target — "target player", with no graveyard
        // clause. Requiring a card here also mis-countered the ability when
        // the graveyard was emptied in response, where CR 608.2b says it
        // resolves and simply does nothing (issue #126).
        matches!(target, Target::Player(_))
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(target_player)) = targets.first() {
            // Collect all cards in the targeted player's graveyard.
            let gy_cards: Vec<Target> = state.objects_in_zone(Zone::Graveyard, *target_player).into_iter()
                .filter(|o| state.is_card(o.id))
                .map(|o| Target::Object(o.id))
                .collect();

            if gy_cards.is_empty() {
                return;
            }

            if gy_cards.len() == 1 {
                // One card, so the choice is forced — but it is the same
                // effect, so it runs through the same code. This branch used
                // to be a second copy of it that tested "creature card" a
                // different way and wrote the life total with `change_life`
                // instead of `gain_life`; two copies of one effect is two
                // places for it to drift.
                self.resolve_card_effect(state, object_id, "", &gy_cards[0], registry);
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
            let controller = crate::cards::helpers::ability_controller(state, source_id);
            state.gain_life(controller, 2);
            state.log(crate::state::LogLevel::Event,
                format!("Graveyard Shovel: p{} gained 2 life (creature exiled)", controller.0));
        }
    }
}
