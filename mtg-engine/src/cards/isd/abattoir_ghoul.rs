use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Abattoir Ghoul — {3}{B} 3/2 Zombie. First strike.
/// Whenever a creature dealt damage by Abattoir Ghoul this turn dies,
/// you gain life equal to that creature's toughness.
pub struct AbattoirGhoul;

impl CardBehavior for AbattoirGhoul {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Abattoir Ghoul".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Zombie".into()],
            power: Some(3),
            toughness: Some(2),
            oracle_text: "First strike\nWhenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.".into(),
            keywords: vec![Keyword::FirstStrike],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "gain life equal to that creature's toughness".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// "Whenever a creature **dealt damage by this creature this turn** dies"
    /// — a condition on the event (CR 603.2), so a creature the Ghoul never
    /// touched is not this ability's event at all.
    ///
    /// `dead_damaged_by` comes from the death event: the zone change clears
    /// the object's own record of who damaged it (CR 400.7), so by resolution
    /// there is nothing left to read (CR 608.2g).
    fn should_trigger_on_creature_dies(&self, _state: &GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, _registry: &CardRegistry) -> bool {
        dead_damaged_by.contains(&self_id)
    }

    fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], dead_toughness: i32, _dead_is_token: bool, _chosen_targets: &[Target], _registry: &CardRegistry) {
        // CR 603.6d: triggered ability resolves even if source has left
        // the battlefield (e.g. simultaneous death in combat).
        let controller = crate::cards::helpers::controller_of(state, self_id);
        // Gain life equal to that creature's toughness (last-known information).
        let toughness = dead_toughness.max(0);
        if toughness > 0 {
            state.change_life(controller, toughness);
            state.log(crate::state::LogLevel::Event,
                format!("Abattoir Ghoul: gained {toughness} life from creature death"));
        }
    }
}
