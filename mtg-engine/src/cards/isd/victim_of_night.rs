use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Victim of Night — {B}{B} instant. Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
pub struct VictimOfNight;

impl CardBehavior for VictimOfNight {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Victim of Night".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target non-Vampire, non-Werewolf, non-Zombie creature.".into(),
            ..Default::default()
        }
    }

    /// The three exclusions are one filter, applied when the target is chosen
    /// (CR 601.2c) and again on resolution (CR 608.2b), creature-ness included.
    /// This card also carried an `is_valid_target` reading
    /// `!["Vampire", "Werewolf", "Zombie"].iter().any(|st| state.has_subtype(..))`;
    /// `NotSubtypes` is `!types.iter().any(|t| state.subtypes_of(..).contains(t))`,
    /// and `has_subtype` is membership in exactly the union `subtypes_of`
    /// returns — the same question, asked twice.
    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(vec![
            "Vampire".into(),
            "Werewolf".into(),
            "Zombie".into(),
        ]))
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
