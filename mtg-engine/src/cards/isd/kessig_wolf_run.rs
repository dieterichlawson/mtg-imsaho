use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, ManaAbilityDef, SacrificeCost, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{CardType, Zone, ManaType, ManaCost, ManaSymbol, Color, Keyword};

/// Kessig Wolf Run — Land.
/// {T}: Add {C}.
/// {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
pub struct KessigWolfRun;

impl CardBehavior for KessigWolfRun {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Kessig Wolf Run".into(),
            card_types: vec![CardType::Land],
            oracle_text: "{T}: Add {C}.\n{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.".into(),
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
            description: "{X}{R}{G}, {T}: Target creature gets +X/+0 and trample until EOT".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::X,
                ManaSymbol::Colored(Color::Red),
                ManaSymbol::Colored(Color::Green),
            ]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::Creature),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    /// "Target creature" — and CR 608.2b asks again on resolution whether it is
    /// still one. A creature that has left the battlefield is not.
    ///
    /// This is the hook `stack.rs`'s ability arm calls before deciding whether
    /// to counter the ability, so answering here counters it outright rather
    /// than resolving into a no-op. The check used to sit inside the resolution
    /// handler, which reached the same answer for this card's single target but
    /// would have been the wrong place for two.
    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, _registry: &CardRegistry) -> bool {
        matches!(target, Target::Object(id)
            if state.get_object(*id).is_some_and(|o| o.zone == Zone::Battlefield))
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], _registry: &CardRegistry) {
        let x = i32::try_from(state.last_activated_x_value.unwrap_or(0)).unwrap_or(i32::MAX);
        // Legality was settled before this ran — see `is_valid_target`.
        let Some(Target::Object(target_id)) = targets.first() else { return };
        state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
            target: *target_id,
            power_mod: x,
            toughness_mod: 0,
        });
        state.until_end_of_turn.push(crate::state::TemporaryEffect::GrantKeyword {
            target: *target_id,
            keyword: Keyword::Trample,
        });
        let name = state.obj_name(*target_id);
        state.log(crate::state::LogLevel::Event,
            format!("Kessig Wolf Run gives {name} +{x}/+0 and trample until end of turn"));
    }
}
