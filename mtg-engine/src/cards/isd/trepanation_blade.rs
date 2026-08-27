use crate::actions::Target;
use crate::cards::{AttackInfo, ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Zone};

/// Trepanation Blade — {3} Artifact — Equipment.
/// Whenever equipped creature attacks, defending player reveals cards from the top
/// of their library until they reveal a land card. The creature gets +1/+0 until end
/// of turn for each card revealed this way. That player puts the revealed cards into
/// their graveyard.
/// Equip {2}.
pub struct TrepanationBlade;

impl CardBehavior for TrepanationBlade {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Trepanation Blade".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
            ])),
            card_types: vec![CardType::Artifact],
            subtypes: vec!["Equipment".into()],
            oracle_text: "Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard.\nEquip {2}".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "mill until land, equipped creature gets +1/+0 per card milled".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_equipment = true;
        }
    }

    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry) && o.controller == caster)
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn on_attacks(&self, state: &mut GameState, _self_id: ObjectId, attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        // "Whenever equipped creature attacks, defending player reveals cards
        // from the top of their library until they reveal a land card. That
        // player puts those cards into their graveyard. Equipped creature gets
        // +1/+0 until end of turn for each card put into that player's
        // graveyard this way."
        //
        // Both halves are about the attack that happened, so they read the
        // snapshot rather than the Blade's current `attached_to`. Killing the
        // equipped creature in response used to cancel the mill as well as the
        // buff, and re-equipping the Blade before the trigger resolved moved
        // the buff onto a creature that never attacked.
        let creature_id = attack.attacker;
        let defending_player = attack.defending_player;

        // Reveal cards from defending player's library until a land is revealed.
        let mut cards_milled = 0;
        loop {
            let card_id = {
                let player = state.get_player(defending_player);
                if player.library_order.is_empty() {
                    break;
                }
                player.library_order[0]
            };

            // Check if this card is a land via the registry (object card_types
            // may be empty for non-token cards).
            let is_land = state.get_object(card_id)
                .and_then(|o| state.face_data(o.id, registry))
                .is_some_and(|d| d.card_types.iter().any(|ct| matches!(ct, CardType::Land)));

            // Remove from library and put into graveyard.
            let player = state.get_player_mut(defending_player);
            player.library_order.remove(0);
            state.move_object(card_id, Zone::Graveyard, registry);
            cards_milled += 1;

            if is_land {
                break; // Stop after revealing a land.
            }
        }

        if cards_milled > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Trepanation Blade: p{} milled {} card{}", defending_player.0, cards_milled,
                    if cards_milled == 1 { "" } else { "s" }));

            // The buff needs a creature to land on: skipped if the attacker
            // died, and it goes on the creature that attacked even if the
            // Blade has since moved elsewhere.
            if state.get_object(creature_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: creature_id,
                        power_mod: cards_milled,
                        toughness_mod: 0,
                    }
                );
                state.log(crate::state::LogLevel::Event,
                    format!("Trepanation Blade: equipped creature gets +{cards_milled}/+0"));
            }
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let Some(obj) = state.get_object(object_id) else { return vec![]; };
        if obj.zone == Zone::Battlefield && !state.is_creature(obj.id, registry) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "Equip {2}".into(),
                cost: ManaCost::new(vec![ManaSymbol::Generic(2)]),
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

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        if let Some(Target::Object(creature_id)) = targets.first() {
            if let Some(obj) = state.get_object_mut(object_id) {
                obj.attached_to = Some(*creature_id);
            }
        }
    }
}
