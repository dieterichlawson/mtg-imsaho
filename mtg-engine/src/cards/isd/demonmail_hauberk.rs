use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, ContinuousEffect, EffectScope};

/// Demonmail Hauberk — {4} Artifact — Equipment.
/// Equipped creature gets +4/+2.
/// Equip—Sacrifice a creature.
pub struct DemonmailHauberk;

impl CardBehavior for DemonmailHauberk {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Demonmail Hauberk".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
            ])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Equipped creature gets +4/+2.\nEquip—Sacrifice a creature.".into(),
            continuous_effects: vec![
                ContinuousEffect::ModifyPT { power: 4, toughness: 2, scope: EffectScope::Attached },
            ],
            ..Default::default()
        }
    }


    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // Nothing here counts creatures. Whether "Sacrifice a creature" can
        // be paid, and which creatures may pay it, is the engine's question
        // (CR 601.2h) — it enumerates one action per (target, sacrifice) pair
        // and drops the ability when no creature can pay. This used to demand
        // two creatures on the battlefield, on the reasoning that one must be
        // left over to equip. That is not a rule: with a single creature you
        // may still equip it and sacrifice it, and the sacrifice is often the
        // point.
        crate::cards::helpers::equip_ability(state, object_id, registry,
            crate::cards::helpers::EquipCost::SacrificeACreature)
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        // Attach equipment to the target creature.
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }
}
