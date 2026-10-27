//! The Equipment, and what each one's text gives the creature it equips.
//!
//! An Equipment's equip ability is "[cost]: Attach this permanent to
//! target creature you control. Activate only as a sorcery." (CR 702.6a),
//! and its resolution is the attachment, if the target is still a creature
//! its controller controls and the Equipment is still on the battlefield
//! and not a creature (702.6b, 301.5c, 608.2b). Both are proved here for
//! each Equipment, with the cost its text prints. What the Equipment then
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

use super::{act, acted, any_object, any_target, asked_for, board, card_proof, prints_effects, triggers, Act, Form, Kinds, Snapshot, ANY_PERMANENT};
use crate::actions::Target;
use crate::cards::helpers::EQUIP_ABILITY_INDEX;
use crate::cards::{AttackInfo, CardBehavior, SacrificeCost, TargetFilter, TargetRequirement, TriggerKind};
use crate::damage::DamageKind;
use crate::events::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::state::TemporaryEffect as T;
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
/// card prints exactly `effects`.
fn prints(card: Box<dyn CardBehavior>, cost: Equip, effects: &[E]) {
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
    assert!(prints_effects(behavior, effects));
    std::mem::forget((registry, state, abilities));
}

/// `card`'s equip ability resolving, the Equipment on the battlefield or
/// not and either player's, holding the other object on the board (any
/// permanent or card, either player's) or a target gone illegal: it asks to
/// attach the Equipment to that object exactly when the Equipment is on the
/// battlefield and the object is a creature on the battlefield its
/// controller controls; nothing else changes. And, unless the card's
/// target check answers for another ability too, that check is "target
/// creature you control".
fn attaches(card: Box<dyn CardBehavior>, equip_targets_only: bool) {
    let (registry, mut state, blank, card) = board(card);
    let other = any_object(&mut state, blank, &ANY_PERMANENT, Form::Card);
    let controller = PlayerId(u8::from(kani::any::<bool>()));
    let on: bool = kani::any();
    let equipment = state.create_object(card, controller, if on { Zone::Battlefield } else { Zone::Exile }, None, None);
    let target = if kani::any() { Target::Object(other) } else { Target::Illegal };
    let before = Snapshot::of(&state);

    // "target creature you control", as the card checks it besides the
    // requirement (the engine asks both, CR 608.2b): a creature on the
    // battlefield its controller controls, and no player.
    let o = state.get_object(other).expect("on the board");
    let yours = o.zone == Zone::Battlefield && o.card_types.contains(&crate::types::CardType::Creature) && o.controller == controller;
    let behavior = registry.get(card).expect("registered");
    if equip_targets_only {
        assert!(behavior.is_valid_target(&state, controller, &Target::Object(other), &registry) == yours);
        assert!(!behavior.is_valid_target(&state, controller, &Target::Player(controller), &registry));
    }

    behavior.resolve_activated_ability(&mut state, equipment, EQUIP_ABILITY_INDEX, &[target.clone()], &registry);

    let legal = target == Target::Object(other) && on && yours;
    if legal {
        assert!(asked_for(&[Act::Attach(equipment, other)]));
    } else {
        assert!(acted() == 0);
    }
    assert!(Snapshot::of(&state) == before);
    kani::cover!(legal, "attached");
    std::mem::forget((registry, state));
}

macro_rules! equipment {
    ($attaches:ident, $prints:ident, $card:expr, $cost:expr, [$($effect:expr),* $(,)?]) => {
        equipment!(@ $attaches, $prints, $card, $cost, true, [$($effect),*]);
    };
    // For an Equipment whose `is_valid_target` answers for another
    // targeted ability too (Blazing Torch's "any target"), which its own
    // proof takes.
    (another targeted ability, $attaches:ident, $prints:ident, $card:expr, $cost:expr, [$($effect:expr),* $(,)?]) => {
        equipment!(@ $attaches, $prints, $card, $cost, false, [$($effect),*]);
    };
    (@ $attaches:ident, $prints:ident, $card:expr, $cost:expr, $equip_targets_only:expr, [$($effect:expr),* $(,)?]) => {
        card_proof! {
            fn $attaches() {
                attaches(Box::new($card), $equip_targets_only);
            }
        }
        card_proof! {
            fn $prints() {
                prints(Box::new($card), $cost, &[$($effect),*]);
            }
        }
    };
}

