use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggeredAbilityDef, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope, Zone};

/// Wooden Stake — {2} Artifact — Equipment.
/// Equipped creature gets +1/+0.
/// Whenever equipped creature blocks or becomes blocked by a Vampire, destroy that Vampire.
/// Equip {1}.
pub struct WoodenStake;

impl WoodenStake {
    /// "destroy that creature. It can't be regenerated." — `that creature` is
    /// the Vampire, the nearest noun, which the ruling confirms ("The Vampire
    /// is destroyed before any combat damage is dealt").
    fn stake(&self, state: &mut GameState, vampire: ObjectId, registry: &CardRegistry) {
        state.log(crate::state::LogLevel::Event,
            format!("Wooden Stake destroys {} (Vampire)", state.obj_name(vampire)));
        crate::destruction::try_destroy_no_regen(state, vampire, registry);
    }
}

impl CardBehavior for WoodenStake {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Wooden Stake".into(),
            cost: Some(ManaCost::new(vec![ManaSymbol::Generic(2)])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +1/+0.\nWhenever equipped creature blocks or becomes blocked by a Vampire, destroy that creature. It can't be regenerated.\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 1, toughness: 0, scope: EffectScope::Attached },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Blocks,
                    description: "destroy that Vampire".into(),
                target_requirement: None,
                },
                TriggeredAbilityDef {
                    kind: TriggerKind::BecomesBlocked,
                    description: "destroy that Vampire".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // CR 301.5c: an Equipment that is also a creature can't equip. The
        // comment here used to point at Cobbled Wings for the reasoning, which
        // no longer explains it.
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield && !state.is_creature(o.id, registry)) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {1}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(1)]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
                once_per_turn: false,
                sorcery_speed_only: true,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == caster),
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }


    // "Whenever equipped creature blocks or becomes blocked by a Vampire" —
    // the Vampire condition is part of the trigger event (CR 603.2), so it is
    // asked here, once, when the ability would trigger.
    //
    // It is deliberately NOT re-asked on resolution. CR 603.4 re-checks only an
    // intervening-if clause ("..., if ..."), and this ability has none: once it
    // has triggered, "destroy that creature" is unconditional. The resolution
    // handlers used to re-test the subtype, which would have spared a creature
    // that stopped being a Vampire in response — nothing in this pool does
    // that, but the check was a rules error wearing a "defense in depth"
    // comment.
    fn should_trigger_on_blocks(&self, state: &GameState, _self_id: ObjectId, blocked_attacker: ObjectId, registry: &CardRegistry) -> bool {
        state.has_subtype(blocked_attacker, "Vampire", registry)
    }

    fn should_trigger_on_becomes_blocked(&self, state: &GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) -> bool {
        state.has_subtype(blocker_id, "Vampire", registry)
    }

    fn on_blocks(&self, state: &mut GameState, _self_id: ObjectId, other_creature: ObjectId, registry: &CardRegistry) {
        self.stake(state, other_creature, registry);
    }

    fn on_becomes_blocked(&self, state: &mut GameState, _self_id: ObjectId, blocker_id: ObjectId, registry: &CardRegistry) {
        self.stake(state, blocker_id, registry);
    }
}
