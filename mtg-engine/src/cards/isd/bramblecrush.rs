use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Bramblecrush — {2}{G}{G} sorcery. Destroy target noncreature permanent.
pub struct Bramblecrush;

impl CardBehavior for Bramblecrush {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bramblecrush".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Destroy target noncreature permanent.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::Noncreature)
    }

    /// No `is_valid_target`: "noncreature permanent on the battlefield" is
    /// exactly `PermanentWithFilter(Noncreature)`, which `legal_actions`
    /// applies when offering targets and `stack::is_target_legal` re-applies on
    /// the way down along with the zone check (CR 608.2b).
    ///
    /// The two also asked slightly different questions: the card read
    /// `face_data`, i.e. printed card types, where `TargetFilter::Noncreature`
    /// is `!state.is_creature(..)`, which counts the object's own types and the
    /// P/T sentinel a token or an animated permanent carries. Nothing in this
    /// set makes those answers differ — swapping the filter to the printed
    /// reading fails no test — so this is a restatement being removed rather
    /// than a disagreement being resolved.
    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        // "Destroy" always goes through the destruction pipeline,
        // which checks indestructible and regeneration.
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
