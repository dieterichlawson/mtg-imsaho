//! The double-faced cards that ask their controller whether to transform
//! (CR 701.28, 712).
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Delver of Secrets // Insectile Aberration: "At the beginning of your
//!   upkeep, look at the top card of your library. You may reveal that
//!   card. If an instant or sorcery card is revealed this way, transform
//!   this creature." // "Flying"
//! - Cloistered Youth // Unholy Fiend: "At the beginning of your upkeep,
//!   you may transform this creature." // "At the beginning of your end
//!   step, you lose 1 life."
//!
//! Each face's triggers are pinned, the engine reading the face that is up
//! (CR 712.8d, `proofs::actions`). Each proof runs the card's code over the
//! creature on the battlefield or gone by then, front face up or back,
//! under either player's control; and, for Delver, a library whose top
//! card is an instant (Lightning Bolt), a sorcery (Lava Axe), the blank
//! card, or no card at all.

use super::{board, card_proof, case_by_case, triggers};
use crate::cards::{CardBehavior, CardRegistry, TriggerKind, TriggerScope};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::GameState;
use crate::types::Zone;

use crate::cards::isd;

struct Board {
    registry: CardRegistry,
    state: GameState,
    me: ObjectId,
    you: PlayerId,
}

fn creature(card: Box<dyn CardBehavior>) -> Board {
    creature_of(card, PlayerId(u8::from(kani::any::<bool>())))
}

/// [`creature`]'s board, controlled (or last controlled) by `you`.
fn creature_of(card: Box<dyn CardBehavior>, you: PlayerId) -> Board {
    creature_at(card, you, kani::any(), kani::any())
}

/// [`creature_of`]'s board, the creature in its owner's graveyard exactly
/// if `gone`, and transformed exactly if `transformed`.
fn creature_at(card: Box<dyn CardBehavior>, you: PlayerId, gone: bool, transformed: bool) -> Board {
    let (registry, mut state, _, card) = board(card);
    let me = state.create_object(card, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Battlefield }, None, None);
    let o = state.get_object_mut(me).expect("just created");
    o.is_transformed = transformed;
    if gone {
        o.last_controller = Some(you);
        state.players[0].graveyard_order.push(me);
    } else {
        o.controller = you;
    }
    Board { registry, state, me, you }
}

impl Board {
    fn card(&self) -> CardId {
        self.state.get_object(self.me).expect("on the board").card_id
    }
}

/// The back face's triggered abilities, as the engine reads them with it up.
fn back_triggers(card: &dyn CardBehavior) -> Vec<TriggerKind> {
    let back = std::mem::ManuallyDrop::new(card.back_face_data().expect("a back face"));
    back.triggered_abilities.iter().map(|t| t.kind.clone()).collect()
}

