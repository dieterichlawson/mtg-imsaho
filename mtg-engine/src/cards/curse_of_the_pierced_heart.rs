use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Curse of the Pierced Heart — {1}{R} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, Curse of the Pierced Heart
/// deals 1 damage to that player.
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
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, Curse of the Pierced Heart deals 1 damage to that player.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
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

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
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
        // Deal 1 damage to that player.
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
    }
}
