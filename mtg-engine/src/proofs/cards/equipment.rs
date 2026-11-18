//! The Equipment, and what each one's text gives the creature it equips.
//!
//! An Equipment's equip ability is "[cost]: Attach this permanent to
//! target creature you control. Activate only as a sorcery." (CR 702.6a),
//! and its resolution is the attachment, if the target is still a creature
//! its controller controls and the Equipment is still on the battlefield
//! and not a creature (702.6b, 301.5c, 608.2b). The ability as each
//! Equipment prints it, with its cost, and the card's own check of its
//! target are proved here; the resolution is played with the engine by
//! `tests/card_semantics.rs`, not proved here. What the Equipment then
//! does is the continuous effects it prints, pinned here to its Oracle
//! text and applied by the engine as `proofs::actions::effects` proves.
//! The abilities some Equipment have besides are proved below: Blazing
//! Torch's "Equipped creature has '{T}, Sacrifice Blazing Torch: Blazing
//! Torch deals 2 damage to any target.'", Trepanation Blade's "Whenever
//! equipped creature attacks, defending player reveals cards from the top
//! of their library until they reveal a land card. The creature gets +1/+0
//! until end of turn for each card revealed this way. That player puts the
//! revealed cards into their graveyard.", and Wooden Stake's "Whenever
//! equipped creature blocks or becomes blocked by a Vampire, destroy that
//! creature. It can't be regenerated." (Runechanter's Pike's X is
//! `proofs::cards::counted_pt`'s.)

use super::{any_object, board, card_proof, either, has_subtype, prints_effects, prints_where, Form, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::helpers::EQUIP_ABILITY_INDEX;
use crate::cards::{CardBehavior, SacrificeCost, TargetFilter, TargetRequirement};
use crate::ids::PlayerId;
use crate::types::{ContinuousEffect as E, CreatureFilter, EffectCondition, EffectScope::Attached, Keyword, ManaCost, ManaSymbol, Zone};

/// What an equip ability costs.
enum Equip {
    /// "Equip {N}".
    Generic(u32),
    /// "Equip—Sacrifice a creature".
    Sacrifice,
}

/// `card`'s equip ability, on the battlefield, costs `cost` and is "attach
/// to target creature you control, activate only as a sorcery"; and the
/// card prints what `effects` accepts (`prints_effects`, or for effects
/// holding a filter or another effect, `prints_where`).
fn prints(card: Box<dyn CardBehavior>, cost: Equip, effects: impl FnOnce(&dyn CardBehavior) -> bool) {
    let (registry, mut state, _, card) = board(card);
    let id = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    let behavior = registry.get(card).expect("registered");
    let abilities = behavior.activated_abilities(&state, id, &registry);
    let equip = abilities.iter().find(|a| a.ability_index == EQUIP_ABILITY_INDEX).expect("an equip ability");
    let (mana, sacrifice) = match cost {
        Equip::Generic(n) => (ManaCost::new(vec![ManaSymbol::Generic(n)]), SacrificeCost::None),
        Equip::Sacrifice => (ManaCost::free(), SacrificeCost::SacrificeCreature),
    };
    assert!(equip.cost == mana && equip.sacrifice_cost == sacrifice);
    assert!(equip.sorcery_speed_only && !equip.requires_tap && !equip.once_per_turn);
    assert!(equip.target_requirement == Some(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)));
    assert!(effects(behavior));
    std::mem::forget((registry, state, abilities));
}

/// `card`'s own check of a target, for its equip ability: a creature on
/// the battlefield its controller controls, and no player ("target
/// creature you control", which the engine asks besides the requirement,
/// CR 608.2b). For the other object on the board, any permanent or card of
/// either player's, and the Equipment either player's.
fn targets_your_creatures(card: Box<dyn CardBehavior>) {
    let (registry, mut state, blank, card) = board(card);
    let other = any_object(&mut state, blank, &ANY_PERMANENT, Form::Card);
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let o = state.get_object(other).expect("on the board");
    let yours = o.zone == Zone::Battlefield && o.card_types.contains(&crate::types::CardType::Creature) && o.controller == controller;
    let behavior = registry.get(card).expect("registered");
    assert!(behavior.is_valid_target(&state, controller, &Target::Object(other), &registry) == yours);
    assert!(!behavior.is_valid_target(&state, controller, &Target::Player(controller), &registry));
    kani::cover!(yours, "a creature of theirs");
    std::mem::forget((registry, state));
}

