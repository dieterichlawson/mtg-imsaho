use crate::actions::Target;
use crate::cards::{AttackInfo, ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{GameState, PendingEffect};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, ContinuousEffect, EffectScope, Zone, CounterType};

/// Grimgrin, Corpse-Born {3}{U}{B} 5/5 Legendary Zombie Warrior.
/// Grimgrin enters tapped and doesn't untap during your untap step.
/// Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
/// Whenever Grimgrin attacks, destroy target creature defending player controls,
/// then put a +1/+1 counter on Grimgrin.
pub struct GrimgrinCorpseBorn;

impl CardBehavior for GrimgrinCorpseBorn {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Grimgrin, Corpse-Born".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Zombie".into(), "Warrior".into()],
            power: Some(5),
            toughness: Some(5),
            oracle_text: "Grimgrin enters tapped and doesn't untap during your untap step.\nSacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.\nWhenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.".into(),
            continuous_effects: vec![
                ContinuousEffect::PreventUntap {
                    scope: EffectScope::OnSelf,
                },
            ],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "destroy target creature defending player controls, then +1/+1 counter".into(),
                    // CR 603.3d: target chosen as the trigger goes on the stack.
                    // Use Creature + is_valid_target to filter to the defending player's creatures.
                    target_requirement: Some(TargetRequirement::Creature),
                },
            ],
            ..Default::default()
        }
    }

    /// "Grimgrin enters tapped" is a replacement effect (CR 614.1c), not
    /// something to do to it after it has arrived.
    ///
    /// This used to override `on_resolve` to `move_object` and then set
    /// `tapped = true`. `move_object` emits `EnteredBattlefield` as part of the
    /// move, so every ETB watcher saw an untapped Grimgrin and the tap happened
    /// afterwards — the ordering CR 614.1c exists to prevent, and the same
    /// override also re-did the trait default's "move a permanent to the
    /// battlefield" and its `is_legendary` stamping.
    fn replace_event(
        &self,
        _state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        crate::cards::helpers::enters_tapped_unless(self_id, event, || false)
    }

    /// Filter the attack trigger's targets to creatures the defending
    /// player controls. The activated ability is untargeted, so this
    /// filter only applies to the trigger's `TargetRequirement::Creature`.
    /// `caster` is the controller of the ability (Grimgrin's controller).
    /// The defending player is whoever Grimgrin is attacking — read from
    /// combat state and fall back to the opponent of `caster`.
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        let Target::Object(id) = target else { return false; };
        let Some(obj) = state.get_object(*id) else { return false; };
        if obj.zone != Zone::Battlefield || !state.is_creature(obj.id, registry) {
            return false;
        }
        // Determine the defending player: any of the caster's attackers
        // has a defender entry in combat.attackers. If there are multiple
        // attackers from the caster, any of their defenders is acceptable
        // (in practice all attack the same defender in a 2-player game).
        let defender = state.combat.as_ref()
            .and_then(|c| c.attackers.iter()
                .find_map(|(atk, def)| {
                    state.get_object(*atk).and_then(|a| {
                        if a.controller == caster { Some(*def) } else { None }
                    })
                }))
            .unwrap_or_else(|| state.opponent(caster));
        obj.controller == defender
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "Sacrifice another creature: Untap Grimgrin, +1/+1 counter".into(),
            cost: ManaCost::free(),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeAnotherCreature,
            target_requirement: None,
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, _targets: &[Target], _registry: &CardRegistry) {
        // The engine already sacrificed another creature as part of paying the cost.
        // Now untap Grimgrin and add a +1/+1 counter.
        state.untap(object_id);
        state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
        state.log(crate::state::LogLevel::Event,
            "Grimgrin: sacrificed creature, untapped, +1/+1 counter".into());
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _attack: AttackInfo, chosen_targets: &[Target], registry: &CardRegistry) {
        // CR 603.3d: target was chosen when the trigger went on the stack.
        // Per ruling: "If the defending player controls no creatures when Grimgrin attacks,
        // the last ability will be removed from the stack and have no effect."
        // That removal is handled by process_pending_trigger_pushes (no legal targets).
        // When we reach here, we have a target to destroy.
        let Some(target) = chosen_targets.first() else { return };
        let effect = PendingEffect::CardEffect { source_id: self_id, key: String::new() };
        crate::engine::apply_pending_effect(state, target, &effect, registry);
    }

    /// "Whenever this creature attacks, destroy target creature. Put a +1/+1
    /// counter on this creature." The counter goes on regardless of whether
    /// the destruction succeeds (indestructible, regeneration) — that is this
    /// card's wording, not a general rule.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let name = state.obj_name(*id);
        // The ruling this card is most often asked about: "if the targeted
        // creature isn't destroyed (perhaps because it regenerated or has
        // indestructible), you'll still put a +1/+1 counter on Grimgrin". So the
        // counter is unconditional — but the log must not claim a destruction
        // that did not happen, or a player reading it back cannot tell the two
        // cases apart.
        let outcome = crate::destruction::try_destroy(state, *id, registry);
        state.log(crate::state::LogLevel::Event, match outcome {
            crate::destruction::DestroyResult::Died =>
                format!("Grimgrin, Corpse-Born destroyed {name}"),
            crate::destruction::DestroyResult::Indestructible =>
                format!("Grimgrin, Corpse-Born could not destroy {name} (indestructible)"),
            crate::destruction::DestroyResult::Regenerated =>
                format!("Grimgrin, Corpse-Born destroyed {name}, but it regenerated"),
        });
        state.add_counters(source_id, crate::types::CounterType::PlusOnePlusOne, 1);
        state.log(crate::state::LogLevel::Event,
            "Grimgrin, Corpse-Born: +1/+1 counter from attack trigger".into());
    }
}
