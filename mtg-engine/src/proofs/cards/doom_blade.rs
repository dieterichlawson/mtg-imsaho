//! Doom Blade — {1}{B} Instant. "Destroy target nonblack creature."
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.

use super::{card_proof, Kinds};
use crate::actions::Target;
use crate::cards::doom_blade::DoomBlade;
use crate::cards::{CardBehavior, TargetFilter, TargetRequirement};
use crate::ids::PlayerId;
use crate::types::{CardType, Color, Zone};

const KINDS: Kinds = Kinds {
    types: &[CardType::Creature, CardType::Artifact],
    subtypes: &[],
    keywords: &[],
};

card_proof! {
    /// What Doom Blade's own code says it may target: its requirement is
    /// "creature with filter: nonblack", and it accepts exactly an object on
    /// the battlefield that is a creature and not black (CR 115.1, 205.1b,
    /// 105.2), and no player or illegal target, whoever casts it and
    /// whatever the object's colours, types, zone and controller.
    ///
    /// With `actions::targeting` (that requirement offers each permanent
    /// that is a creature, matches the filter, can be targeted by its caster
    /// and the card accepts) and `actions::predicates` (what each of those
    /// is), the targets Doom Blade offers are exactly the nonblack
    /// creatures its caster can target.
    fn doom_blade_accepts_exactly_the_nonblack_creatures() {
        let (registry, mut state, blank, _) = super::board(Box::new(DoomBlade));
        let id = super::any_object(&mut state, blank, &KINDS, super::Form::Card);
        let caster = PlayerId(u8::from(kani::any::<bool>()));
        assert!(DoomBlade.target_requirement() == TargetRequirement::CreatureWithFilter(TargetFilter::Nonblack));

        let o = state.get_object(id).expect("on the board");
        let legal = o.zone == Zone::Battlefield
            && o.card_types.contains(&CardType::Creature)
            && !o.colors.contains(&Color::Black);
        assert!(DoomBlade.is_valid_target(&state, caster, &Target::Object(id), &registry) == legal);
        assert!(!DoomBlade.is_valid_target(&state, caster, &Target::Player(PlayerId(0)), &registry));
        assert!(!DoomBlade.is_valid_target(&state, caster, &Target::Player(PlayerId(1)), &registry));
        assert!(!DoomBlade.is_valid_target(&state, caster, &Target::Illegal, &registry));
        kani::cover!(legal, "a nonblack creature");
        kani::cover!(!legal && o.zone == Zone::Battlefield && o.card_types.contains(&CardType::Creature), "a black creature");
        std::mem::forget((registry, state));
    }
}

