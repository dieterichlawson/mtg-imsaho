use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Ancient Grudge — {1}{R} instant. Destroy target artifact. Flashback {G}.
pub struct AncientGrudge;

impl CardBehavior for AncientGrudge {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Ancient Grudge".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Destroy target artifact.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PermanentWithFilter(TargetFilter::HasCardType(vec![CardType::Artifact]))
    }

    /// No `is_valid_target`: "an artifact on the battlefield" is exactly
    /// `PermanentWithFilter(HasCardType([Artifact]))`, which `legal_actions`
    /// applies when offering targets and `stack::is_target_legal` re-applies
    /// with the zone check on the way down (CR 608.2b).
    ///
    /// Note what the requirement does *not* say: nothing about creatures. An
    /// artifact creature is an artifact, so it is a legal target — the mirror
    /// of Bramblecrush, whose "noncreature permanent" refuses the same card.
    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
