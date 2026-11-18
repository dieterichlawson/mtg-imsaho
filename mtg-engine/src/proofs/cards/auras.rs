//! The Auras that enchant a creature, and what each one's text gives it.
//!
//! What an Aura does is the continuous effects it prints, pinned here to
//! its Oracle text, and applied by the engine as `proofs::actions::effects`
//! proves for each kind. "Enchant creature" is the requirement "target
//! creature" (CR 303.4a, 115.1), proved for each Aura here and in
//! `proofs::actions::targeting`. An Aura spell's resolution, its becoming
//! attached to the creature it targets (CR 303.4f, 608.3a), is the
//! engine's, and is played with the engine by `tests/card_semantics.rs`,
//! not proved here.
//!
//! - Holy Strength: "Enchanted creature gets +1/+2."
//! - Pacifism: "Enchanted creature can't attack or block."
//! - Bonds of Faith: "Enchanted creature gets +2/+2 as long as it's a
//!   Human. Otherwise, it can't attack or block."
//! - Claustrophobia: "When this Aura enters, tap enchanted creature.
//!   Enchanted creature doesn't untap during its controller's untap step."
//!   (its trigger proved below)
//! - Curiosity: "Whenever enchanted creature deals damage to an opponent,
//!   you may draw a card." (its trigger proved below)
//! - Dead Weight: "Enchanted creature gets -2/-2."
//! - Furor of the Bitten: "Enchanted creature gets +2/+2 and attacks each
//!   combat if able."
//! - Ghostly Possession: "Enchanted creature has flying. Prevent all combat
//!   damage that would be dealt to and dealt by enchanted creature."
//! - Gruesome Deformity: "Enchanted creature has intimidate."
//! - Sensory Deprivation: "Enchanted creature gets -3/-0."
//! - Skeletal Grimace: "Enchanted creature gets +1/+1 and has '{B}:
//!   Regenerate this creature.'" (the granted ability proved below)
//! - Spectral Flight: "Enchanted creature gets +2/+2 and has flying."
//! - Wreath of Geists: "Enchanted creature gets +X/+X, where X is the
//!   number of creature cards in your graveyard." (its X proved in
//!   `proofs::cards::counted_pt`)

use super::{card_proof, prints_effects, prints_where};
use crate::cards::{CardBehavior, TargetRequirement};
use crate::types::{ContinuousEffect as E, EffectCondition, EffectScope::Attached, Keyword};

/// `card` enchants a creature, and prints exactly `effects`.
fn prints(card: &dyn CardBehavior, effects: &[E]) {
    assert!(card.target_requirement() == TargetRequirement::Creature);
    assert!(prints_effects(card, effects));
}

macro_rules! aura {
    ($prints:ident, $card:expr, where $shape:expr) => {
        card_proof! {
            fn $prints() {
                assert!($card.target_requirement() == TargetRequirement::Creature);
                assert!(prints_where(&$card, $shape));
            }
        }
    };
    ($prints:ident, $card:expr, [$($effect:expr),* $(,)?]) => {
        card_proof! {
            fn $prints() {
                prints(&$card, &[$($effect),*]);
            }
        }
    };
}

aura!(holy_strength_gives_plus_one_plus_two, crate::cards::holy_strength::HolyStrength,
    [E::ModifyPT { power: 1, toughness: 2, scope: Attached }]);
aura!(pacifism_stops_attacking_and_blocking, crate::cards::pacifism::Pacifism,
    [E::PreventAttack { scope: Attached }, E::PreventBlock { scope: Attached }]);
aura!(bonds_of_faith_pumps_a_human_and_holds_anything_else, crate::cards::isd::bonds_of_faith::BondsOfFaith,
    where |e| matches!(e, [
        E::When { condition: EffectCondition::AttachedHasSubtype(a), effect: pump },
        E::When { condition: EffectCondition::AttachedLacksSubtype(b), effect: no_attack },
        E::When { condition: EffectCondition::AttachedLacksSubtype(c), effect: no_block },
    ] if a == "Human" && b == "Human" && c == "Human"
        && matches!(**pump, E::ModifyPT { power: 2, toughness: 2, scope: Attached })
        && matches!(**no_attack, E::PreventAttack { scope: Attached })
        && matches!(**no_block, E::PreventBlock { scope: Attached })));
aura!(claustrophobia_keeps_it_tapped, crate::cards::isd::claustrophobia::Claustrophobia,
    [E::PreventUntap { scope: Attached }]);
aura!(curiosity_prints_no_static_effect, crate::cards::isd::curiosity::Curiosity, []);
aura!(dead_weight_gives_minus_two_minus_two, crate::cards::isd::dead_weight::DeadWeight,
    [E::ModifyPT { power: -2, toughness: -2, scope: Attached }]);
aura!(furor_of_the_bitten_pumps_and_forces_attacks, crate::cards::isd::furor_of_the_bitten::FurorOfTheBitten,
    [E::ModifyPT { power: 2, toughness: 2, scope: Attached }, E::ForceAttack { scope: Attached }]);
aura!(ghostly_possession_gives_flying_and_prevents_combat_damage, crate::cards::isd::ghostly_possession::GhostlyPossession,
    [E::GrantKeyword { keyword: Keyword::Flying, scope: Attached }, E::PreventCombatDamage { scope: Attached }]);
aura!(gruesome_deformity_gives_intimidate, crate::cards::isd::gruesome_deformity::GruesomeDeformity,
    [E::GrantKeyword { keyword: Keyword::Intimidate, scope: Attached }]);
aura!(sensory_deprivation_gives_minus_three_minus_zero, crate::cards::isd::sensory_deprivation::SensoryDeprivation,
    [E::ModifyPT { power: -3, toughness: 0, scope: Attached }]);
aura!(skeletal_grimace_gives_plus_one_plus_one, crate::cards::isd::skeletal_grimace::SkeletalGrimace,
    [E::ModifyPT { power: 1, toughness: 1, scope: Attached }]);
aura!(spectral_flight_gives_plus_two_plus_two_and_flying, crate::cards::isd::spectral_flight::SpectralFlight,
    [E::ModifyPT { power: 2, toughness: 2, scope: Attached }, E::GrantKeyword { keyword: Keyword::Flying, scope: Attached }]);
aura!(wreath_of_geists_prints_no_fixed_effect, crate::cards::isd::wreath_of_geists::WreathOfGeists, []);

