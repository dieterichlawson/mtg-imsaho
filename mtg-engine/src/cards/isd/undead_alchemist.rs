use crate::cards::{CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Undead Alchemist — {3}{U} 4/2 Zombie.
/// If a Zombie you control would deal combat damage to a player, instead that
/// player mills that many cards. Whenever a creature card is put into an
/// opponent's graveyard from their library, exile that card and create a 2/2
/// black Zombie creature token.
///
/// Ability 1 is a replacement effect: combat damage from Zombies is replaced
/// with milling. Implemented via replace_combat_damage_to_player.
///
/// Ability 2 (mill-watcher trigger for non-combat mill sources) is not yet
/// implemented as a standalone trigger — currently the exile-and-token logic
/// is inlined in the replacement effect for the combat mill path only.
pub struct UndeadAlchemist;

impl CardBehavior for UndeadAlchemist {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Undead Alchemist".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Blue),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Zombie".into()],
            power: Some(4),
            toughness: Some(2),
            oracle_text: "If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.\nWhenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                crate::cards::TriggeredAbilityDef {
                    kind: crate::cards::TriggerKind::CreatureCardMilled,
                    description: "exile milled creature, create Zombie token".into(),
                },
            ],
        }
    }

    fn on_creature_card_milled(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        milled_object: ObjectId,
        _milled_player: PlayerId,
        registry: &CardRegistry,
    ) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Exile the milled creature card and create a 2/2 Zombie token.
        state.move_object(milled_object, Zone::Exile, registry);
        state.create_token_with_subtypes(
            "Zombie", controller, 2, 2,
            vec![Color::Black],
            vec![CardType::Creature],
            vec![],
            vec!["Zombie".into()],
            registry,
        );
        let name = state.get_object(milled_object).map(|o| o.name.clone()).unwrap_or_default();
        state.log(crate::state::LogLevel::Event,
            format!("Undead Alchemist: exiled milled {}, created Zombie token", name));
    }

    fn replace_combat_damage_to_player(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        source_id: ObjectId,
        damaged_player: PlayerId,
        amount: u32,
        registry: &CardRegistry,
    ) -> bool {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return false,
        };
        // Only replace damage from Zombies we control.
        let source = match state.get_object(source_id) {
            Some(o) if o.controller == controller => o,
            _ => return false,
        };
        let is_zombie = registry.card_data(source.card_id)
            .map(|d| d.subtypes.iter().any(|s| s == "Zombie"))
            .unwrap_or(false)
            || source.subtypes.iter().any(|s| s == "Zombie");
        if !is_zombie {
            return false;
        }

        // Mill that many cards instead of dealing damage. mill_cards emits
        // CreatureCardMilled events, which the trigger system picks up to
        // fire our on_creature_card_milled (exile + create Zombie token).
        crate::engine::mill_cards(state, damaged_player, amount as usize, registry);

        state.log(crate::state::LogLevel::Event,
            format!("Undead Alchemist: Zombie combat damage replaced with mill ({})",
                amount));
        true // Damage fully replaced
    }
}
