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
            supertypes: vec![],
            subtypes: vec![],
            power: None,
            toughness: None,
            oracle_text: "As an additional cost to cast this spell, exile a creature card from your graveyard.\nCorpse Lunge deals damage equal to the exiled card's power to target creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: Some(AdditionalCost::ExileCreaturesFromGraveyard(1)),
            triggered_abilities: vec![],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // The creature was exiled at cast time (additional cost). Read the stored power.
        let power = state.get_object(object_id)
            .and_then(|o| o.card_state.get("exiled_power").copied())
            .map_or(0, |id| i32::try_from(id.0).unwrap_or(i32::MAX));

        let damage = u32::try_from(power.max(0)).unwrap_or(0);
        if let Some(Target::Object(target_id)) = targets.first() {
            crate::damage::deal_damage(state, object_id,
                crate::events::DamageTarget::Object(*target_id), damage,
                crate::damage::DamageKind::NonCombat, registry);
        }
        state.move_spell_after_resolve(object_id, registry);
    }
}
