use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Heretic's Punishment — {4}{R} Enchantment.
/// {3}{R}: Mill three cards, then Heretic's Punishment deals damage to any target
/// equal to the highest mana value among the milled cards.
pub struct HereticsPunishment;

impl CardBehavior for HereticsPunishment {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Heretic's Punishment".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Enchantment],
            oracle_text: "{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        if state.get_object(object_id).is_some_and(|o| o.zone == Zone::Battlefield) {
            vec![ActivatedAbilityDef {
                ability_index: 0,
                description: "{3}{R}: Mill three cards, deal damage equal to highest mana value".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                    ManaSymbol::Colored(Color::Red),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::AnyTarget),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            }]
        } else {
            vec![]
        }
    }

    /// "Choose any target" — and CR 608.2b asks again on resolution whether it
    /// is still one. A permanent that has left the battlefield is not.
    ///
    /// This lives here rather than inside `resolve_activated_ability` because
    /// here is where the engine asks: `stack.rs`'s ability arm calls
    /// `is_valid_target` for each target, substitutes `Target::Illegal`, and
    /// counters the ability outright when every target is illegal — which is
    /// this card's ruling in as many words: "the entire ability won't resolve.
    /// No cards will be put into your graveyard, and no damage will be dealt."
    ///
    /// The card used to answer that question itself, at the top of its
    /// resolution handler. It got the right answer, but a step too late: with
    /// two targets, one of them illegal, the engine calls the handler and the
    /// early return there would have thrown away the legal half.
    ///
    /// Elder of Laurels, Kessig Wolf Run and Silverchase Fox share the pattern
    /// (Kessig Wolf Run has since been moved here too).
    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => state.get_object(*id)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            Target::Player(_) => true,
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        // CR 602.2a: an activated ability's controller is the player who
        // activated it, which the engine records; CR 608.2g falls back to the
        // source's last known controller. Reading `o.controller` here gave the
        // *current* controller, so an opponent taking the permanent in
        // response to the ability collected the effect — and `None => return`
        // threw the whole effect away if the source had left, against
        // CR 113.7a.
        let controller = crate::cards::helpers::ability_controller(state, object_id);

        // Legality was settled before this ran — see `is_valid_target`. An
        // ability with no target at all is not something this card can produce,
        // but the engine hands resolution a slice, so read it as one.
        let Some(damage_target) = targets.first().and_then(|t| match t {
            Target::Object(id) => Some(crate::events::DamageTarget::Object(*id)),
            Target::Player(pid) => Some(crate::events::DamageTarget::Player(*pid)),
            Target::Illegal => None,
        }) else { return };

        // Mill three cards first.
        let player = state.get_player(controller);
        let mill_count = std::cmp::min(3, player.library_order.len());
        let to_mill: Vec<ObjectId> = player.library_order[..mill_count].to_vec();

        // Compute highest mana value among cards to be milled before moving them.
        let mut max_mv: u32 = 0;
        for &card_obj_id in &to_mill {
            let mv = state.face_data(card_obj_id, registry)
                .and_then(|d| d.cost.map(|c| c.mana_value()))
                .unwrap_or(0);
            if mv > max_mv {
                max_mv = mv;
            }
        }

        // "…then mill three cards." Routed through `mill_one` so a creature
        // card among them emits CreatureCardMilled — moving them directly meant
        // an opponent's Undead Alchemist ("whenever a creature card is put into
        // an opponent's graveyard from their library") never saw the three
        // cards this puts into *your* graveyard from *your* library. Which
        // watchers care is the collector's decision, not the miller's.
        let milled: Vec<ObjectId> = state.get_player_mut(controller)
            .library_order.drain(..mill_count).collect();
        for card_id in milled {
            crate::engine::mill_one(state, controller, card_id, registry);
        }

        // Ruling: "If all three cards have a mana value of 0, no damage will be
        // dealt." Nothing is dealt rather than a zero-damage event, which
        // damage watchers would otherwise see.
        if max_mv > 0 {
            crate::damage::deal_damage(state, object_id, damage_target, max_mv,
                crate::damage::DamageKind::NonCombat, registry);
        }

        state.log(crate::state::LogLevel::Event,
            format!("Heretic's Punishment milled {mill_count} cards, dealt {max_mv} damage"));
    }
}
