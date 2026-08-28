use crate::cards::{AttackInfo, CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};
use crate::actions::Target;

/// Kessig Cagebreakers — {4}{G} 3/4 Human Rogue.
/// Whenever Kessig Cagebreakers attacks, create a 2/2 green Wolf creature token
/// that's tapped and attacking for each creature card in your graveyard.
pub struct KessigCagebreakers;

impl CardBehavior for KessigCagebreakers {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kessig Cagebreakers".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(4),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Human".into(), "Rogue".into()],
            power: Some(3),
            toughness: Some(4),
            oracle_text: "Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.".into(),
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::Attacks,
                    description: "create Wolf tokens tapped and attacking".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    fn on_attacks(&self, state: &mut GameState, self_id: ObjectId, _attack: AttackInfo, _chosen_targets: &[Target], registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) => o.controller,
            None => return,
        };
        // Count creature cards in graveyard.
        let creature_count = state.objects_in_zone(Zone::Graveyard, controller)
            .iter()
            .filter(|o| {
                // "for each creature **card** in your graveyard" — CR 109.1.
                state.is_card(o.id) && state.is_creature(o.id, registry)
            })
            .count();
        if creature_count == 0 {
            return;
        }
        // Find the defending player from the combat state.
        let defending_player = state.combat.as_ref()
            .and_then(|c| c.attackers.get(&self_id).copied())
            .unwrap_or_else(|| state.opponent(controller));

        for _ in 0..creature_count {
            let token_ids = state.create_token_with_subtypes(
                "", controller, 2, 2,
                vec![Color::Green],
                vec![CardType::Creature],
                vec![],
                vec!["Wolf".into()],
                registry,
            );
            // Tapped and attacking.
            for token_id in token_ids {
                if let Some(obj) = state.get_object_mut(token_id) {
                    obj.tapped = true;
                    obj.summoning_sick = false;
                }
                if let Some(combat) = &mut state.combat {
                    combat.attackers.insert(token_id, defending_player);
                }
            }
        }
        state.log(crate::state::LogLevel::Event,
            format!("Kessig Cagebreakers created {creature_count} Wolf tokens tapped and attacking"));
    }
}
