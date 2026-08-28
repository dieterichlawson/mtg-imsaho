use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, ManaType, ManaCost, ManaSymbol, Color};

/// Nephalia Drownyard — Land.
/// {T}: Add {C}.
/// {1}{U}{B}, {T}: Target player mills three cards.
pub struct NephaliaDrownyard;

impl CardBehavior for NephaliaDrownyard {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Nephalia Drownyard".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{1}{U}{B}, {T}: Target player mills three cards.".into(),
            ..Default::default()
        }
    }

    fn mana_abilities(&self, _state: &GameState, _object_id: ObjectId) -> Vec<ManaAbilityDef> {
        vec![ManaAbilityDef {
            ability_index: 0,
            description: "Add {C}".into(),
            produced: vec![(ManaType::Colorless, 1)],
            requires_tap: true,
            cost: ManaCost::free(),
            has_side_effects: false,
        }]
    }

    fn activated_abilities(&self, _state: &GameState, _object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        // No zone-or-tapped guard here: `legal_actions` enumerates only
        // battlefield permanents its player controls and rejects a
        // `requires_tap` ability on a tapped one, and it also applies the
        // summoning-sickness rule this never did (CR 302.6 — irrelevant to a
        // land, but a card should not be the place that decides).
        vec![ActivatedAbilityDef {
            ability_index: 1,
            description: "{1}{U}{B}, {T}: Target player mills three cards".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Generic(1),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::PlayerOnly),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Player(player_id)) = targets.first() {
            crate::engine::mill_cards(state, *player_id, 3, "Nephalia Drownyard", registry);
        }
    }
}
