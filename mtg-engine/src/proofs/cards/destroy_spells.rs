//! The spells whose resolution is "Destroy target <permanent>", for each
//! permanent or card they could hold.
//!
//! - Doom Blade, {1}{B} Instant: "Destroy target nonblack creature." (its own
//!   file also proves its targets)
//! - Ancient Grudge, {1}{R} Instant: "Destroy target artifact. Flashback {G}"
//! - Naturalize, {1}{G} Instant: "Destroy target artifact or enchantment."
//! - Rebuke, {2}{W} Instant: "Destroy target attacking creature."
//! - Smite the Monstrous, {3}{W} Instant: "Destroy target creature with
//!   power 4 or greater."
//! - Urgent Exorcism, {1}{W} Instant: "Destroy target Spirit or enchantment."
//! - Victim of Night, {B}{B} Instant: "Destroy target non-Vampire,
//!   non-Werewolf, non-Zombie creature."
//! - Bramblecrush, {2}{G}{G} Sorcery: "Destroy target noncreature permanent."
//!
//! What each may target is the engine's targeting with each card's
//! requirement, proved for each card in `targets`; what each asks for as it
//! resolves is the same, proved here: the permanent it holds destroyed, if
//! it still is one (CR 701.8a), and with it gone to a graveyard or its
//! target illegal, nothing (400.7, 608.2b). Nothing else changes.

use super::{card_proof, destroys_its_target, resolve, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::isd::ancient_grudge::AncientGrudge;
use crate::cards::isd::bramblecrush::Bramblecrush;
use crate::cards::isd::naturalize::Naturalize;
use crate::cards::isd::rebuke::Rebuke;
use crate::cards::isd::smite_the_monstrous::SmiteTheMonstrous;
use crate::cards::isd::urgent_exorcism::UrgentExorcism;
use crate::cards::isd::victim_of_night::VictimOfNight;

/// One proof per card: holding the other object (any permanent or card),
/// or a target gone illegal.
macro_rules! destroys {
    ($name:ident, $card:expr) => {
        card_proof! {
            fn $name() {
                let illegal: bool = kani::any();
                let r = resolve(Box::new($card), &ANY_PERMANENT, |_, _| {},
                    |other, _| if illegal { Target::Illegal } else { Target::Object(other) });
                assert!(destroys_its_target(&r) && r.nothing_else_changed());
                kani::cover!(super::acted() > 0, "a permanent destroyed");
                std::mem::forget(r);
            }
        }
    };
}

destroys!(ancient_grudge_destroys_its_target, AncientGrudge);
destroys!(naturalize_destroys_its_target, Naturalize);
destroys!(rebuke_destroys_its_target, Rebuke);
destroys!(smite_the_monstrous_destroys_its_target, SmiteTheMonstrous);
destroys!(urgent_exorcism_destroys_its_target, UrgentExorcism);
destroys!(victim_of_night_destroys_its_target, VictimOfNight);
destroys!(bramblecrush_destroys_its_target, Bramblecrush);
