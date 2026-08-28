use crate::cards::{CardBehavior, CardData, CardRegistry, TriggerKind, TriggeredAbilityDef};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, CardType, Keyword};

/// Creepy Doll — {5} 1/1 Artifact Creature — Construct with Indestructible.
/// Whenever Creepy Doll deals combat damage to a creature, flip a coin.
/// If you win the flip, destroy that creature.
pub struct CreepyDoll;

impl CardBehavior for CreepyDoll {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Creepy Doll".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(5),
            ])),
            card_types: vec![CardType::Artifact, CardType::Creature],
            subtypes: vec!["Construct".into()],
            power: Some(1),
            toughness: Some(1),
            oracle_text: "Indestructible\nWhenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.".into(),
            keywords: vec![Keyword::Indestructible],
            triggered_abilities: vec![
                TriggeredAbilityDef {
                    kind: TriggerKind::DealsCombatDamageToCreature,
                    description: "flip a coin; if you win, destroy that creature".into(),
                target_requirement: None,
                },
            ],
            ..Default::default()
        }
    }

    /// Ruling: "You don't flip the coin until the ability resolves." So the
    /// flip is here, in the resolution hook, and an opponent responding to the
    /// trigger — regenerating the damaged creature, say — does so without
    /// knowing the outcome.
    ///
    /// Nothing asks whether the Doll is still on the battlefield. A triggered
    /// ability on the stack exists independently of its source (CR 113.7a),
    /// and the Doll is indestructible but not unsacrificeable — Grimgrin eats
    /// it at instant speed. The flip happens and the creature is destroyed
    /// either way.
    fn on_deals_combat_damage_to_creature(&self, state: &mut GameState, _self_id: ObjectId, damaged_creature: ObjectId, _amount: u32, registry: &CardRegistry) {
        if !state.flip_coin() {
            state.log(crate::state::LogLevel::Event,
                "Creepy Doll lost the coin flip.".to_string());
            return;
        }
        // Ruling: lethal combat damage does not skip the flip, and a creature
        // that regenerated from that damage "could be destroyed a second
        // time". `try_destroy_by` is the destruction pipeline plus one log
        // line that says what actually happened — the Doll is not entitled to
        // announce a kill it did not get.
        crate::destruction::try_destroy_by(state, damaged_creature, "Creepy Doll", registry);
    }
}
