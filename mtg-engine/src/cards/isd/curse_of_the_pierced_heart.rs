use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Curse of the Pierced Heart — {1}{R} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, this Aura deals 1 damage
/// to that player or a planeswalker that player controls.
pub struct CurseOfThePiercedHeart;

impl CardBehavior for CurseOfThePiercedHeart {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of the Pierced Heart".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into(), "Curse".into()],
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "deal 1 damage to enchanted player".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets, registry);
    }

    /// "At the beginning of ENCHANTED PLAYER's upkeep" — CR 603.2: the trigger
    /// event is that player's upkeep beginning, so it must not go on the stack
    /// during anyone else's.
    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::AttachedPlayer,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 113.7a: destroying the Curse in response does not counter its
        // trigger, and `attached_player` still knows whom it cursed.
        //
        // Whose upkeep this is was settled when the trigger was collected —
        // `TriggerScope::AttachedPlayer` in `triggers/collect/timing.rs`
        // implements CR 603.2 for every Curse at once. Re-checking it here
        // would also be wrong: once the ability is on the stack it resolves
        // whatever the turn has done since.
        let Some(controller) = state.get_object(self_id).map(|o| o.controller) else { return };
        let Some(cursed_player) = state.attached_player(self_id) else { return };

        // Check if the cursed player controls any planeswalkers.
        // `obj.card_types` is empty for non-token permanents, so reading it
        // directly made "or a planeswalker that player controls" dead code for
        // every real planeswalker. `has_card_type` reads the active face.
        let planeswalkers: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == cursed_player)
            .map(|o| o.id)
            .filter(|id| state.has_card_type(*id, CardType::Planeswalker, registry))
            .map(Target::Object)
            .collect();

        // "…deals 1 damage to that player **or** a planeswalker that player
        // controls." One effect either way; only the number of options differs.
        //
        // The no-planeswalker branch used to write the life total by hand —
        // `state.get_player_mut(cursed_player).life = new_life` plus its own
        // NonCombatDamageDealt and LifeChanged events — so the ordinary case,
        // the one that happens every game, skipped `damage::deal_damage` and
        // everything it applies: protection, prevention, damage multipliers,
        // and the lifelink and damage watchers that key on the pipeline.
        let effect = PendingEffect::DealDamage {
            amount: 1,
            source_id: self_id,
            source_name: "Curse of the Pierced Heart".into(),
        };
        let mut options = vec![Target::Player(cursed_player)];
        options.extend(planeswalkers);

        if options.len() == 1 {
            crate::engine::apply_pending_effect(state, &options[0], &effect, registry);
        } else {
            // The *Curse's* controller chooses, not the cursed player.
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Curse of the Pierced Heart: deal 1 damage to player or a planeswalker they control".into(),
                    options,
                    optional: false,
                    effect,
                },
            });
        }
    }
}
