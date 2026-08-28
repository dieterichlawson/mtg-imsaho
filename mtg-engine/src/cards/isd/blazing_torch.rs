use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::events::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, CreatureFilter, EffectScope, Zone};

/// Blazing Torch — {1} Artifact — Equipment.
/// Equipped creature can't be blocked by Vampires or Zombies.
/// Equipped creature has "{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage
/// to any target."
/// Equip {1}.
pub struct BlazingTorch;

impl CardBehavior for BlazingTorch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Blazing Torch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
            ])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature can't be blocked by Vampires or Zombies.\nEquipped creature has \"{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            continuous_effects: vec![
                ContinuousEffect::CanOnlyBeBlockedBy {
                    allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::Or(vec![
                        CreatureFilter::HasSubtype("Vampire".into()),
                        CreatureFilter::HasSubtype("Zombie".into()),
                    ]))),
                    scope: EffectScope::Attached,
                },
            ],
            ..Default::default()
        }
    }


    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            Target::Player(pid) => !state.get_player(*pid).lost,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };

        if obj.zone != Zone::Battlefield {
            return vec![];
        }

        if !state.is_creature(obj.id, registry) {
            // Called with the equipment's own ID — return the equip ability.
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
            // Called with a creature's ID (the equipment is attached to it).
            // Grant the creature "{T}, Sacrifice Blazing Torch: deal 2 damage to any target."
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{T}, Sacrifice Blazing Torch: Deal 2 damage to any target".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                // The Torch is not the object the ability is activated on, so
                // `SacrificeCost` cannot say it; `pay_activation_cost` does.
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        }
    }

    /// "{T}, Sacrifice Blazing Torch:" — the sacrifice is a cost, so it is paid
    /// on activation (CR 601.2h via 602.2b) and an opponent responding to the
    /// ability already sees the Torch in the graveyard.
    fn pay_activation_cost(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, _targets: &[Target], registry: &CardRegistry) {
        if ability_index == 1 {
            if let Some(torch) = attached_torch(state, object_id, registry) {
                crate::destruction::sacrifice(state, torch, registry);
            }
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if ability_index == 0 {
            // Equip: attach to target creature.
            crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
        } else if ability_index == 1 {
            // Per ruling: "The source of the damage is Blazing Torch, not the
            // equipped creature." The Torch was sacrificed to pay the cost, so
            // it is found by the `last_attached_to` the engine records on every
            // zone change — last known information, CR 608.2g.
            let damage_source = sacrificed_torch(state, object_id, registry).unwrap_or(object_id);

            if let Some(target) = targets.first() {
                let damage_target = match target {
                    Target::Object(target_id) => DamageTarget::Object(*target_id),
                    Target::Player(player_id) => DamageTarget::Player(*player_id),
                    // CR 608.2b: a target that is no longer legal is not
                    // dealt damage at all.
                    Target::Illegal => return,
                };
                crate::damage::deal_damage(state, damage_source, damage_target, 2,
                    crate::damage::DamageKind::NonCombat, registry);
            }
        }
    }
}

/// The Blazing Torch attached to `creature_id` on the battlefield.
fn attached_torch(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Option<ObjectId> {
    let torch_card_id = registry.get_id_by_name("Blazing Torch")?;
    state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .find(|o| o.attached_to == Some(creature_id)
            && o.card_id == torch_card_id)
        .map(|o| o.id)
}

/// The Blazing Torch that was attached to `creature_id` before it was
/// sacrificed to pay this ability's cost.
fn sacrificed_torch(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Option<ObjectId> {
    let torch_card_id = registry.get_id_by_name("Blazing Torch")?;
    // In id order rather than map order: with two Torches that were both
    // attached to this creature at some point, `find` must not pick a
    // different one on a replay of the same game.
    state.objects_in_id_order().into_iter()
        .find(|o| o.card_id == torch_card_id
            && o.card_state.get("last_attached_to") == Some(&creature_id))
        .map(|o| o.id)
}
