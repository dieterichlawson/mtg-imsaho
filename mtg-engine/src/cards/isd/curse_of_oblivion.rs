use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::{AwaitingAction, GameState, PendingEffect, ResolutionChoiceKind};
use crate::types::*;

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
                },
            ],
        }
    }

    fn target_requirement(&self) -> TargetRequirement {
        TargetRequirement::PlayerOnly
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
        crate::cards::helpers::resolve_curse(state, object_id, targets);
    }

    fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let cursed_player = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.attached_to_player,
            _ => return,
        };
        let cursed_player = match cursed_player {
            Some(p) => p,
            None => return,
        };
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
                .map(|t| match t { Target::Object(id) => *id, _ => unreachable!() })
                .collect();
            let count = to_exile.len();
            for id in to_exile {
                state.move_object(id, Zone::Exile);
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
                effect: PendingEffect::ExileCurseOfOblivion { remaining: 1 },
            },
        });
    }
}
