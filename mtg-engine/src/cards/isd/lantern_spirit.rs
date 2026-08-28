use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword, Zone};

/// Lantern Spirit — {2}{U} 2/1 Spirit with Flying. {U}: Return Lantern Spirit to its owner's hand.
pub struct LanternSpirit;

impl CardBehavior for LanternSpirit {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Lantern Spirit".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Spirit".into()],
            power: Some(2),
            toughness: Some(1),
            oracle_text: "Flying\n{U}: Return this creature to its owner's hand.".into(),
            keywords: vec![Keyword::Flying],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{U}: Return to hand".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Colored(Color::Blue),
                ]),
                requires_tap: false,
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
        // CR 400.7: a permanent that has left the battlefield is a new object,
        // and "return **this creature**" has nothing left to return. Without
        // this, killing the Spirit in response to its own ability rescued the
        // card out of the graveyard and into its owner's hand.
        //
        // The ability still resolves — it is not countered, there being no
        // target to become illegal — it just does as much as it can, which is
        // nothing (CR 608.2). Moldgraf Monstrosity guards its own zone change
        // for the same reason, in the other direction.
        if !crate::cards::helpers::still_on_battlefield(state, object_id) {
            return;
        }
        // "to its **owner's** hand", which is what `move_object` does: hands are
        // keyed by owner (CR 108.4), so a Spirit stolen with Traitorous Blood
        // goes home rather than to the thief.
        state.move_object(object_id, Zone::Hand, registry);
    }
}
