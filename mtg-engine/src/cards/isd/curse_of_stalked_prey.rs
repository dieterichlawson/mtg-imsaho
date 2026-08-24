use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone, CounterType};

/// Curse of Stalked Prey — {1}{R} Enchantment — Aura Curse.
/// Enchant player.
/// Whenever a creature deals combat damage to enchanted player,
/// put a +1/+1 counter on that creature.
pub struct CurseOfStalkedPrey;

impl CardBehavior for CurseOfStalkedPrey {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of Stalked Prey".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            subtypes: vec!["Aura".into(), "Curse".into()],
            oracle_text: "Enchant player\nWhenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCombatDamageToPlayer,
                    description: "put a +1/+1 counter on that creature".into(),
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

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, self_id: ObjectId, source_id: ObjectId, damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // Only trigger when the enchanted player is dealt damage.
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
            _ => return,
        };
        if cursed_player != Some(damaged_player) {
            return;
        }
        // Put a +1/+1 counter on the creature that dealt damage.
        if state.get_object(source_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.add_counters(source_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}
