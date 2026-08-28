use crate::cards::{CardBehavior, CardData, CardRegistry, ManaAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, ManaType};

/// Deranged Assistant — {1}{U} 1/1 Human Wizard.
/// {T}, Mill a card: Add {C}.
///
/// Note: The mill-a-card cost is handled as part of the mana ability resolution.
/// The engine's mana ability system calls `on_activate_mana_ability` (via the
/// standard tap-for-mana path), so we mill during the mana production step.
pub struct DerangedAssistant;

impl CardBehavior for DerangedAssistant {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Deranged Assistant".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Wizard".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "{T}, Mill a card: Add {C}. (To mill a card, put the top card of your library into your graveyard.)".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, state: &GameState, object_id: ObjectId) -> Vec<ManaAbilityDef> {
        // The zone / untapped / summoning-sickness part of the cost is the
        // engine's to check (`available_mana_abilities`); what's particular to
        // this ability is that milling a card needs a card left to mill.
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if state.get_player(obj.controller).library_order.is_empty() {
            return vec![];
        }
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Mill a card, add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: true,
        }]
    }

    fn on_activate_mana_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);
        // Mill a card as part of the mana ability cost.
        crate::engine::mill_cards(state, controller, 1, "Deranged Assistant", registry);
    }
}
