use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, TriggeredAbilityDef, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope};

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
        crate::cards::helpers::equip_for_generic(state, object_id, registry, 1)
    }

    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
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
