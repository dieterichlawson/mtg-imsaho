use crate::cards::{AttackInfo, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Keyword};
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

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = crate::cards::helpers::controller_of(state, self_id);
        let Some(source_card_id) = state.get_object(self_id).map(|o| o.card_id) else { return };

        // Create a 4/4 Angel token with flying, tapped and attacking.
        let token_ids = state.create_token_with_subtypes(
            "",
            controller,
            4, 4,
            vec![Color::White],
            vec![CardType::Creature],
            vec![Keyword::Flying],
            vec!["Angel".into()],
            registry,
        );

        // "that's tapped and attacking" — attacking the player Geist is
        // attacking, which the trigger already knows. Re-deriving it as
        // `state.opponent(controller)` is the same answer only while there are
        // exactly two players and no planeswalkers to attack.
        let defender = attack.defending_player;
        for token_id in token_ids {
            // "tapped and attacking" — the token arrives that way; nothing
            // tapped it.
            state.arrives_tapped(token_id);
            if let Some(obj) = state.get_object_mut(token_id) {
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