card_proof! {
    /// Cloistered Youth // Unholy Fiend: an upkeep trigger on the front
    /// face and an end step trigger on the back, each its controller's
    /// alone. What they do is the proof below.
    fn cloistered_youth_and_unholy_fiend_trigger_at_your_upkeep_and_end_step() {
        let youth = isd::cloistered_youth::CloisteredYouth;
        assert!(triggers(&youth) == [(TriggerKind::Upkeep, None)] && back_triggers(&youth) == [TriggerKind::EndStep]);
        for back in [false, true] {
            assert!(youth.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
            assert!(youth.step_trigger_scope(&TriggerKind::EndStep, back) == TriggerScope::Your);
        }
    }
}

card_proof! {
    /// Screeching Bat // Stalking Vampire: an upkeep trigger on each face,
    /// its controller's alone. What it does is the proof below.
    fn screeching_bat_triggers_at_your_upkeep_either_way() {
        let bat = isd::screeching_bat::ScreechingBat;
        assert!(triggers(&bat) == [(TriggerKind::Upkeep, None)] && back_triggers(&bat) == [TriggerKind::Upkeep]);
        for back in [false, true] {
            assert!(bat.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
        }
    }
}

card_proof! {
    /// Thraben Sentry // Thraben Militia: triggers, front face up, for
    /// another creature its controller controlled as it died, and for no
    /// other death (back face up, it has no such ability). What it does is
    /// the proof below.
    fn thraben_sentry_triggers_when_another_creature_of_yours_dies() {
        let sentry = isd::thraben_sentry::ThrabenSentry;
        assert!(triggers(&sentry) == [(TriggerKind::AnyCreatureDies, None)] && back_triggers(&sentry).is_empty());
        let b = creature(Box::new(sentry));
        let transformed = b.state.get_object(b.me).expect("on the board").is_transformed;
        let dead_controller = PlayerId(u8::from(kani::any::<bool>()));
        let card = b.card();
        let fires = b.registry.get(card).expect("registered").should_trigger_on_creature_dies(
            &b.state, b.me, ObjectId(100), dead_controller, &[], 1, false, &[], &b.registry);
        assert!(fires == (!transformed && dead_controller == b.you));
        std::mem::forget(b);
    }
}

/// The card's activated abilities as the engine reads them, each at its
/// index; compared with `Ability::describes`, as the activated-ability
/// proofs pin them.
fn abilities(b: &Board) -> std::mem::ManuallyDrop<Vec<crate::cards::ActivatedAbilityDef>> {
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    for (i, d) in defs.iter().enumerate() {
        assert!(d.ability_index == i);
    }
    defs
}

/// Civilized Scholar // Homicidal Brute: front face up on the battlefield,
/// "{T}: Draw a card, then discard a card"; transformed or off the
/// battlefield, no ability. Its resolution is the proof below. Where it is
/// and which face is up are constants in each harness, as for Ulvenwald
/// Mystics: whether it has the ability is a list of abilities of a length
/// they decide, and with them the solver's, and the resolution in the same
/// harness, the proof ran out of memory at four and a half gigabytes in
/// symbolic execution. In its owner's graveyard for 0, front face up on
/// the battlefield for 1, and transformed there for 2.
fn civilized_scholar_placed(placed: u8) {
    let b = creature_at(Box::new(isd::civilized_scholar::CivilizedScholar), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let got = abilities(&b);
    if placed == 1 {
        assert!(got.len() == 1 && super::activated::Ability { tap: true, ..super::activated::Ability::default() }.describes(&got[0]));
    } else {
        assert!(got.is_empty());
    }
    std::mem::forget((b, got));
}

case_by_case! { civilized_scholar_placed;
    civilized_scholar_gone_has_no_ability: 0,
    civilized_scholar_face_up_has_its_ability: 1,
    homicidal_brute_has_no_ability: 2,
}

card_proof! {
    /// Lord of Lineage, Bloodline Keeper's back face: "Other Vampire
    /// creatures you control get +2/+2". Its abilities and their
    /// resolutions are the proofs below.
    fn lord_of_lineage_pumps_your_other_vampires() {
        let back = std::mem::ManuallyDrop::new(isd::bloodline_keeper::BloodlineKeeper.back_face_data().expect("a back face"));
        assert!(matches!(back.continuous_effects.as_slice(), [crate::types::ContinuousEffect::ModifyPT {
            power: 2, toughness: 2, scope: crate::types::EffectScope::GlobalOther(f),
        }] if super::both(f, |f| matches!(f, crate::types::CreatureFilter::ControlledByYou), |f| super::has_subtype(f, "Vampire"))));
    }
}

/// Bloodline Keeper // Lord of Lineage: either face, "{T}: Create a 2/2
/// black Vampire creature token with flying"; front face up, a card, and
/// with five or more Vampires its controller controls, "{B}: Transform
/// this creature"; off the battlefield, no ability. Over four tokens
/// beside it, each either player's and a Vampire or not; with it in its
/// owner's graveyard for `placed` 0, front face up on the battlefield for
/// 1 and transformed there for 2, and, front face up, `yours` of the
/// tokens Vampires of its controller and the others each a Vampire of the
/// other player or not a Vampire, each a constant in its harness: the
/// abilities are a list of a length they decide, and with them the
/// solver's the proof ran out of memory at four and a half gigabytes in
/// symbolic execution.
fn bloodline_keeper_has_its_abilities((placed, yours): (u8, Option<u8>)) {
    let mut b = creature_at(Box::new(isd::bloodline_keeper::BloodlineKeeper), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let mut vampires = 1;
    for k in 0..4 {
        // Either player's and a Vampire or not; or, `yours` given, the
        // first `yours` Vampires of `you`, and each other one not a
        // Vampire of `you`.
        let (controller, vampire) = match yours {
            None => (PlayerId(u8::from(kani::any::<bool>())), kani::any()),
            Some(n) if k < n => (b.you, true),
            Some(_) => {
                let vampire: bool = kani::any();
                (if vampire { PlayerId(1 - b.you.0) } else { PlayerId(u8::from(kani::any::<bool>())) }, vampire)
            }
        };
        let id = b.state.create_object(CardId(0), controller, Zone::Battlefield, Some(2), Some(2));
        let o = b.state.get_object_mut(id).expect("just created");
        o.is_token = true;
        o.card_types = vec![crate::types::CardType::Creature];
        o.subtypes = Vec::with_capacity(1);
        if vampire {
            o.subtypes.push("Vampire".into());
            if controller == b.you {
                vampires += 1;
            }
        }
    }
    let make = super::activated::Ability { tap: true, ..super::activated::Ability::default() };
    let transform = super::activated::Ability {
        cost: crate::types::ManaCost::new(vec![crate::types::ManaSymbol::Colored(crate::types::Color::Black)]),
        ..super::activated::Ability::default()
    };
    let got = abilities(&b);
    if placed == 0 {
        assert!(got.is_empty());
    } else if placed == 1 && vampires >= 5 {
        assert!(got.len() == 2 && make.describes(&got[0]) && transform.describes(&got[1]));
    } else {
        assert!(got.len() == 1 && make.describes(&got[0]));
    }
    std::mem::forget((b, got, make, transform));
}

case_by_case! { bloodline_keeper_has_its_abilities;
    bloodline_keeper_gone_has_no_ability: (0, None),
    lord_of_lineage_makes_vampires: (2, None),
    bloodline_keeper_with_no_other_vampire_of_yours_only_makes_vampires: (1, Some(0)),
    bloodline_keeper_with_one_other_vampire_of_yours_only_makes_vampires: (1, Some(1)),
    bloodline_keeper_with_two_other_vampires_of_yours_only_makes_vampires: (1, Some(2)),
    bloodline_keeper_with_three_other_vampires_of_yours_only_makes_vampires: (1, Some(3)),
    bloodline_keeper_with_four_other_vampires_of_yours_may_transform: (1, Some(4)),
}

/// Ludevic's Test Subject // Ludevic's Abomination: front face up on the
/// battlefield, "{1}{U}: Put a hatchling counter on this creature. Then if
/// there are five or more hatchling counters on it, remove all of them
/// and transform it"; transformed or off the battlefield, no ability. Its
/// resolution is the proof below. Where it is and which face is up are
/// constants in each harness, as for Ulvenwald Mystics: whether it has
/// the ability is a list of abilities of a length they decide, and with
/// them the solver's, and the resolution in the same harness, the proof
/// ran out of memory at four and a half gigabytes in symbolic execution.
/// In its owner's graveyard for 0, front face up on the battlefield for
/// 1, and transformed there for 2.
fn ludevics_test_subject_placed(placed: u8) {
    let b = creature_at(Box::new(isd::ludevics_test_subject::LudevicsTestSubject), PlayerId(u8::from(kani::any::<bool>())), placed == 0, placed == 2);
    let got = abilities(&b);
    if placed == 1 {
        assert!(got.len() == 1 && super::activated::Ability {
            cost: crate::types::ManaCost::new(vec![
                crate::types::ManaSymbol::Generic(1), crate::types::ManaSymbol::Colored(crate::types::Color::Blue),
            ]),
            ..super::activated::Ability::default()
        }.describes(&got[0]));
    } else {
        assert!(got.is_empty());
    }
    std::mem::forget((b, got));
}

case_by_case! { ludevics_test_subject_placed;
    ludevics_test_subject_gone_has_no_ability: 0,
    ludevics_test_subject_face_up_has_its_ability: 1,
    ludevics_abomination_has_no_ability: 2,
}

