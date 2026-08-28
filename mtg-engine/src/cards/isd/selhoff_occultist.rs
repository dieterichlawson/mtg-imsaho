use crate::actions::Target;
use crate::cards::{TargetRequirement, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType};

/// Selhoff Occultist — {2}{U} 2/3 Human Rogue.
/// Whenever this creature or another creature dies, target player mills a card.
pub struct SelhoffOccultist;

impl CardBehavior for SelhoffOccultist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Selhoff Occultist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Rogue".into()],
            power: Some(2),
            toughness: Some(3),
            oracle_text: "Whenever this creature or another creature dies, target player mills a card.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "target player mills a card".into(),
                    // CR 603.3b: chosen when the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureDies,
                    description: "target player mills a card".into(),
                    // CR 603.3b: chosen when the trigger goes on the stack.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
            ],
            ..Default::default()
        }
    }

    /// When Selhoff Occultist itself dies, target player mills a card.
    /// CR 603.3b: the target arrived with the trigger, chosen when it went on
    /// the stack — the engine's `process_pending_trigger_pushes` auto-picks a
    /// lone legal target or prompts, and applies hexproof filtering. This used
    /// to build its own prompt at resolution, which also skipped the CR 603.3c
    /// "no legal targets" removal and the CR 608.2b legality re-check.
    fn on_dies(&self, state: &mut GameState, _object_id: ObjectId, chosen_targets: &[Target], registry: &CardRegistry) {
        mill_target(state, chosen_targets, registry);
    }

    /// When another creature dies, target player mills a card.
    ///
    /// No battlefield check on the Occultist. A death trigger fires even when
    /// its watcher died in the same event — a board wipe that kills both puts
    /// the trigger on the stack, and by the time it resolves the Occultist is
    /// in the graveyard. Requiring it to still be on the battlefield made the
    /// trigger a no-op in exactly the case the trigger exists for.
    fn on_any_creature_dies(&self, state: &mut GameState, _self_id: ObjectId, _dead_id: ObjectId, _dead_controller: PlayerId, _dead_damaged_by: &[ObjectId], _dead_toughness: i32, _dead_is_token: bool, chosen_targets: &[Target], registry: &CardRegistry) {
        mill_target(state, chosen_targets, registry);
    }
}

/// "...target player mills a card."
fn mill_target(state: &mut GameState, chosen_targets: &[Target], registry: &CardRegistry) {
    let Some(Target::Player(pid)) = chosen_targets.first() else { return };
    crate::engine::mill_cards(state, *pid, 1, "Selhoff Occultist", registry);
}
