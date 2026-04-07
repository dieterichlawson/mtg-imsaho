use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::events::{DamageTarget, GameEvent};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

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
            supertypes: vec![],
            subtypes: vec!["Equipment".into()],
            power: None,
            toughness: None,
            oracle_text: "Equipped creature can't be blocked by Vampires or Zombies.\nEquipped creature has \"{T}, Sacrifice Blazing Torch: Blazing Torch deals 2 damage to any target.\"\nEquip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![
                ContinuousEffect::CanOnlyBeBlockedBy {
                    allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::Or(vec![
                        CreatureFilter::HasSubtype("Vampire".into()),
                        CreatureFilter::HasSubtype("Zombie".into()),
                    ]))),
                    scope: EffectScope::Attached,
                },
            ],
            additional_cost: None, triggered_abilities: vec![],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn is_valid_target(&self, state: &GameState, _caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .map(|o| o.zone == Zone::Battlefield)
                .unwrap_or(false),
            Target::Player(pid) => !state.get_player(*pid).lost,
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) => o,
            None => return vec![],
        };

        if obj.zone != Zone::Battlefield {
            return vec![];
        }

        if obj.power.is_none() {
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
            }]
        } else {
            // Called with a creature's ID (the equipment is attached to it).
            // Grant the creature "{T}, Sacrifice Blazing Torch: deal 2 damage to any target."
            vec![ActivatedAbilityDef {
                ability_index: 1,
                description: "{T}, Sacrifice Blazing Torch: Deal 2 damage to any target".into(),
                cost: ManaCost::free(),
                requires_tap: true,
                sacrifice_cost: SacrificeCost::None, // Torch sacrifice handled manually in on_activate_ability.
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
            }]
        }
    }

    fn on_activate_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if ability_index == 0 {
            // Equip: attach to target creature.
            if let Some(Target::Object(creature_id)) = targets.first() {
                if let Some(obj) = state.get_object_mut(object_id) {
                    obj.attached_to = Some(*creature_id);
                }
            }
        } else if ability_index == 1 {
            // Damage ability: object_id is the creature. Find the attached Blazing Torch.
            let torch_card_id = registry.get_id_by_name("Blazing Torch");
            let torch_id = state.objects.values()
                .find(|o| {
                    o.zone == Zone::Battlefield
                        && o.is_equipment
                        && o.attached_to == Some(object_id)
                        && torch_card_id.map(|tc| o.card_id == tc).unwrap_or(false)
                })
                .map(|o| o.id);

            // Sacrifice the torch, but keep its ID for damage source attribution.
            // Per ruling: "The source of the damage is Blazing Torch, not the equipped creature."
            let damage_source = torch_id.unwrap_or(object_id);
            if let Some(torch) = torch_id {
                crate::destruction::sacrifice(state, torch, registry);
            }

            // Deal 2 damage to the target. Source is the torch, not the creature.
            if let Some(target) = targets.first() {
                match target {
                    Target::Object(target_id) => {
                        if let Some(obj) = state.get_object_mut(*target_id) {
                            obj.damage_marked += 2;
                            obj.damaged_by.push(damage_source);
                        }
                        state.events.push(GameEvent::NonCombatDamageDealt {
                            source: damage_source,
                            target: DamageTarget::Object(*target_id),
                            amount: 2,
                        });
                    }
                    Target::Player(player_id) => {
                        let old_life = state.get_player(*player_id).life;
                        let new_life = old_life - 2;
                        state.get_player_mut(*player_id).life = new_life;
                        state.events.push(GameEvent::NonCombatDamageDealt {
                            source: damage_source,
                            target: DamageTarget::Player(*player_id),
                            amount: 2,
                        });
                        state.events.push(GameEvent::LifeChanged {
                            player: *player_id,
                            old: old_life,
                            new_life,
                        });
                    }
                }
                state.log(crate::state::LogLevel::Event,
                    "Blazing Torch deals 2 damage".into());
            }
        }
    }
}
