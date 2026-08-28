use crate::actions::Target;
use crate::cards::{CardBehavior, CardData, CardRegistry, ActivatedAbilityDef, SacrificeCost, TargetRequirement, TargetFilter};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Silverchase Fox — {1}{W} 2/2 Fox.
/// {1}{W}, Sacrifice Silverchase Fox: Exile target enchantment.
pub struct SilverchaseFox;

impl CardBehavior for SilverchaseFox {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Silverchase Fox".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::White),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Fox".into()],
            power: Some(2),
            toughness: Some(2),
            oracle_text: "{1}{W}, Sacrifice this creature: Exile target enchantment.".into(),
            ..Default::default()
        }
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{1}{W}, Sacrifice: Exile target enchantment".into(),
            cost: ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::White)]),
            requires_tap: false,
            sacrifice_cost: SacrificeCost::SacrificeThis,
            target_requirement: Some(TargetRequirement::PermanentWithFilter(
                TargetFilter::HasCardType(vec![CardType::Enchantment]),
            )),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        // No zone guard: CR 608.2b is the engine's, and it re-checks the
        // target against the `PermanentWithFilter` above before this is
        // called — an enchantment that left the battlefield in response is an
        // illegal target and the ability is countered by game rules, rather
        // than resolving and quietly exiling the card out of the graveyard.
        // The guard used to live here, which made the ability resolve and do
        // nothing: the right board state by the wrong route.
        if let Some(Target::Object(target_id)) = targets.first() {
            let exiled_name = state.obj_name(*target_id);
            state.move_object(*target_id, Zone::Exile, registry);
            state.log(crate::state::LogLevel::Event, format!("Silverchase Fox exiled {exiled_name}"));
        }
    }
}
