use crate::cards::{AttackInfo, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Keyword, Zone};
use crate::actions::Target;

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
            oracle_text: "Hexproof (This creature can't be the target of spells or abilities your opponents control.)\nWhenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.".into(),
            keywords: vec![Keyword::Hexproof],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "create a 4/4 Angel token tapped and attacking".into(),
                    target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, _targets: &[crate::actions::Target], registry: &CardRegistry) {
        state.move_object(object_id, Zone::Battlefield, registry);
        if let Some(obj) = state.get_object_mut(object_id) {
            obj.is_legendary = true;
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        let (controller, source_card_id) = match state.get_object(self_id) {
            Some(o) => (o.controller, o.card_id),
            None => return,
        };

        // Create a 4/4 Angel token with flying, tapped and attacking.
        let token_ids = state.create_token_with_subtypes(
            "Angel",
            controller,
            4, 4,
            vec![Color::White],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Angel".into()],
            registry,
        );

        let defender = state.opponent(controller);
        for token_id in token_ids {
            // Set the token as tapped and attacking.
            if let Some(obj) = state.get_object_mut(token_id) {
                obj.tapped = true;
                obj.summoning_sick = false; // It's attacking, so summoning sickness doesn't matter.
            }

            // Add the token to combat as an attacker.
            if let Some(ref mut combat) = state.combat {
                combat.attackers.insert(token_id, defender);
            }

            // Register a delayed triggered ability (CR 603.7) to exile the
            // Angel at end of combat. It will be drained onto the stack by
            // triggers::collect_triggers on StepStarted { EndCombat }, giving
            // players priority to respond before it resolves.
            state.end_of_combat_exiles.push(crate::state::EndOfCombatExileEntry {
                target_id: token_id,
                source_id: self_id,
                source_card_id,
                controller,
                description: "exile the Angel token".into(),
            });
        }

        state.log(crate::state::LogLevel::Event,
            "Geist of Saint Traft: created a 4/4 Angel token tapped and attacking".into());
    }
}
