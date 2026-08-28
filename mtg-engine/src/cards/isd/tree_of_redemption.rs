use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Tree of Redemption — {3}{G} 0/13 Plant with Defender.
/// {T}: Exchange your life total with Tree of Redemption's toughness.
pub struct TreeOfRedemption;

impl CardBehavior for TreeOfRedemption {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Tree of Redemption".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Plant".into()],
            power: Some(0),
            toughness: Some(13),
            oracle_text: "Defender\n{T}: Exchange your life total with this creature's toughness.".into(),
            keywords: vec![Keyword::Defender],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !obj.tapped {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{T}: Exchange life total with Tree's toughness".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: None,
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        // The ability exchanges the life total with *this creature's* toughness,
        // so it does nothing if the Tree is no longer on the battlefield when it
        // resolves (destroyed or bounced in response). CR 608.2: an ability that
        // can't perform its action does as much as it can, which here is nothing.
        if !crate::cards::helpers::still_on_battlefield(state, object_id) {
            return;
        }
        let controller = crate::cards::helpers::controller_of(state, object_id);
        let current_toughness = state.effective_toughness(object_id, registry).unwrap_or(13);
        let current_life = state.get_player(controller).life;

        // An exchange is still a life change: through `change_life`, so the
        // LifeChanged event is emitted the same way as everywhere else.
        let old_life = current_life;
        state.change_life(controller, current_toughness - current_life);

        if let Some(obj) = state.get_object_mut(object_id) {
            obj.toughness = Some(current_life);
        }

        state.log(crate::state::LogLevel::Event,
            format!("Tree of Redemption: exchanged life ({old_life}) with toughness ({current_toughness})"));
    }
}