macro_rules! equipment {
    ($targets:ident, $prints:ident, $card:expr, $cost:expr, [$($effect:expr),* $(,)?]) => {
        equipment!(@ $targets, $prints, $card, $cost, |b| prints_effects(b, &[$($effect),*]));
    };
    // For an Equipment printing an effect that holds a filter or another
    // effect: its effects matched as `shape` says (`prints_effects` says
    // why such values are not compared with ones built to compare).
    ($targets:ident, $prints:ident, $card:expr, $cost:expr, where $shape:expr) => {
        equipment!(@ $targets, $prints, $card, $cost, |b| prints_where(b, $shape));
    };
    // For an Equipment whose `is_valid_target` answers for another
    // targeted ability too (Blazing Torch's "any target"), which its own
    // proof takes: no harness of the equip ability's check alone.
    (another targeted ability, $prints:ident, $card:expr, $cost:expr, where $shape:expr) => {
        card_proof! {
            fn $prints() {
                prints(Box::new($card), $cost, |b| prints_where(b, $shape));
            }
        }
    };
    (@ $targets:ident, $prints:ident, $card:expr, $cost:expr, $effects:expr) => {
        card_proof! {
            fn $targets() {
                targets_your_creatures(Box::new($card));
            }
        }
        card_proof! {
            fn $prints() {
                prints(Box::new($card), $cost, $effects);
            }
        }
    };
}

use crate::cards::isd as isd;

equipment!(another targeted ability, blazing_torch_equip_one_and_blocked_only_by_others, isd::blazing_torch::BlazingTorch, Equip::Generic(1),
    where |e: &[E]| matches!(e, [E::CanOnlyBeBlockedBy { allowed_blockers: CreatureFilter::Not(f), scope: Attached }]
        if either(f, |f| has_subtype(f, "Vampire"), |f| has_subtype(f, "Zombie"))));
equipment!(butchers_cleaver_equips_only_a_creature_you_control, butchers_cleaver_equip_three_plus_three_and_lifelink_for_a_human, isd::butchers_cleaver::ButchersCleaver, Equip::Generic(3),
    where |e: &[E]| matches!(e, [E::ModifyPT { power: 3, toughness: 0, scope: Attached },
        E::When { condition: EffectCondition::AttachedHasSubtype(s), effect }]
        if s == "Human" && matches!(**effect, E::GrantKeyword { keyword: Keyword::Lifelink, scope: Attached })));
equipment!(cobbled_wings_equips_only_a_creature_you_control, cobbled_wings_equip_one_and_flying, isd::cobbled_wings::CobbledWings, Equip::Generic(1), [
    E::GrantKeyword { keyword: Keyword::Flying, scope: Attached },
]);
equipment!(demonmail_hauberk_equips_only_a_creature_you_control, demonmail_hauberk_equip_by_sacrifice_plus_four_plus_two, isd::demonmail_hauberk::DemonmailHauberk, Equip::Sacrifice, [
    E::ModifyPT { power: 4, toughness: 2, scope: Attached },
]);
equipment!(inquisitors_flail_equips_only_a_creature_you_control, inquisitors_flail_equip_two_and_double_combat_damage, isd::inquisitors_flail::InquisitorsFlail, Equip::Generic(2), [
    E::DoubleCombatDamage { scope: Attached },
]);
equipment!(mask_of_avacyn_equips_only_a_creature_you_control, mask_of_avacyn_equip_three_plus_one_plus_two_and_hexproof, isd::mask_of_avacyn::MaskOfAvacyn, Equip::Generic(3), [
    E::ModifyPT { power: 1, toughness: 2, scope: Attached },
    E::GrantKeyword { keyword: Keyword::Hexproof, scope: Attached },
]);
equipment!(runechanters_pike_equips_only_a_creature_you_control, runechanters_pike_equip_two_and_first_strike, isd::runechanters_pike::RunechantersPike, Equip::Generic(2), [
    E::GrantKeyword { keyword: Keyword::FirstStrike, scope: Attached },
]);
equipment!(sharpened_pitchfork_equips_only_a_creature_you_control, sharpened_pitchfork_equip_one_first_strike_and_plus_one_for_a_human, isd::sharpened_pitchfork::SharpenedPitchfork, Equip::Generic(1),
    where |e: &[E]| matches!(e, [E::GrantKeyword { keyword: Keyword::FirstStrike, scope: Attached },
        E::When { condition: EffectCondition::AttachedHasSubtype(s), effect }]
        if s == "Human" && matches!(**effect, E::ModifyPT { power: 1, toughness: 1, scope: Attached })));
equipment!(silver_inlaid_dagger_equips_only_a_creature_you_control, silver_inlaid_dagger_equip_two_plus_two_and_one_more_for_a_human, isd::silver_inlaid_dagger::SilverInlaidDagger, Equip::Generic(2),
    where |e: &[E]| matches!(e, [E::ModifyPT { power: 2, toughness: 0, scope: Attached },
        E::When { condition: EffectCondition::AttachedHasSubtype(s), effect }]
        if s == "Human" && matches!(**effect, E::ModifyPT { power: 1, toughness: 0, scope: Attached })));
equipment!(trepanation_blade_equips_only_a_creature_you_control, trepanation_blade_equip_two, isd::trepanation_blade::TrepanationBlade, Equip::Generic(2), []);
equipment!(wooden_stake_equips_only_a_creature_you_control, wooden_stake_equip_one_plus_one, isd::wooden_stake::WoodenStake, Equip::Generic(1), [
    E::ModifyPT { power: 1, toughness: 0, scope: Attached },
]);