use crate::cards::isd as isd;

equipment!(another targeted ability, blazing_torch_equips, blazing_torch_equip_one_and_blocked_only_by_others, isd::blazing_torch::BlazingTorch, Equip::Generic(1), [
    E::CanOnlyBeBlockedBy {
        allowed_blockers: CreatureFilter::Not(Box::new(CreatureFilter::Or(vec![
            CreatureFilter::HasSubtype("Vampire".into()),
            CreatureFilter::HasSubtype("Zombie".into()),
        ]))),
        scope: Attached,
    },
]);
equipment!(butchers_cleaver_equips, butchers_cleaver_equip_three_plus_three_and_lifelink_for_a_human, isd::butchers_cleaver::ButchersCleaver, Equip::Generic(3), [
    E::ModifyPT { power: 3, toughness: 0, scope: Attached },
    E::when(EffectCondition::AttachedHasSubtype("Human".into()), E::GrantKeyword { keyword: Keyword::Lifelink, scope: Attached }),
]);
equipment!(cobbled_wings_equips, cobbled_wings_equip_one_and_flying, isd::cobbled_wings::CobbledWings, Equip::Generic(1), [
    E::GrantKeyword { keyword: Keyword::Flying, scope: Attached },
]);
equipment!(demonmail_hauberk_equips, demonmail_hauberk_equip_by_sacrifice_plus_four_plus_two, isd::demonmail_hauberk::DemonmailHauberk, Equip::Sacrifice, [
    E::ModifyPT { power: 4, toughness: 2, scope: Attached },
]);
equipment!(inquisitors_flail_equips, inquisitors_flail_equip_two_and_double_combat_damage, isd::inquisitors_flail::InquisitorsFlail, Equip::Generic(2), [
    E::DoubleCombatDamage { scope: Attached },
]);
equipment!(mask_of_avacyn_equips, mask_of_avacyn_equip_three_plus_one_plus_two_and_hexproof, isd::mask_of_avacyn::MaskOfAvacyn, Equip::Generic(3), [
    E::ModifyPT { power: 1, toughness: 2, scope: Attached },
    E::GrantKeyword { keyword: Keyword::Hexproof, scope: Attached },
]);
equipment!(runechanters_pike_equips, runechanters_pike_equip_two_and_first_strike, isd::runechanters_pike::RunechantersPike, Equip::Generic(2), [
    E::GrantKeyword { keyword: Keyword::FirstStrike, scope: Attached },
]);
equipment!(sharpened_pitchfork_equips, sharpened_pitchfork_equip_one_first_strike_and_plus_one_for_a_human, isd::sharpened_pitchfork::SharpenedPitchfork, Equip::Generic(1), [
    E::GrantKeyword { keyword: Keyword::FirstStrike, scope: Attached },
    E::when(EffectCondition::AttachedHasSubtype("Human".into()), E::ModifyPT { power: 1, toughness: 1, scope: Attached }),
]);
equipment!(silver_inlaid_dagger_equips, silver_inlaid_dagger_equip_two_plus_two_and_one_more_for_a_human, isd::silver_inlaid_dagger::SilverInlaidDagger, Equip::Generic(2), [
    E::ModifyPT { power: 2, toughness: 0, scope: Attached },
    E::when(EffectCondition::AttachedHasSubtype("Human".into()), E::ModifyPT { power: 1, toughness: 0, scope: Attached }),
]);
equipment!(trepanation_blade_equips, trepanation_blade_equip_two, isd::trepanation_blade::TrepanationBlade, Equip::Generic(2), []);
equipment!(wooden_stake_equips, wooden_stake_equip_one_plus_one, isd::wooden_stake::WoodenStake, Equip::Generic(1), [
    E::ModifyPT { power: 1, toughness: 0, scope: Attached },
]);

