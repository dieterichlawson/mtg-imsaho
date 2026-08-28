use crate::actions::Target;
use crate::cards::{ActivatedAbilityDef, CardBehavior, CardData, CardRegistry, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::ObjectId;
use crate::state::GameState;
use crate::types::{ManaCost, ManaSymbol, Color, CardType, Supertype, Keyword, Zone, CounterType};

/// Olivia Voldaren — {2}{B}{R} 3/3 Legendary Vampire with Flying.
/// {1}{R}: Deal 1 damage to another target creature. That creature becomes a Vampire in addition
/// to its other types. Put a +1/+1 counter on Olivia Voldaren.
/// {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
pub struct OliviaVoldaren;

impl CardBehavior for OliviaVoldaren {
    fn card_data(&self) -> CardData {
        CardData {
            name: "Olivia Voldaren".into(),
            cost: Some(ManaCost::new(vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colored(Color::Black),
                ManaSymbol::Colored(Color::Red),
            ])),
            card_types: vec![CardType::Creature],
            supertypes: vec![Supertype::Legendary],
            subtypes: vec!["Vampire".into()],
            power: Some(3),
            toughness: Some(3),
            oracle_text: "Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.".into(),
            keywords: vec![Keyword::Flying],
            // No triggered abilities. "for as long as you control Olivia
            // Voldaren" is a duration on the control-change effect (CR 611.2b),
            // not a leaves-the-battlefield trigger, and `expire_control_effects`
            // ends it as a state-based action. The `LeavesBattlefield` entry
            // declared here outlived the `on_leave_battlefield` that once
            // implemented it by hand, so every time Olivia left she put an
            // ability on the stack that did nothing and gave both players a
            // priority window for it.
            ..Default::default()
        }
    }

    fn activated_abilities(&self, state: &GameState, object_id: ObjectId, _registry: &CardRegistry) -> Vec<ActivatedAbilityDef> {
        match state.get_object(object_id) {
            Some(o) if o.zone == Zone::Battlefield => {}
            _ => return vec![],
        }

        vec![
            // Ability 0: {1}{R}: Deal 1 damage to another target creature.
            ActivatedAbilityDef {
                ability_index: 0,
                description: "{1}{R}: Deal 1 damage to target creature, make it a Vampire, +1/+1 counter on Olivia".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(1),
                    ManaSymbol::Colored(Color::Red),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Another)),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            },
            // Ability 1: {3}{B}{B}: Gain control of target Vampire.
            ActivatedAbilityDef {
                ability_index: 1,
                description: "{3}{B}{B}: Gain control of target Vampire".into(),
                cost: ManaCost::new(vec![
                    ManaSymbol::Generic(3),
                    ManaSymbol::Colored(Color::Black),
                    ManaSymbol::Colored(Color::Black),
                ]),
                requires_tap: false,
                sacrifice_cost: SacrificeCost::None,
                target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::HasSubtype("Vampire".into()))),
                once_per_turn: false,
                sorcery_speed_only: false,
                counter_cost: None,
            },
        ]
    }

    fn is_valid_target(&self, state: &GameState, _caster: crate::ids::PlayerId, target: &Target, registry: &CardRegistry) -> bool {
        match target {
            Target::Object(id) => {
                let obj = state.get_object(*id);
                obj.is_some_and(|o| o.zone == Zone::Battlefield && state.is_creature(o.id, registry))
            }
            Target::Player(_) => false,
            // CR 608.2b: a target that stopped being legal is skipped.
            Target::Illegal => false,
        }
    }

    fn resolve_activated_ability(&self, state: &mut GameState, object_id: ObjectId, ability_index: usize, targets: &[Target], registry: &CardRegistry) {

        match ability_index {
            0 => {
                if let Some(Target::Object(target_id)) = targets.first() {
                    if *target_id == object_id { return; }
                    let on_battlefield = state.get_object(*target_id).is_some_and(|o| o.zone == Zone::Battlefield);
                    if !on_battlefield { return; }
                    // No protection re-check here. Protection stops the
                    // targeting, which the engine already enforces at
                    // announcement and again at resolution (CR 608.2b), and
                    // prevents the damage, which `damage::deal_damage` already
                    // does. Returning early on it also skipped the two things
                    // the card grants unconditionally — the Vampire type and
                    // the counter — neither of which protection touches.
                    let effect = crate::state::PendingEffect::DealDamage {
                        amount: 1,
                        source_id: object_id,
                    };
                    crate::engine::apply_pending_effect(
                        state,
                        &Target::Object(*target_id),
                        &effect,
                        registry,
                    );
                    if let Some(obj) = state.get_object_mut(*target_id) {
                        if !obj.subtypes.contains(&"Vampire".to_string()) {
                            obj.subtypes.push("Vampire".to_string());
                        }
                    }
                    state.add_counters(object_id, CounterType::PlusOnePlusOne, 1);
                    state.log(crate::state::LogLevel::Event,
                        format!("Olivia Voldaren deals 1 damage to {}, makes it a Vampire, and gets a +1/+1 counter", state.obj_name(*target_id)));
                }
            }
            1 => {
                // {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia.
                if let Some(Target::Object(target_id)) = targets.first() {
                    // Recognise printed Vampires (registry subtype) as well as
                    // creatures Olivia's other ability turned into Vampires
                    // (object-level subtype) — via the characteristics layer.
                    let is_vampire = state.get_object(*target_id)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                        && state.has_subtype(*target_id, "Vampire", registry);
                    // "...for as long as **you** control Olivia Voldaren." Per
                    // the ruling, an ability activated and then answered by
                    // taking Olivia resolves with no effect: the duration is
                    // already over, so nothing is gained (CR 611.2b). "You" is
                    // the player who activated the ability (CR 602.2a), which
                    // is not always whoever holds Olivia by the time it
                    // resolves.
                    let activator = state.resolving_ability_activator
                        .unwrap_or_else(|| crate::cards::helpers::controller_of(state, object_id));
                    let still_controls_olivia = state.get_object(object_id)
                        .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == activator);
                    if is_vampire && still_controls_olivia {
                        // "...for as long as you control Olivia Voldaren."
                        // The duration is the engine's to track: it ends when
                        // Olivia leaves the battlefield *and* when anyone else
                        // takes control of her (CR 611.2b). Olivia used to keep
                        // her own list of what she had stolen and unwind it in
                        // on_leave_battlefield, which handled the first way and
                        // missed the second — an Act of Treason on Olivia left
                        // the stolen Vampires where they were.
                        let stolen_name = state.obj_name(*target_id);
                        state.gain_control_while_source_controlled(*target_id, object_id, registry);
                        state.log(crate::state::LogLevel::Event,
                            format!("Olivia Voldaren gains control of {stolen_name}"));
                    }
                }
            }
            _ => {}
        }
    }

}
