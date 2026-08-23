use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Curse of Oblivion — {3}{B} Enchantment — Aura Curse.
/// Enchant player.
/// At the beginning of enchanted player's upkeep, that player exiles two cards
/// from their graveyard.
pub struct CurseOfOblivion;

impl CardBehavior for CurseOfOblivion {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Curse of Oblivion".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Enchantment],
            supertypes: vec![],
            subtypes: vec!["Aura".into(), "Curse".into()],
            power: None,
            toughness: None,
            oracle_text: "Enchant player\nAt the beginning of enchanted player's upkeep, that player exiles two cards from their graveyard.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Upkeep,
                    description: "enchanted player exiles two cards from their graveyard".into(),
                target_requirement: None,
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets, registry);
    }

    /// "At the beginning of ENCHANTED PLAYER's upkeep" — CR 603.2: the trigger
    /// event is that player's upkeep beginning, so it must not go on the stack
    /// during anyone else's.
    fn step_trigger_scope(&self, kind: &TriggerKind, _is_back_face: bool) -> crate::cards::TriggerScope {
        match kind {
            TriggerKind::Upkeep => crate::cards::TriggerScope::AttachedPlayer,
            _ => crate::cards::TriggerScope::Each,
        }
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _chosen_targets: &[Target], registry: &CardRegistry) {
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
            _ => return,
        };
        let Some(cursed_player) = cursed_player else { return; };
        if state.active_player != cursed_player {
            return;
        }
        // "That player exiles two cards from their graveyard."
        // The cursed player chooses which cards to exile.
        let gy_cards: Vec<Target> = state.objects_in_zone(Zone::Graveyard, cursed_player)
            .iter()
            .map(|o| Target::Object(o.id))
            .collect();
        if gy_cards.is_empty() {
            return;
        }
        // If 2 or fewer cards, just exile them all — no choice needed.
        if gy_cards.len() <= 2 {
            let to_exile: Vec<ObjectId> = gy_cards.iter()
                .map(|t| match t { Target::Object(id) => *id, Target::Player(_) => unreachable!() })
                .collect();
            let count = to_exile.len();
            for id in to_exile {
                state.move_object(id, Zone::Exile, registry);
            }
            state.log(crate::state::LogLevel::Event,
                format!("Curse of Oblivion: p{} exiled {} card(s) from graveyard", cursed_player.0, count));
            return;
        }
        // Present choice: cursed player picks a card to exile (will need to pick again for second).
        // For now, present as a mandatory single-target choice.
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: cursed_player,
            source: self_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Curse of Oblivion: choose a card to exile from your graveyard".into(),
                options: gy_cards,
                optional: false,
                effect: PendingEffect::CardEffect { source_id: self_id, key: "1".into() },
            },
        });
    }

    /// "At the beginning of enchanted player's upkeep, that player exiles two
    /// cards from their graveyard." The chained second choice is this card's
    /// own loop; `key` carries how many exiles are still owed.
    fn resolve_card_effect(&self, state: &mut GameState, source_id: ObjectId, key: &str, target: &Target, _registry: &CardRegistry) {
        let Target::Object(id) = target else { return };
        let owner = state.get_object(*id).map_or(crate::ids::PlayerId(0), |o| o.owner);
        state.move_object(*id, Zone::Exile, _registry);
        state.log(crate::state::LogLevel::Event,
            format!("Curse of Oblivion: exiled a card from p{}'s graveyard", owner.0));

        let remaining: u32 = key.parse().unwrap_or(0);
        if remaining == 0 {
            return;
        }
        let gy_cards: Vec<Target> = state.objects_in_zone(Zone::Graveyard, owner)
            .iter()
            .map(|o| Target::Object(o.id))
            .collect();
        if gy_cards.is_empty() {
            return;
        }
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: owner,
            source: source_id,
            choice: ResolutionChoiceKind::ChooseTarget {
                description: "Curse of Oblivion: choose another card to exile".into(),
                options: gy_cards,
                optional: false,
                effect: PendingEffect::CardEffect {
                    source_id,
                    key: (remaining - 1).to_string(),
                },
            },
        });
    }
}
