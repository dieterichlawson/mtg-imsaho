use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Harvest Pyre — {1}{R} Instant.
/// As an additional cost to cast this spell, exile X cards from your graveyard.
/// Harvest Pyre deals X damage to target creature.
pub struct HarvestPyre;

impl CardBehavior for HarvestPyre {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Harvest Pyre".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "As an additional cost to cast this spell, exile X cards from your graveyard.\nHarvest Pyre deals X damage to target creature.".into(),
            additional_cost: Some(crate::cards::AdditionalCost::ExileXFromGraveyard),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // X was fixed while the spell was cast — "as an additional cost",
        // CR 601.2b — and the engine recorded what it exiled. No guard on
        // X being zero: CR 120.8 says an effect that would deal 0 damage
        // deals none, and `damage::deal_damage` is where that lives.
        let count = state.get_object(object_id)
            .and_then(|o| o.card_state.get(crate::cards::EXILE_COUNT).copied())
            .map_or(0, |id| u32::try_from(id.0).unwrap_or(u32::MAX));

        if let Some(Target::Object(target_id)) = targets.first() {
            let effect = crate::state::PendingEffect::DealDamage {
                amount: count,
                source_id: object_id,
                source_name: "Harvest Pyre".into(),
            };
            crate::engine::apply_pending_effect(
                state,
                &Target::Object(*target_id),
                &effect,
                registry,
            );
        }
    }
}
