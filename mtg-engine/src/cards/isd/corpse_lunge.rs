use crate::actions::Target;
use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Corpse Lunge — {2}{B} Instant.
/// As an additional cost to cast Corpse Lunge, exile a creature card from your graveyard.
/// Corpse Lunge deals damage equal to the exiled card's power to target creature.
pub struct CorpseLunge;

impl CardBehavior for CorpseLunge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Corpse Lunge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "As an additional cost to cast this spell, exile a creature card from your graveyard.\nCorpse Lunge deals damage equal to the exiled card's power to target creature.".into(),
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // "damage equal to the exiled card's power": its power as it last
        // existed in the graveyard, written as the cost was paid
        // (`cards::EXILED_TO_COST_POWER` cites the ruling). Not read in exile
        // as the spell resolves, which left Boneyard Wurm out of its own
        // count of the creature cards there (defect 42).
        let power = state.get_object(object_id)
            .and_then(|o| o.card_state.get(&crate::cards::exiled_to_cost_power_key(0)).copied())
            .map_or(0, |p| p.0);

        let damage = u32::try_from(power).unwrap_or(u32::MAX);
        if let Some(Target::Object(target_id)) = targets.first() {
            crate::damage::deal_damage(state, object_id,
                crate::events::DamageTarget::Object(*target_id), damage,
                crate::damage::DamageKind::NonCombat, registry);
        }
    }
}
