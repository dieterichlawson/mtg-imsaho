use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, TargetRequirement, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Sever the Bloodline — {3}{B} Sorcery.
/// Exile target creature and all other creatures with the same name as that creature.
/// Flashback {5}{B}{B}.
pub struct SeverTheBloodline;

impl CardBehavior for SeverTheBloodline {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Sever the Bloodline".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Exile target creature and all other creatures with the same name as that creature.\nFlashback {5}{B}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Black),
            ])),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::Creature
    }

    fn on_resolve(&self, state: &mut GameState, _object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            if state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                // Get the name of the target creature.
                let name = state.get_object(*target_id).map(|o| o.name.clone()).unwrap_or_default();

                // Find all creatures on the battlefield with the same name.
                let to_exile: Vec<ObjectId> = state.objects.values()
                    .filter(|o| o.zone == Zone::Battlefield && o.power.is_some() && o.name == name)
                    .map(|o| o.id)
                    .collect();

                for id in &to_exile {
                    state.move_object(*id, Zone::Exile, registry);
                }

                state.log(crate::state::LogLevel::Event,
                    format!("Sever the Bloodline exiles {} creature(s) named \"{}\"", to_exile.len(), name));
            }
        }
    }
}
