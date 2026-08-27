use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, Keyword};

/// Spider Spawning — {4}{G} sorcery. Create a 1/2 green Spider creature token with reach
/// for each creature card in your graveyard. Flashback {6}{B}.
pub struct SpiderSpawning;

impl CardBehavior for SpiderSpawning {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Spider Spawning".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Create a 1/2 green Spider creature token with reach for each creature card in your graveyard.\nFlashback {6}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            flashback_cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(6),
                ManaSymbol::Colored(Color::Black),
            ])),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(PlayerId(0), |o| o.controller);
        // Count creature cards in controller's graveyard (excluding this spell which is still on the stack).
        let creature_count = state.objects.values()
            .filter(|o| o.zone == Zone::Graveyard && o.owner == controller && state.is_card(o.id) && state.is_creature(o.id, registry) && o.id != object_id)
            .count();
        for _ in 0..creature_count {
            state.create_token_with_subtypes("Spider", controller, 1, 2, vec![Color::Green], vec![CardType::Creature], vec![Keyword::Reach], vec!["Spider".into()], registry);
        }
        if creature_count > 0 {
            state.log(crate::state::LogLevel::Event, format!("Spider Spawning created {creature_count} Spider tokens"));
        }
    }
}
