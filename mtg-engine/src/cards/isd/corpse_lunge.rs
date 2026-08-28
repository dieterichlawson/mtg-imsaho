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
        // "damage equal to the exiled card's power" — the card's power where it
        // now is, asked now. The card was exiled to pay the additional cost
        // (CR 601.2f), and a characteristic-defining power goes with it
        // (CR 604.3), so Boneyard Wurm exiled out of a graveyard stops counting
        // itself among the creature cards there. The engine used to hand this
        // card a power snapshotted while the Wurm was still in the graveyard,
        // one too high.
        let power = state.get_object(object_id)
            .and_then(|o| o.card_state.get(&crate::cards::exiled_to_cost_key(0)).copied())
            .and_then(|exiled| state.effective_power(exiled, registry))
            .unwrap_or(0);

        let damage = u32::try_from(power.max(0)).unwrap_or(0);
        if let Some(Target::Object(target_id)) = targets.first() {
            crate::damage::deal_damage(state, object_id,
                crate::events::DamageTarget::Object(*target_id), damage,
                crate::damage::DamageKind::NonCombat, registry);
        }
    }
}
