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
        let controller = crate::cards::helpers::controller_of(state, self_id);
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
        let mut all_tokens: Vec<crate::ids::ObjectId> = Vec::new();
        for _ in 0..creature_count {
            all_tokens.extend(state.create_token_with_subtypes(
                "", controller, 2, 2,
                vec![Color::Green],
                vec![CardType::Creature],
                vec![],
                vec!["Wolf".into()],
                registry,
            ));
        }

        // "a 2/2 green Wolf creature token that's tapped and attacking" —
        // CR 508.4b, and this card's 2011-09-22 ruling: "You declare which
        // player or planeswalker each token is attacking as you put it onto
        // the battlefield. It doesn't have to be the same player or
        // planeswalker Kessig Cagebreakers is attacking." The shared helper
        // asks once per token when there is a real choice and sends every Wolf
        // at the only opponent silently when there is not.
        crate::cards::helpers::tokens_enter_combat_attacking(
            state, self_id, controller, &all_tokens, registry);
        state.log(crate::state::LogLevel::Event,
            format!("Kessig Cagebreakers created {creature_count} Wolf tokens tapped and attacking"));
    }
}
