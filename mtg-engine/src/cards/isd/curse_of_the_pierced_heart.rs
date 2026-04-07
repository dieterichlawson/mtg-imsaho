use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "deal 1 damage to enchanted player".into(),
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets, registry);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let (controller, cursed_player) = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => (o.controller, o.attached_to_player),
            _ => return,
        };
        let cursed_player = match cursed_player {
            Some(p) => p,
            None => return,
        };
        // Only trigger on the enchanted player's upkeep.
        if state.active_player != cursed_player {
            return;
        }

        // Check if the cursed player controls any planeswalkers.
        let planeswalkers: Vec<Target> = state.objects.values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == cursed_player
                && o.card_types.contains(&CardType::Planeswalker))
            .map(|o| Target::Object(o.id))
            .collect();

        if planeswalkers.is_empty() {
            // No planeswalkers — deal damage directly to the player.
            let old = state.get_player(cursed_player).life;
            let new_life = old - 1;
            state.get_player_mut(cursed_player).life = new_life;
            state.events.push(crate::events::GameEvent::NonCombatDamageDealt {
                source: self_id,
                target: crate::events::DamageTarget::Player(cursed_player),
                amount: 1,
            });
            state.events.push(crate::events::GameEvent::LifeChanged { player: cursed_player, old, new_life });
            state.log(crate::state::LogLevel::Event,
                format!("Curse of the Pierced Heart dealt 1 damage to p{}", cursed_player.0));
        } else {
            // Planeswalkers present — Curse controller chooses target.
            let mut options = vec![Target::Player(cursed_player)];
            options.extend(planeswalkers);
            state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
                player: controller,
                source: self_id,
                choice: ResolutionChoiceKind::ChooseTarget {
                    description: "Curse of the Pierced Heart: deal 1 damage to player or a planeswalker they control".into(),
                    options,
                    optional: false,
                    effect: PendingEffect::DealDamage {
                        amount: 1,
                        source_id: self_id,
                        source_name: "Curse of the Pierced Heart".into(),
                    },
                },
            });
        }
    }
}
