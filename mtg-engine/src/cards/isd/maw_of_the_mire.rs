use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TargetFilter};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Maw of the Mire — {4}{B} Sorcery.
/// Destroy target land. You gain 4 life.
pub struct MawOfTheMire;

impl CardBehavior for MawOfTheMire {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Maw of the Mire".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Destroy target land. You gain 4 life.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(
            TargetFilter::HasCardType(vec![CardType::Land]),
        )
    }

    /// "Destroy target land. You gain 4 life."
    ///
    /// No `is_valid_target` and no zone guard here. Both were duplicates of
    /// `PermanentWithFilter(HasCardType([Land]))`, which `legal_actions`
    /// applies when offering targets and `stack::is_target_legal` re-applies
    /// on the way down (CR 608.2b) along with a zone check. Reaching this
    /// function at all means the target was still a land on the battlefield —
    /// so the guard could never fire, and the published ruling ("it won't
    /// resolve and none of its effects will occur. You won't gain 4 life") is
    /// the engine countering the spell before this runs, not the card
    /// returning early.
    ///
    /// The card's copy was also *narrower* than the filter it restated: it
    /// asked `face_data`, i.e. printed types only, where `has_card_type` is
    /// printed types union whatever the object has been granted.
    ///
    /// The two sentences are sequential, not conditional: an indestructible
    /// land survives (CR 701.7b) and the 4 life is gained anyway.
    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let Some(Target::Object(land_id)) = targets.first() else { return };
        let controller = crate::cards::helpers::controller_of(state, object_id);

        crate::destruction::try_destroy_by(state, *land_id, "Maw of the Mire", registry);

        state.change_life(controller, 4);
        state.log(crate::state::LogLevel::Event,
            format!("Maw of the Mire: p{} gained 4 life", controller.0));
    }
}
