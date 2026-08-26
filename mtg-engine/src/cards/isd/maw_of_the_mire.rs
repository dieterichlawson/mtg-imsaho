use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TargetFilter};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

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

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = match state.get_object(*id) {
                    Some(o) if o.zone == Zone::Battlefield => o,
                    _ => return false,
                };
                state.face_data(obj.id, registry)
                    .is_some_and(|d| d.card_types.contains(&CardType::Land))
            }
            Target::Player(_) => false,
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        if let Some(Target::Object(land_id)) = targets.first() {
            // If the target is illegal (not on battlefield), the spell fizzles — no effects.
            if !state.get_object(*land_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                return;
            }
            let name = state.get_object(*land_id).map(|o| o.name.clone()).unwrap_or_default();
            crate::destruction::try_destroy(state, *land_id, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Maw of the Mire destroyed {name}"));

            // Gain 4 life (only if target was valid).
            let old_life = state.get_player(controller).life;
            let new_life = old_life + 4;
            state.get_player_mut(controller).life = new_life;
            state.events.push(crate::events::GameEvent::LifeChanged {
                player: controller,
                old: old_life,
                new_life,
            });
            state.log(crate::state::LogLevel::Event,
                format!("Maw of the Mire: p{} gained 4 life", controller.0));
        }

    }
}
