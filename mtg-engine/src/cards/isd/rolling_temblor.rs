use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Rolling Temblor — {2}{R} sorcery. Deals 2 damage to each creature without flying.
pub struct RollingTemblor;

impl CardBehavior for RollingTemblor {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Rolling Temblor".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Rolling Temblor deals 2 damage to each creature without flying.\nFlashback {4}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Generic(4), ManaSymbol::Colored(Color::Red), ManaSymbol::Colored(Color::Red)])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let creatures: Vec<ObjectId> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry))
            .map(|o| o.id)
            .collect();
        for id in creatures {
            if !state.has_keyword(id, Keyword::Flying, registry) {
                crate::damage::deal_damage(state, object_id,
                    crate::events::DamageTarget::Object(id), 2,
                    crate::damage::DamageKind::NonCombat, registry);
            }
        }
    }
}
