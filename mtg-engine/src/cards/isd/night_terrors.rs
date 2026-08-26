use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Night Terrors — {2}{B} Sorcery.
/// Target player reveals their hand. You choose a nonland card from it. Exile that card.
pub struct NightTerrors;

impl CardBehavior for NightTerrors {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Night Terrors".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Sorcery],
            oracle_text: "Target player reveals their hand. You choose a nonland card from it. Exile that card.".into(),
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        let controller = state.get_object(object_id).map_or(crate::ids::PlayerId(0), |o| o.controller);
        if let Some(Target::Player(target_player)) = targets.first() {
            // Reveal target player's hand — find all nonland cards.
            let nonland_cards: Vec<ObjectId> = state.objects_in_zone(Zone::Hand, *target_player)
                .iter()
                .filter(|o| {
                    let is_land = state.face_data(o.id, registry)
                        .is_some_and(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)));
                    !is_land
                })
                .map(|o| o.id)
                .collect();

            if nonland_cards.is_empty() {
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: no nonland card in p{}'s hand", target_player.0));
            } else if nonland_cards.len() == 1 {
                // Only one option — auto-select.
                let exile_id = nonland_cards[0];
                let name = state.get_object(exile_id).map(|o| o.name.clone()).unwrap_or_default();
                state.move_object(exile_id, Zone::Exile, registry);
                state.log(crate::state::LogLevel::Event,
                    format!("Night Terrors: exiled {} from p{}'s hand", name, target_player.0));
            } else {
                // Multiple nonland cards — controller chooses which to exile.
                crate::cards::helpers::present_target_choice(
                    state, object_id, controller, nonland_cards.iter().map(|&id| Target::Object(id)).collect(),
                    crate::state::PendingEffect::CardEffect { source_id: object_id, key: String::new() },
                    "Night Terrors: choose a nonland card to exile",
                    false,
                );
                return; // Don't move spell yet — awaiting choice.
            }
        }
    }

    /// "...exile a nonland card from it." Moving the chosen card to exile and
    /// finishing this spell's own resolution is Night Terrors' business.
    fn resolve_card_effect(&self, state: &mut GameState, _source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let name = state.obj_name(*id);
        state.move_object(*id, Zone::Exile, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Night Terrors exiled {name} from hand"));
    }
}
