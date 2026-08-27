use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetFilter, TargetRequirement, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(vec!["Vampire".into(), "Werewolf".into(), "Zombie".into()]))
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let Some(obj) = state.get_object(*id) else { return false; };
                if obj.zone != Zone::Battlefield || !state.is_creature(obj.id, registry) { return false; }
                !["Vampire", "Werewolf", "Zombie"].iter()
                    .any(|st| state.has_subtype(obj.id, st, registry))
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_destroy(state, targets, registry);
    }
}
