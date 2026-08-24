use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Urgent Exorcism — {1}{W} instant. Destroy target Spirit or enchantment.
pub struct UrgentExorcism;

impl CardBehavior for UrgentExorcism {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Urgent Exorcism".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target Spirit or enchantment.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::SubtypeOrCardType { subtypes: vec!["Spirit".into()], card_types: vec![CardType::Enchantment] })
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                state.has_card_type(obj.id, CardType::Enchantment, registry)
                    || state.has_subtype(obj.id, "Spirit", registry)
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, object_id, targets, registry);
    }
}
