use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, LogLevel, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Keyword};
use crate::actions::Target;

/// Bitterheart Witch — {4}{B} 1/2 Human Shaman with Deathtouch.
/// When Bitterheart Witch dies, you may search your library for a Curse card,
/// put it onto the battlefield attached to target player, then shuffle.
pub struct BitterheartWitch;

impl CardBehavior for BitterheartWitch {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Bitterheart Witch".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Shaman".into()],
            power: Some(1),
            toughness: Some(2),
            oracle_text: "Deathtouch\nWhen this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.".into(),
            keywords: vec![Keyword::Deathtouch],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::SelfDies,
                    description: "search library for a Curse card".into(),
                    // CR 603.3d: "attached to **target player**" — the target
                    // is chosen as the trigger goes on the stack, before the
                    // search, so an opponent responding knows whom it will hit
                    // and CR 608.2b re-checks it on resolution.
                    target_requirement: Some(TargetRequirement::PlayerOnly),
                },
            ],
            ..Default::default()
        }
    }

    fn on_dies(&self, state: &mut GameState, object_id: ObjectId, chosen_targets: &[Target], _registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, object_id);

        // The player was targeted when the trigger went on the stack
        // (CR 603.3d). Remember which, so the search below knows where the
        // Curse is going — the choice chain runs entirely inside this one
        // resolution and cannot ask again.
        let Some(Target::Player(pid)) = chosen_targets.first() else { return };
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.card_state.insert("curse_target".into(), ObjectId(u64::from(pid.0)));
        }

        // "you may" — present a yes/no choice before searching.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: object_id,
            choice: ResolutionChoiceKind::YesNo {
                description: "Bitterheart Witch: search your library for a Curse card?".into(),
                source_card: object_id,
            },
        });
    }


    fn on_yes_no_choice(&self, state: &mut GameState, self_id: ObjectId, yes: bool, registry: &CardRegistry) {
        use rand::seq::SliceRandom;

        if !yes {
            return;
        }

        let controller = crate::cards::helpers::controller_of(state, self_id);

        // Search library for Curse cards.
        let curse_ids: Vec<ObjectId> = state.get_player(controller).library_order.iter()
            .filter(|&&obj_id| {
                state.has_subtype(obj_id, "Curse", registry)
            })
            .copied()
            .collect();

        // The shuffle happens here rather than after the Curse is found, so
        // that declining to find one still shuffles. "…then shuffle" is part of
        // the search, not of finding: a player who searched and found nothing
        // has still shuffled. Doing it before removing the Curse rather than
        // after is unobservable — the result is a uniformly random order either
        // way.
        let mut rng = rand::thread_rng();
        state.get_player_mut(controller).library_order.shuffle(&mut rng);

        if curse_ids.is_empty() {
            state.log(LogLevel::Event,
                "Bitterheart Witch: no Curse found in library".to_string());
            return;
        }

        // CR 701.19b: searching a hidden zone never forces you to find. Even
        // with exactly one Curse in the library the player may decline it, so
        // the choice is offered — and offered as optional — rather than taken
        // for them.
        let curse_targets: Vec<crate::actions::Target> = curse_ids.iter()
            .map(|&id| crate::actions::Target::Object(id))
            .collect();
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: controller,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Bitterheart Witch: choose a Curse card from your library".into(),
                options: curse_targets,
                optional: true,
                effect: PendingEffect::CardEffect {
                    source_id: self_id,
                    key: "choose".into(),
                },
            },
        });
    }

    /// "When this creature dies, you may search your library for a Curse card,
    /// put it onto the battlefield attached to target player, then shuffle."
    /// The player was targeted when the trigger went on the stack; the only
    /// choice left inside the resolution is *which* Curse.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, _key: &str, target: &Target, registry: &CardRegistry) {
        let Target::Object(curse_id) = target else { return };
        Self::attach_curse(state, source_id, *curse_id, registry);
    }
}

impl BitterheartWitch {
    /// The player this trigger targeted, stashed by `on_dies`.
    fn curse_target(state: &GameState, self_id: ObjectId) -> Option<PlayerId> {
        state.get_object(self_id)?
            .card_state.get("curse_target")
            .map(|id| PlayerId(u8::try_from(id.0).unwrap_or(u8::MAX)))
    }

    /// Put the found Curse onto the battlefield attached to the targeted
    /// player. The shuffle is not here: it belongs to the search, which
    /// happens whether or not a Curse is found.
    fn attach_curse(state: &mut GameState, self_id: ObjectId, curse_id: ObjectId, registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let name = state.obj_name(curse_id);

        if let Some(pid) = Self::curse_target(state, self_id) {
            // CR 303.4h: an Aura that would enter attached to something it
            // can't legally enchant doesn't enter the battlefield — it stays
            // where it is. Ruling: "The Curse must be legally able to enchant
            // the player. For example, if the player has protection from red,
            // you couldn't put a red Curse onto the battlefield this way."
            // The shuffle below still happens; the search did.
            if state.player_can_be_enchanted_by(curse_id, pid, registry) {
                state.get_player_mut(controller).library_order.retain(|&id| id != curse_id);
                state.move_object(curse_id, crate::types::Zone::Battlefield, registry);
                if let Some(obj) = state.get_object_mut(curse_id) {
                    obj.attached_to_player = Some(pid);
                    obj.summoning_sick = false;
                }
                state.log(crate::state::LogLevel::Event,
                    format!("Bitterheart Witch: attached {name} to p{}", pid.0));
            } else {
                state.log(crate::state::LogLevel::Event,
                    format!("Bitterheart Witch: {name} can't enchant p{} and stays in the library", pid.0));
            }
        }
    }
}
