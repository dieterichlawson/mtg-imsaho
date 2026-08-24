use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Make a Wish — {3}{G} Sorcery.
/// Return two cards at random from your graveyard to your hand.
pub struct MakeAWish;

impl CardBehavior for MakeAWish {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Make a Wish".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Return two cards at random from your graveyard to your hand.".into(),
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        use rand::seq::SliceRandom;

        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);

        // Get all cards in graveyard (excluding tokens).
        let mut gy_cards: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| !o.is_token && o.id != object_id)
            .map(|o| o.id)
            .collect();

        // Pick two at random.
        let mut rng = rand::thread_rng();
        gy_cards.shuffle(&mut rng);

        let to_return: Vec<ObjectId> = gy_cards.into_iter().take(2).collect();
        for card_id in &to_return {
            let name = state.get_object(*card_id).map(|o| o.name.clone()).unwrap_or_default();
            state.move_object(*card_id, Zone::Hand, registry);
            state.log(crate::state::LogLevel::Event,
                format!("Make a Wish returned {name} to hand"));
        }

        if to_return.is_empty() {
            state.log(crate::state::LogLevel::Event,
                "Make a Wish: no cards in graveyard to return".into());
        }

    }
}
