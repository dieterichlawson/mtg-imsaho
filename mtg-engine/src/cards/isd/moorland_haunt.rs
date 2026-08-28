use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, ManaSymbol, Color, Keyword};

/// Moorland Haunt — Land.
/// {T}: Add {C}.
/// {W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white
/// Spirit creature token with flying.
pub struct MoorlandHaunt;

impl CardBehavior for MoorlandHaunt {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Moorland Haunt".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone != Zone::Battlefield || obj.tapped {
            return vec![];
        }

        let controller = obj.controller;

        // Check if there's a creature card in the graveyard to exile.
        let has_creature_in_graveyard = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .any(|o| state.is_creature(o.id, registry) && state.is_card(o.id));

        if has_creature_in_graveyard {
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{W}{U}, {T}, Exile a creature from graveyard: Create 1/1 white Spirit with flying".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Colored(Color::White),
                    ManaSymbol::Colored(Color::Blue),
                ]),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    /// "{1}{W}, {T}, Exile a creature card from your graveyard:" — everything
    /// before the colon is a cost, paid on activation (CR 601.2h via 602.2b).
    /// The token it buys is the effect and waits for resolution.
    fn pay_activation_cost(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // CR 109.1: "a creature CARD in your graveyard", so a token there is
        // not one of them.
        let creatures_in_gy: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| state.is_creature(o.id, registry) && state.is_card(o.id))
            .map(|o| o.id)
            .collect();

        match creatures_in_gy.len() {
            0 => {}
            1 => {
                exile_for_cost(state, creatures_in_gy[0], registry);
            }
            _ => {
                // Which card to exile is the player's choice; `resolve_card_effect`
                // pays it and then puts the ability on the stack.
                let options: Vec<Target> = creatures_in_gy.iter().map(|&id| Target::Object(id)).collect();
                crate::cards::helpers::present_target_choice(
                    state, object_id, controller, options,
                    crate::state::PendingEffect::CardEffect {
                        source_id: object_id,
                        key: String::new(),
                    },
                    "Moorland Haunt: choose a creature card from your graveyard to exile",
                    false,
                    registry,
                );
            }
        }
    }

    /// The tail of the cost payment. CR 602.2b sends activation through the
    /// casting steps, where the ability is on the stack (CR 602.2a) before
    /// costs are paid (CR 601.2h) — so the engine has already pushed it and
    /// this only finishes paying.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        exile_for_cost(state, *id, registry);
    }

    /// "Create a 1/1 white Spirit creature token with flying." The token's
    /// characteristics are this card's text, not the engine's business.
    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        state.create_token_with_subtypes(
            "", controller, 1, 1,
            vec![Color::White], vec![CardType::Creature],
            vec![Keyword::Flying], vec!["Spirit".into()], registry,
        );
        state.log(crate::state::LogLevel::Event,
            "Moorland Haunt created a 1/1 white Spirit token with flying".into());
    }
}

/// Exile one creature card from a graveyard to pay Moorland Haunt's cost.
fn exile_for_cost(state: &mut GameState, id: ObjectId, registry: &CardRegistry) {
    let name = state.obj_name(id);
    state.move_object(id, Zone::Exile, registry);
    state.log(crate::state::LogLevel::Event,
        format!("Moorland Haunt exiled {name} from graveyard (cost)"));
}
