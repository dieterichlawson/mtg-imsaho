use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, TargetRequirement, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
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
        // Exile up to 2 cards from their graveyard.
        // Auto-choose: exile the first 2 found.
        let to_exile: Vec<ObjectId> = state.objects_in_zone(Zone::Graveyard, cursed_player)
            .iter()
            .take(2)
            .map(|o| o.id)
            .collect();
        let count = to_exile.len();
        for id in to_exile {
            state.move_object(id, Zone::Exile);
        }
        if count > 0 {
            state.log(crate::state::LogLevel::Event,
                format!("Curse of Oblivion: p{} exiled {} card(s) from graveyard", cursed_player.0, count));
        }
    }
}
