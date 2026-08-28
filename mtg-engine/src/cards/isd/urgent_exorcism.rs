use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

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

    /// "Spirit or enchantment" is exactly one filter, and the engine both
    /// offers targets by it (CR 601.2c) and re-checks it on resolution
    /// (CR 608.2b). This card also carried an `is_valid_target` that asked
    /// `has_card_type(Enchantment) || has_subtype("Spirit")` — the same two
    /// calls `matches_target_filter` makes for `SubtypeOrCardType` — behind a
    /// `zone == Battlefield` guard that the enumerator already guarantees.
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::SubtypeOrCardType {
            subtypes: vec!["Spirit".into()],
            card_types: vec![CardType::Enchantment],
        })
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
