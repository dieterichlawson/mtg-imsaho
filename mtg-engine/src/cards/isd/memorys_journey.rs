use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Memory's Journey — {1}{U} Instant.
/// Target player shuffles up to three target cards from their graveyard into their library.
/// Flashback {G}.
pub struct MemorysJourney;

impl CardBehavior for MemorysJourney {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Memory's Journey".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Instant],
            oracle_text: "Target player shuffles up to three target cards from their graveyard into their library.\nFlashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        // Oracle: "Target player shuffles up to three target cards from their graveyard into their library."
        // Requires a mandatory player target plus up to 3 graveyard card targets.
        TargetRequirement::TwoTargets(
            Box::new(TargetRequirement::PlayerOnly),
            // "from THEIR graveyard" — constrained to the targeted player's,
            // not every graveyard in the game (CR 601.2c).
            Box::new(TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByTargetPlayer))),
        )
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let _controller = crate::cards::helpers::controller_of(state, object_id);

        // "TARGET PLAYER shuffles" — the player is a target in its own right,
        // and the first slot of the `TwoTargets` requirement. There is no
        // reading of this card without one: falling back to the controller, as
        // this used to, invents a target the caster never declared, and the
        // rule for a target that stopped being legal is CR 608.2b — the spell
        // does not resolve at all, which `stack::resolve_spell` applies before
        // this is ever called.
        let Some(target_player) = targets.iter().find_map(|t| match t {
            Target::Player(pid) => Some(*pid),
            Target::Object(_) | Target::Illegal => None,
        }) else { return };

        for target in targets {
            if let Target::Object(card_id) = target {
                let (name, owner, in_gy) = match state.get_object(*card_id) {
                    Some(o) => (o.name.clone(), o.owner, o.zone == Zone::Graveyard),
                    None => continue,
                };
                // Only shuffle cards from the targeted player's graveyard.
                if in_gy && owner == target_player {
                    state.put_into_library(*card_id, crate::state::LibraryPosition::Bottom, registry);
                    state.log(crate::state::LogLevel::Event,
                        format!("Memory's Journey: {name} shuffled into library"));
                }
            }
        }

        // Ruling: "If no cards were targeted by Memory's Journey or if all the
        // targeted cards are illegal targets by the time Memory's Journey
        // resolves, the targeted player will still shuffle their library."
        crate::cards::helpers::shuffle_library(state, target_player);
        state.log(crate::state::LogLevel::Event,
            format!("Memory's Journey: p{}'s library shuffled", target_player.0));
    }
}
