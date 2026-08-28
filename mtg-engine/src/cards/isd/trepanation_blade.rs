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


    /// CR 702.6b: equip attaches to "target creature you control".
    fn is_valid_target(&self, state: &GameState, caster: PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        crate::cards::helpers::equip_target_is_legal(state, caster, target, registry)
    }

    fn on_attacks(&self, state: &mut GameState, _self_id: ObjectId, attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        // "Whenever equipped creature attacks, defending player reveals cards
        // from the top of their library until they reveal a land card. The
        // creature gets +1/+0 until end of turn for each card revealed this
        // way. That player puts the revealed cards into their graveyard."
        //
        // The bonus counts cards **revealed**, not cards that reached the
        // graveyard — the card was errata'd from the latter, and the comment
        // here still quoted the printed wording. Nothing in this set can
        // separate the two counts, but the distinction is the card's, so the
        // count is named for what it counts.
        //
        // Ruling: "The land card is counted when calculating the bonus, and it
        // will be put into the graveyard with the other revealed cards" — so
        // the land is revealed, counted, and milled before the loop stops.
        //
        // Both halves are about the attack that happened, so they read the
        // snapshot rather than the Blade's current `attached_to`. Killing the
        // equipped creature in response used to cancel the mill as well as the
        // buff, and re-equipping the Blade before the trigger resolved moved
        // the buff onto a creature that never attacked.
        let creature_id = attack.attacker;
        let defending_player = attack.defending_player;

        // Reveal cards from defending player's library until a land is revealed.
        let mut cards_revealed = 0;
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

            // "That player puts the revealed cards into their graveyard" —
            // library to graveyard, so it is a mill.
            crate::engine::mill_one(state, defending_player, card_id, registry);
            cards_revealed += 1;

            if is_land {
                break; // Stop after revealing a land.
            }
        }

        if cards_revealed > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Trepanation Blade: p{} revealed and milled {} card{}", defending_player.0, cards_revealed,
                    if cards_revealed == 1 { "" } else { "s" }));

            // The buff needs a creature to land on: skipped if the attacker
            // died, and it goes on the creature that attacked even if the
            // Blade has since moved elsewhere.
            if state.get_object(creature_id).is_some_and(|o| o.zone == Zone::Battlefield) {
                state.until_end_of_turn.push(
                    crate::state::TemporaryEffect::ModifyPT {
                        target: creature_id,
                        power_mod: cards_revealed,
                        toughness_mod: 0,
                    }
                );
                state.log(crate::state::LogLevel::Event,
                    format!("Trepanation Blade: equipped creature gets +{cards_revealed}/+0"));
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

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_equip(state, object_id, targets, registry);
    }
}
