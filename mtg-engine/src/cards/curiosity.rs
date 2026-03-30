use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Curiosity — {U} Aura. Enchant creature.
/// Whenever enchanted creature deals damage to an opponent, you may draw a card.
///
/// TODO: The triggered ability is not yet implemented. It would need a hook to watch
/// damage events from the enchanted creature (the source of the aura's attached_to).
/// This requires an `on_enchanted_creature_deals_damage` watcher or similar mechanism.
/// For now, only the aura attachment is implemented.
pub struct Curiosity;

impl CardBehavior for Curiosity {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curiosity".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant creature\nWhenever enchanted creature deals damage to an opponent, you may draw a card.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            triggered_abilities: vec![],
            // TODO: Add triggered ability when the watcher hook is available.
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets);
    }
}
