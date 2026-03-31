use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::*;

/// Geist of Saint Traft {1}{W}{U} 2/2 Legendary Spirit Cleric with Hexproof.
/// Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token
/// with flying that's tapped and attacking. Exile that token at end of combat.
pub struct GeistOfSaintTraft;

impl CardBehavior for GeistOfSaintTraft {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Geist of Saint Traft".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Spirit".into(), "Cleric".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "Hexproof\nWhenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.".into(),
            keywords: vec![Keyword::Hexproof],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "create a 4/4 Angel token tapped and attacking".into(),
                },
            ],
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[crate::actions::Target], _registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_legendary = true;
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };

        // Create a 4/4 Angel token with flying, tapped and attacking.
        let token_id = state.create_token_with_subtypes(
            "Angel",
            controller,
            4, 4,
            vec![Color::White],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Angel".into()],
        );

        // Set the token as tapped and attacking.
        if let Some(obj) = state.get_object_mut(token_id) {
            obj.tapped = true;
            obj.summoning_sick = false; // It's attacking, so summoning sickness doesn't matter.
        }

        // Add the token to combat as an attacker.
        let defender = state.opponent(controller);
        if let Some(ref mut combat) = state.combat {
            combat.attackers.insert(token_id, defender);
        }

        // Store the token ID so we can exile it at end of combat.
        // We use card_state on the Geist to track the angel token(s).
        if let Some(obj) = state.get_object_mut(self_id) {
            obj.card_state.insert("angel_token".into(), token_id);
        }

        state.log(crate::state::LogLevel::Event,
            "Geist of Saint Traft: created a 4/4 Angel token tapped and attacking".into());
    }

    fn on_end_step(&self, state: &mut GameState, self_id: ObjectId, _registry: &CardRegistry) {
        // Exile the angel token at end of combat (simplified: at end step).
        let angel_id = state.get_object(self_id)
            .and_then(|o| o.card_state.get("angel_token").copied());

        if let Some(angel_id) = angel_id {
            if state.get_object(angel_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
                state.move_object(angel_id, Zone::Exile);
                state.log(crate::state::LogLevel::Event,
                    "Geist of Saint Traft: Angel token exiled at end of combat".into());
            }
            if let Some(obj) = state.get_object_mut(self_id) {
                obj.card_state.remove("angel_token");
            }
        }
    }
}
