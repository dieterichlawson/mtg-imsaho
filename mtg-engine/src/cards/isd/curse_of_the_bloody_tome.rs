use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Curse of the Bloody Tome — {2}{U} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, that player mills two cards.
pub struct CurseOfTheBloodyTome;

impl CardBehavior for CurseOfTheBloodyTome {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of the Bloody Tome".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, that player mills two cards.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "enchanted player mills two cards".into(),
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
            _ => return,
        };
        let cursed_player = match cursed_player {
            Some(p) => p,
            None => return,
        };
        if state.active_player != cursed_player {
            return;
        }
        crate::engine::mill_cards(state, cursed_player, 2);
        state.log(crate::state::LogLevel::Event,
            format!("Curse of the Bloody Tome: p{} milled 2 cards", cursed_player.0));
    }
}
