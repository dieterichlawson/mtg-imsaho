use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost,
                   TargetFilter, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::{EnterAsCopyChoice, GameState};
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Zone};

/// Evil Twin {2}{U}{B} 0/0 Shapeshifter.
/// You may have Evil Twin enter the battlefield as a copy of any creature on the battlefield,
/// except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
pub struct EvilTwin;

impl CardBehavior for EvilTwin {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Evil Twin".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ])),
            card_types: vec![CardType::Creature],
            subtypes: vec!["Shapeshifter".into()],
            power: Some(0),
            toughness: Some(0),
            oracle_text: "You may have this creature enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"".into(),
            ..Default::default()
        }
    }

    /// CR 614.12b: "You may have this creature enter as a copy of any
    /// creature on the battlefield" is a replacement effect applied as it
    /// enters, not a triggered ability. The engine defers the entry until
    /// the choice is answered and records the answer on the object; this
    /// turns that answer into the copy.
    ///
    /// Doing it through an ETB trigger — as this card used to — put a
    /// printed 0/0 onto the battlefield with the choice sitting on the
    /// stack: it survived state-based actions only because of a bespoke
    /// exemption, the copied creature's "enters tapped" never applied
    /// (CR 614.1c), and both players got a priority window that the rules
    /// do not have.
    fn chooses_copy_as_it_enters(&self) -> bool { true }

    // "except it has '{U}{B}, {T}: Destroy target creature with the same
    // name as this creature'" — the copy carries an Evil Twin-granted
    // ability, so the collectors must consult this behavior for it.
    fn grants_abilities_to_copies(&self) -> bool { true }

    fn replace_event(
        &self,
        state: &mut GameState,
        self_id: ObjectId,
        event: &crate::replacement::ReplaceableEvent,
        _registry: &CardRegistry,
    ) -> Option<crate::replacement::Replacement> {
        use crate::replacement::{ReplaceableEvent, Replacement};
        let ReplaceableEvent::EntersBattlefield(e) = event else { return None };
        // Only its own arrival, and only if a copy effect has not already
        // decided what it enters as (CR 616.1 — an Essence of the Wild the
        // controller also has out settles it first).
        if e.object != self_id || e.copy_of.is_some() {
            return None;
        }
        let EnterAsCopyChoice::Copy(source) = state
            .get_object(self_id)
            .map_or(EnterAsCopyChoice::Unasked, |o| o.entering_copy_choice)
        else {
            // Declined, or nothing to copy: it enters as its printed 0/0.
            return None;
        };
        let mut e = e.clone();
        e.copy_of = Some(source);
        Some(Replacement::Modified(ReplaceableEvent::EntersBattlefield(e)))
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        let obj = match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => o,
            _ => return vec![],
        };
        // The granted ability exists only on a permanent that actually entered
        // as a copy (CR 706.2 "except it has ..."). An Evil Twin whose copy
        // choice was declined is a plain 0/0 with no ability. The engine
        // consults this behavior for a copy whose `copy_grantor` is this card,
        // so the presence of a grantor is the whole test.
        if obj.copy_grantor.is_none() {
            return vec![];
        }

        vec![ActivatedAbilityDef {
            ability_index: 0,
            description: "{U}{B}, {T}: Destroy target creature with the same name".into(),
            cost: ManaCost::new(vec![
                ManaSymbol::Colored(Color::Blue),
                ManaSymbol::Colored(Color::Black),
            ]),
            requires_tap: true,
            sacrifice_cost: SacrificeCost::None,
            target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::SameNameAsSource)),
            once_per_turn: false,
            sorcery_speed_only: false,
            counter_cost: None,
        }]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        // Basic validation: target must be a creature on the battlefield.
        // The SameNameAsSource filter in matches_ability_target_filter handles
        // the name-matching restriction for the activated ability.
        match target {
            Target::Object(id) => {
                state.get_object(*id)
                    .is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry))
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, _object_id: ObjectId, _ability_index: usize, targets: &[Target], registry: &CardRegistry) {
        if let Some(Target::Object(target_id)) = targets.first() {
            crate::destruction::try_destroy_by(state, *target_id, "Evil Twin", registry);
        }
    }
}