card_proof! {
    /// Blazing Torch's granted ability, on the creature it equips: "{T},
    /// Sacrifice Blazing Torch", targeting any target (CR 115.4: a
    /// permanent on the battlefield, or a player still in the game),
    /// offered while a Torch the creature's controller controls is on it
    /// (only they could sacrifice it, CR 701.21a); paid, that Torch
    /// sacrificed; resolving, 2 damage from the Torch, not the creature
    /// (the card's ruling), to its target; its target gone, nothing.
    fn blazing_torch_gives_its_creature_a_sacrifice_to_deal_two() {
        let (registry, mut state, blank, card) = board(Box::new(isd::blazing_torch::BlazingTorch));
        let creature = state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(2), Some(2));
        let controller = PlayerId(u8::from(kani::any::<bool>()));
        state.get_object_mut(creature).expect("just created").controller = controller;
        state.get_object_mut(creature).expect("just created").card_types = vec![crate::types::CardType::Creature];
        let torch_controller = PlayerId(u8::from(kani::any::<bool>()));
        let torch = state.create_object(card, torch_controller, Zone::Battlefield, None, None);
        let attached: bool = kani::any();
        if attached {
            state.get_object_mut(torch).expect("just created").attached_to = Some(creature);
        }
        let other = any_object(&mut state, blank, &ANY_PERMANENT, Form::Card);
        let behavior = registry.get(card).expect("registered");
        let defs = std::mem::ManuallyDrop::new(behavior.activated_abilities(&state, creature, &registry));
        if attached && torch_controller == controller {
            let a = &defs[..];
            assert!(a.len() == 1 && a[0].ability_index == 1 && a[0].requires_tap && a[0].cost == ManaCost::free()
                && a[0].sacrifice_cost == SacrificeCost::None && a[0].target_requirement == Some(TargetRequirement::AnyTarget)
                && !a[0].sorcery_speed_only && !a[0].once_per_turn && a[0].counter_cost.is_none() && a[0].tap_cost.is_none());
        } else {
            assert!(defs.is_empty());
        }
        let o = state.get_object(other).expect("on the board");
        let on = o.zone == Zone::Battlefield;
        assert!(behavior.is_valid_target(&state, controller, &Target::Object(other), &registry) == on);
        let lost: bool = kani::any();
        state.players[1].lost = lost;
        assert!(behavior.is_valid_target(&state, controller, &Target::Player(PlayerId(1)), &registry) == !lost);
        assert!(!behavior.is_valid_target(&state, controller, &Target::Illegal, &registry));
        state.players[1].lost = false;
        if !(attached && torch_controller == controller) {
            std::mem::forget((registry, state));
            return;
        }
        let before = Snapshot::of(&state);
        behavior.pay_activation_cost(&mut state, creature, 1, &[], &registry);
        assert!(asked_for(&[Act::Sacrifice(torch)]) && Snapshot::of(&state) == before);
        // As the cost left it: the Torch in its owner's graveyard, knowing
        // what it was attached to.
        let o = state.get_object_mut(torch).expect("on the board");
        o.zone = Zone::Graveyard;
        o.attached_to = None;
        o.card_state.insert("last_attached_to".into(), creature);
        let target = any_target([other, creature]);
        let before = Snapshot::of(&state);
        behavior.resolve_activated_ability(&mut state, creature, 1, &[target.clone()], &registry);
        let to = match target {
            Target::Object(id) => Some(DamageTarget::Object(id)),
            Target::Player(p) => Some(DamageTarget::Player(p)),
            Target::Illegal => None,
        };
        match to {
            Some(to) => {
                assert!(acted() == 2 && act(1) == Some(Act::Damage { source: torch, target: to, amount: 2, kind: DamageKind::NonCombat }));
            }
            None => {
                assert!(acted() == 1);
            }
        }
        assert!(Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

card_proof! { taking_milled
    /// Trepanation Blade: its creature attacking, the defending player
    /// reveals from the top of their library until a land card, and every
    /// card revealed, that land among them, goes to their graveyard; the
    /// creature gets +1/+0 until end of turn for each, if it is still on
    /// the battlefield (none revealed, nothing). Nothing else. Over the
    /// defending player's library of two nonland cards and a land card, the
    /// land anywhere among them or not there at all, and the other
    /// player's library of a land card.
    fn trepanation_blade_mills_to_a_land_and_pumps_for_each() {
        assert!(triggers(&isd::trepanation_blade::TrepanationBlade) == [(TriggerKind::Attacks, None)]);
        let (mut registry, mut state, blank, card) = board(Box::new(isd::trepanation_blade::TrepanationBlade));
        let township = registry.register(Box::new(isd::gavony_township::GavonyTownship));
        let attacker = PlayerId(u8::from(kani::any::<bool>()));
        let defender = PlayerId(1 - attacker.0);
        let on: bool = kani::any();
        let creature = state.create_object(blank, attacker, if on { Zone::Battlefield } else { Zone::Graveyard }, Some(2), Some(2));
        if !on {
            if attacker == PlayerId(0) {
                state.players[0].graveyard_order.push(creature);
            } else {
                state.players[1].graveyard_order.push(creature);
            }
        }
        let blade = state.create_object(card, attacker, Zone::Battlefield, None, None);
        if on {
            state.get_object_mut(blade).expect("just created").attached_to = Some(creature);
        }
        let n = [state.create_object(blank, defender, Zone::Library, None, None), state.create_object(blank, defender, Zone::Library, None, None)];
        let land = state.create_object(township, defender, Zone::Library, None, None);
        let theirs = state.create_object(township, attacker, Zone::Library, None, None);
        let library: Vec<ObjectId> = match kani::any::<u8>() % 4 {
            0 => vec![land, n[0], n[1]],
            1 => vec![n[0], land, n[1]],
            2 => vec![n[0], n[1], land],
            _ => vec![n[0], n[1]],
        };
        let revealed = library.iter().position(|id| *id == land).map_or(library.len(), |i| i + 1);
        if defender == PlayerId(0) {
            state.players[0].library_order = library.clone();
            state.players[1].library_order.push(theirs);
        } else {
            state.players[1].library_order = library.clone();
            state.players[0].library_order.push(theirs);
        }
        let before = Snapshot::of(&state);
        let attack = AttackInfo::new(creature, defender);
        registry.get(card).expect("registered").on_attacks(&mut state, blade, attack, &[], &registry);
        let mut milled = Vec::with_capacity(3);
        for id in &library[..revealed] {
            milled.push(Act::MillCard(*id));
        }
        assert!(asked_for(&milled));
        let added = &state.until_end_of_turn[before.until_end_of_turn()..];
        if on {
            let x = i32::try_from(revealed).expect("small");
            assert!(added == [T::ModifyPT { target: creature, power_mod: x, toughness_mod: 0 }]);
        } else {
            assert!(added.is_empty());
        }
        let mut after = before.without_until_end_of_turn();
        let rest = super::zone_rows(&library[revealed..]);
        if defender == PlayerId(0) {
            after.players[0].1 = rest;
        } else {
            after.players[1].1 = rest;
        }
        assert!(Snapshot::of(&state).without_until_end_of_turn() == after);
        std::mem::forget((registry, state, library, milled));
    }
}

card_proof! {
    /// Wooden Stake: its creature blocking, or becoming blocked by, a
    /// creature triggers it exactly when that creature is a Vampire; and
    /// resolving, that Vampire destroyed, unable to regenerate (whether it
    /// is still there to destroy is the action's). Nothing else.
    fn wooden_stake_destroys_a_vampire_in_combat_with_its_creature() {
        let stake = isd::wooden_stake::WoodenStake;
        assert!(triggers(&stake) == [(TriggerKind::Blocks, None), (TriggerKind::BecomesBlocked, None)]);
        const VAMPIRES: Kinds = Kinds { types: &[crate::types::CardType::Creature], subtypes: &["Vampire"], keywords: &[] };
        let (registry, mut state, blank, card) = board(Box::new(stake));
        let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
        let other = any_object(&mut state, blank, &VAMPIRES, Form::Token);
        let behavior = registry.get(card).expect("registered");
        let vampire = state.get_object(other).expect("on the board").subtypes.iter().any(|s| s == "Vampire");
        assert!(behavior.should_trigger_on_blocks(&state, me, other, &registry) == vampire);
        assert!(behavior.should_trigger_on_becomes_blocked(&state, me, other, &registry) == vampire);
        let before = Snapshot::of(&state);
        if kani::any() {
            behavior.on_blocks(&mut state, me, other, &registry);
        } else {
            behavior.on_becomes_blocked(&mut state, me, other, &registry);
        }
        assert!(asked_for(&[Act::DestroyNoRegen(other)]) && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}
