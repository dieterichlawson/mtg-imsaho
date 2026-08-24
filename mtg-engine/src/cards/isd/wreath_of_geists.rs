use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Wreath of Geists — {G} Aura enchantment. Enchant creature.
/// Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
pub struct WreathOfGeists;

impl CardBehavior for WreathOfGeists {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Wreath of Geists".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into()],
            oracle_text: "Enchant creature\nEnchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn dynamic_pt(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Option<(i32, i32)> {
        // The controller of the aura determines whose graveyard to count.
        let controller = state.get_object(object_id)?.controller;
        let creature_count = i32::try_from(state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.power.is_some())
            .count()).unwrap_or(i32::MAX);
        Some((creature_count, creature_count))
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_aura(state, object_id, targets, registry);
    }
}
