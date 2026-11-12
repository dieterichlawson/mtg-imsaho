//! The permanents whose rules text, beyond keywords, is static abilities
//! alone (CR 604): each one's continuous effects, pinned to its Oracle
//! text. Which objects each reaches, and what the engine does with it, is
//! `proofs::actions::effects`, for each kind of effect and scope; the
//! keywords are each card's printed characteristics, checked against the
//! Oracle record for every card.
//!
//! - Glorious Anthem: "Creatures you control get +1/+1."
//! - Angelic Overseer: "As long as you control a Human, this creature has
//!   hexproof and indestructible."
//! - Battleground Geist: "Other Spirit creatures you control get +1/+0."
//! - Elite Inquisitor: "Protection from Vampires, from Werewolves, and from
//!   Zombies"
//! - Gallows Warden: "Other Spirit creatures you control get +0/+1."
//! - Grave Bramble: "protection from Zombies"
//! - Heartless Summoning: "Creature spells you cast cost {2} less to cast.
//!   Creatures you control get -1/-1."
//! - Intangible Virtue: "Creature tokens you control get +1/+1 and have
//!   vigilance."
//! - Invisible Stalker: "This creature can't be blocked."
//! - Night Revelers: "This creature has haste as long as an opponent
//!   controls a Human."
//! - One-Eyed Scarecrow: "Creatures with flying your opponents control get
//!   -1/-0."
//! - Orchard Spirit: "This creature can't be blocked except by creatures
//!   with flying or reach."
//! - Rooftop Storm: "You may pay {0} rather than pay the mana cost for
//!   Zombie creature spells you cast."
//! - Stony Silence: "Activated abilities of artifacts can't be activated."
//! - Vampire Interloper: "This creature can't block."

use super::{both, card_proof, either, has_subtype, prints_effects, prints_where};
use crate::types::{
    ContinuousEffect as E, CreatureFilter as F, EffectCondition as If, EffectScope::{Global, GlobalOther, OnSelf},
    Keyword, ManaCost, SpellFilter,
};

/// A pin of a card whose effects hold no filter or effect inside another,
/// written out; and of one whose effects do, as a pattern
/// (`proofs::cards::prints_effects` says why the two).
macro_rules! statics {
    ($($name:ident: $card:expr => $expected:tt),* $(,)?) => {
        $(
            card_proof! {
                fn $name() {
                    assert!(statics!(@check $card, $expected));
                }
            }
        )*
    };
    (@check $card:expr, [$($effect:expr),* $(,)?]) => { prints_effects(&$card, &[$($effect),*]) };
    (@check $card:expr, (where $shape:expr)) => { prints_where(&$card, $shape) };
}

use crate::cards as core;
use crate::cards::isd as isd;

/// "Other Spirit creatures you control".
fn other_spirits_you_control(f: &F) -> bool {
    both(f, |f| matches!(f, F::ControlledByYou), |f| has_subtype(f, "Spirit"))
}

statics! {
    glorious_anthem_pumps_your_creatures: core::glorious_anthem::GloriousAnthem
        => [E::ModifyPT { power: 1, toughness: 1, scope: Global(F::ControlledByYou) }],
    angelic_overseer_has_hexproof_and_indestructible_with_a_human: isd::angelic_overseer::AngelicOverseer => (where |e| matches!(e, [
        E::When { condition: If::YouControlSubtype(a), effect: hexproof },
        E::When { condition: If::YouControlSubtype(b), effect: indestructible },
    ] if a == "Human" && b == "Human"
        && matches!(**hexproof, E::GrantKeyword { keyword: Keyword::Hexproof, scope: OnSelf })
        && matches!(**indestructible, E::GrantKeyword { keyword: Keyword::Indestructible, scope: OnSelf }))),
    battleground_geist_pumps_your_other_spirits: isd::battleground_geist::BattlegroundGeist
        => (where |e| matches!(e, [E::ModifyPT { power: 1, toughness: 0, scope: GlobalOther(f) }] if other_spirits_you_control(f))),
    elite_inquisitor_has_protection_from_vampires_werewolves_and_zombies: isd::elite_inquisitor::EliteInquisitor => (where |e| matches!(e, [
        E::ProtectionFromSubtype { subtype: v, scope: OnSelf },
        E::ProtectionFromSubtype { subtype: w, scope: OnSelf },
        E::ProtectionFromSubtype { subtype: z, scope: OnSelf },
    ] if v == "Vampire" && w == "Werewolf" && z == "Zombie")),
    gallows_warden_toughens_your_other_spirits: isd::gallows_warden::GallowsWarden
        => (where |e| matches!(e, [E::ModifyPT { power: 0, toughness: 1, scope: GlobalOther(f) }] if other_spirits_you_control(f))),
    grave_bramble_has_protection_from_zombies: isd::grave_bramble::GraveBramble
        => [E::ProtectionFromSubtype { subtype: "Zombie".into(), scope: OnSelf }],
    heartless_summoning_discounts_creatures_and_shrinks_yours: isd::heartless_summoning::HeartlessSummoning => [
        E::ModifyPT { power: -1, toughness: -1, scope: Global(F::ControlledByYou) },
        E::ReduceCost { reduction: 2, filter: SpellFilter::CreatureSpells },
    ],
    intangible_virtue_pumps_your_tokens: isd::intangible_virtue::IntangibleVirtue => [
        E::ModifyPT { power: 1, toughness: 1, scope: Global(F::ControlledByYouToken) },
        E::GrantKeyword { keyword: Keyword::Vigilance, scope: Global(F::ControlledByYouToken) },
    ],
    invisible_stalker_cant_be_blocked: isd::invisible_stalker::InvisibleStalker => [E::CantBeBlocked { scope: OnSelf }],
    night_revelers_has_haste_against_a_human: isd::night_revelers::NightRevelers => (where |e| matches!(e, [
        E::When { condition: If::OpponentControlsSubtype(h), effect },
    ] if h == "Human" && matches!(**effect, E::GrantKeyword { keyword: Keyword::Haste, scope: OnSelf }))),
    one_eyed_scarecrow_shrinks_opposing_fliers: isd::one_eyed_scarecrow::OneEyedScarecrow => (where |e| matches!(e, [
        E::ModifyPT { power: -1, toughness: 0, scope: Global(f) },
    ] if both(f, |f| matches!(f, F::ControlledByOpponent), |f| matches!(f, F::HasKeyword(Keyword::Flying))))),
    orchard_spirit_is_blocked_only_by_flying_or_reach: isd::orchard_spirit::OrchardSpirit => (where |e| matches!(e, [
        E::CanOnlyBeBlockedBy { allowed_blockers: f, scope: OnSelf },
    ] if either(f, |f| matches!(f, F::HasKeyword(Keyword::Flying)), |f| matches!(f, F::HasKeyword(Keyword::Reach))))),
    rooftop_storm_makes_zombies_free: isd::rooftop_storm::RooftopStorm
        => [E::AlternativeCost { cost: ManaCost::free(), filter: SpellFilter::CreatureWithSubtype("Zombie".into()) }],
    stony_silence_stops_artifact_abilities: isd::stony_silence::StonySilence => [E::PreventArtifactAbilities],
    vampire_interloper_cant_block: isd::vampire_interloper::VampireInterloper => [E::PreventBlock { scope: OnSelf }],
}
