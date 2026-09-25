use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Mindshrieker — {1}{U} 1/1 Spirit Bird with Flying.
/// {2}: Target player mills a card. Mindshrieker gets +X/+X until end of turn,
/// where X is the milled card's mana value.
pub struct Mindshrieker;

impl CardBehavior for Mindshrieker {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Mindshrieker".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into(), "Bird".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Flying\n{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{2}: Target player mills a card. Mindshrieker gets +X/+X (X = mana value)".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(2),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::PlayerOnly),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            // "Target player mills a card." Through the mill pipeline, so a
            // creature card among them emits CreatureCardMilled (an opponent's
            // Undead Alchemist watches for exactly this) and so the log line
            // names the source. This used to take the top of `library_order`,
            // call `mill_one` by hand, and log an unsourced "p1 milled 1 card"
            // beside it.
            let milled = crate::engine::mill_cards(state, *player_id, 1, "Mindshrieker", registry);

            // "where X is the milled card's mana value" — so an empty library
            // mills nothing and X never exists.
            let Some(&milled_id) = milled.first() else { return };
            let mana_value = i32::try_from(
                state.face_data(milled_id, registry)
                    .and_then(|d| d.cost.map(|c| c.mana_value()))
                    .unwrap_or(0)
            ).unwrap_or(i32::MAX);

            // Two decisions, which used to be one condition (#590).
            //
            // Whether to make an entry in `until_end_of_turn` is a question
            // about the game: +0/+0 changes nothing, and CR 400.7 says a
            // permanent that has left the battlefield is not there to be
            // modified.
            //
            // Whether to say what X came out as is a question about the
            // record, and the answer is always yes. A land has no mana cost at
            // all and a land is the commonest card in any library, so the
            // commonest outcome of this ability was reported by the *absence*
            // of a line — byte-identical to an ability that resolved and did
            // nothing. A player at the CLI can recover the answer from the
            // battlefield pane; the LLM seat is handed this log and no pane,
            // so for that seat the ability's whole effect on its own creature
            // went unreported. Same question #553 settled for damage that came
            // out 0 and #299 for a permanent entering with 0 counters.
            let still_on_battlefield = state.get_object(object_id)
                .is_some_and(|o| o.zone == Zone::Battlefield);

            if mana_value > 0 && still_on_battlefield {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: object_id,
                        power_mod: mana_value,
                        toughness_mod: mana_value,
                    }
                );
            }

            let line = if still_on_battlefield {
                format!("Mindshrieker gets +{mana_value}/+{mana_value} (milled card's mana value)")
            } else {
                format!("Mindshrieker has left the battlefield, so its                          +{mana_value}/+{mana_value} (milled card's mana value) lands on nothing")
            };
            state.log(crate::state::LogLevel::Event, line);
        }
    }
}
