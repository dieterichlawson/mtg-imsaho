use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::{ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::*;

/// Essence of the Wild {3}{G}{G}{G} 6/6 Avatar.
/// Creatures you control enter the battlefield as a copy of Essence of the Wild.
///
/// Implementation: When any creature enters the battlefield under our control, override its
/// stats to 6/6 with no abilities. This is a replacement effect, but we implement it as an
/// ETB watcher trigger for simplicity.
pub struct EssenceOfTheWild;

impl CardBehavior for EssenceOfTheWild {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Essence of the Wild".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
                ManaSymbol::Colored(Color::Green),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![],
            subtypes: vec!["Avatar".into()],
            power: Some(6),
            toughness: Some(6),
            oracle_text: "Creatures you control enter as a copy of this creature.".into(),
            keywords: vec![],
            flashback_cost: None,
            continuous_effects: vec![],
            additional_cost: None,
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::AnyCreatureEnters,
                    description: "creature enters as copy of Essence of the Wild".into(),
                },
            ],
        }
    }

    fn on_any_creature_enters(&self, state: &mut GameState, self_id: ObjectId, entered_id: ObjectId, entered_controller: PlayerId, _registry: &CardRegistry) {
        let controller = match state.get_object(self_id) {
            Some(o) if o.zone == Zone::Battlefield => o.controller,
            _ => return,
        };
        // Only affect creatures we control (not Essence itself).
        if entered_controller != controller || entered_id == self_id {
            return;
        }
        // Override the entering creature to be a copy of Essence of the Wild.
        // Copy all copiable values: name, mana cost, colors, card types, subtypes,
        // power/toughness, keywords, oracle text, and abilities.
        if let Some(obj) = state.get_object_mut(entered_id) {
            let old_name = obj.name.clone();
            obj.name = "Essence of the Wild".into();
            obj.power = Some(6);
            obj.toughness = Some(6);
            obj.colors = vec![Color::Green];
            obj.card_types = vec![CardType::Creature];
            obj.subtypes = vec!["Avatar".into()];
            obj.keywords.clear();
            obj.instance_continuous_effects = Some(vec![]);
            obj.instance_oracle_text = Some("Creatures you control enter as a copy of this creature.".into());
            state.log(crate::state::LogLevel::Event,
                format!("Essence of the Wild: {} enters as a copy of Essence of the Wild (6/6)", old_name));
        }
    }
}
