use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};

/// Brain Weevil — {3}{B} 1/1 Insect. Intimidate.
/// Sacrifice Brain Weevil: Target player discards two cards. Activate only as a sorcery.
pub struct BrainWeevil;

impl CardBehavior for BrainWeevil {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Brain Weevil".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Insect".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)\nSacrifice this creature: Target player discards two cards. Activate only as a sorcery.".into(),
            keywords: vec![Keyword::Intimidate],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice: Target player discards two cards".into(),
            cost: ManaCost::new(vec![]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: Some(TargetRequirement::PlayerOnly),
            once_per_turn: false,
            sorcery_speed_only: true,
            counter_cost: None,
        }]
    }

    /// "Sacrifice this creature: Target player discards two cards."
    ///
    /// Discarding N cards, with the player choosing each one, is the engine's
    /// (`engine::discard_cards`). This used to be a hand-rolled chain: discard
    /// the first, then pick the second up in `on_discard_choice`, with the
    /// target player carried between the two in `card_state` — a
    /// `String -> ObjectId` map — as `ObjectId(player.0 as u64)`, read back
    /// through an `unwrap_or(u8::MAX)` that would have named a player who does
    /// not exist. It also wrote that state onto a permanent the engine had
    /// already sacrificed to pay the cost.
    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        let Some(&Target::Player(target_player)) = targets.first() else { return };
        crate::engine::discard_cards(state, target_player, 2, object_id, "Brain Weevil", registry);
    }
}
