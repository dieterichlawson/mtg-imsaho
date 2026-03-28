use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Elder Cathar — {2}{W} 2/2 Human Soldier.
/// When Elder Cathar dies, put a +1/+1 counter on target creature you control.
/// If that creature is a Human, put two +1/+1 counters on it instead.
pub struct ElderCathar;

impl CardBehavior for ElderCathar {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Elder Cathar".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Human".into(), "Soldier".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "When Elder Cathar dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.".into(),
            keywords: vec![],
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, _registry: &CardRegistry) {
        let controller = state.get_object(object_id).map(|o| o.controller).unwrap_or(crate::ids::PlayerId(0));
        // Find a creature we control on the battlefield.
        let target = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == controller && o.power.is_some() && o.id != object_id)
            .map(|o| o.id)
            .next();
        if let Some(target_id) = target {
            // Simplified: always add 1 counter (Human check is future work).
            state.add_counters(target_id, CounterType::PlusOnePlusOne, 1);
            state.log(crate::state::LogLevel::Event, "Elder Cathar's death granted a +1/+1 counter".into());
        }
    }
}
