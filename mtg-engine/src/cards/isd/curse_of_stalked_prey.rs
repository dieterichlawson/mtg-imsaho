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

    /// CR 603.2: "to **enchanted player**" is part of the trigger event, so it
    /// is answered here, as the damage is dealt — not at resolution. Combat
    /// damage to anyone else does not make this ability trigger at all, and so
    /// does not put a do-nothing entry on the stack.
    ///
    /// Note what is *not* here: any condition on who controls the dealer. The
    /// ruling is explicit that "the ability will trigger when any creature
    /// deals combat damage to the enchanted player, including one controlled
    /// by another opponent or even by the enchanted player".
    fn should_trigger_on_damage_to_player(&self, state: &GameState, self_id: ObjectId, _source_id: ObjectId, damaged_player: PlayerId, _registry: &CardRegistry) -> bool {
        state.attached_player(self_id) == Some(damaged_player)
    }

    fn on_any_combat_damage_to_player(&self, state: &mut GameState, _self_id: ObjectId, source_id: ObjectId, _damaged_player: PlayerId, _amount: u32, _registry: &CardRegistry) {
        // Whether this triggered was settled above. What is left is a
        // resolution-time question: CR 121.1, a counter can only be put on a
        // permanent that is still on the battlefield. A creature that dealt
        // its combat damage and died in the same step gets nothing.
        //
        // CR 113.7a: destroying the Curse in response does not counter this —
        // the trigger exists independently of it, and nothing here reads the
        // Curse at all.
        if state.get_object(source_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            state.add_counters(source_id, CounterType::PlusOnePlusOne, 1);
        }
    }
}
