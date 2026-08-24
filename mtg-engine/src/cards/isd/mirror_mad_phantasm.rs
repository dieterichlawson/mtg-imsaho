use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Mirror-Mad Phantasm {3}{U}{U} 5/1 Spirit with Flying.
/// {1}{U}: Mirror-Mad Phantasm's owner shuffles it into their library. If that player does,
/// they reveal cards from the top of their library until they reveal a card named Mirror-Mad
/// Phantasm, put that card onto the battlefield, and put all other cards revealed this way
/// into their graveyard.
///
/// Simplified: Shuffle into library, then mill until we find Mirror-Mad Phantasm (or run out).
/// If found, put it on the battlefield. All others go to graveyard.
pub struct MirrorMadPhantasm;

impl CardBehavior for MirrorMadPhantasm {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mirror-Mad Phantasm".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Spirit".into()],
            power: Some(5),
            toughness: Some(1),
            oracle_text: "Flying\n{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.".into(),
            keywords: vec![Keyword::Flying],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![],
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{U}: Shuffle into library, reveal until Mirror-Mad Phantasm, mill the rest".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        let owner = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o.owner,
            _ => return,
        };

        state.move_object(object_id, Zone::Library, registry);
        state.get_player_mut(owner).library_order.push(object_id);
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            state.get_player_mut(owner).library_order.shuffle(&mut rng);
        }

        state.log(crate::state::LogLevel::Event,
            "Mirror-Mad Phantasm shuffled into library".into());

        let mut milled = Vec::new();
        let mut found = None;
        loop {
            let top = state.get_player_mut(owner).reveal_top_card();
            match top {
                Some(card_id) => {
                    let name = state.get_object(card_id).map(|o| o.name.clone()).unwrap_or_default();
                    if name == "Mirror-Mad Phantasm" {
                        found = Some(card_id);
                        break;
                    }
                    milled.push(card_id);
                }
                None => break,
            }
        }

        for card_id in &milled {
            state.move_object(*card_id, Zone::Graveyard, registry);
        }

        if let Some(phantasm_id) = found {
            state.move_object(phantasm_id, Zone::Battlefield, registry);
            if let Some(obj) = state.get_object_mut(phantasm_id) {
                obj.controller = owner;
            }
            state.log(crate::state::LogLevel::Event,
                format!("Mirror-Mad Phantasm: found after milling {} cards, enters the battlefield", milled.len()));
        } else {
            state.log(crate::state::LogLevel::Event,
                format!("Mirror-Mad Phantasm: milled {} cards, not found (entire library milled!)", milled.len()));
        }
    }
}
