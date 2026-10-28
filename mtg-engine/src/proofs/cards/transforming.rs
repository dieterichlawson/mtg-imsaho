//! The double-faced cards that ask their controller whether to transform
//! (CR 701.28, 712).
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

use super::{acted, asked_for, board, card_proof, case_by_case, picked, triggers, Act, Snapshot};
use crate::cards::{CardBehavior, CardRegistry, TriggerKind, TriggerScope};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
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

    /// Whether the creature is on the battlefield front face up.
    fn front_up(&self) -> bool {
        let o = self.state.get_object(self.me).expect("on the board");
        o.zone == Zone::Battlefield && !o.is_transformed
    }

    /// Whether its controller has been asked yes or no about it.
    fn asked(&self) -> bool {
        matches!(&self.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
            player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
        }) if *player == self.you && *source == self.me && *source_card == self.me)
    }

    fn upkeep(&mut self) -> Snapshot {
        let before = Snapshot::of(&self.state);
        let card = self.card();
        self.registry.get(card).expect("registered").on_upkeep(&mut self.state, self.me, &[], &self.registry);
        before
    }

    /// The answer to the question, after it was asked.
    fn answer(&mut self, yes: bool) {
        self.state.awaiting_action = None;
        let card = self.card();
        self.registry.get(card).expect("registered").on_yes_no_choice(&mut self.state, self.me, yes, &self.registry);
    }
}

/// The back face's triggered abilities, as the engine reads them with it up.
fn back_triggers(card: &dyn CardBehavior) -> Vec<TriggerKind> {
    let back = std::mem::ManuallyDrop::new(card.back_face_data().expect("a back face"));
    back.triggered_abilities.iter().map(|t| t.kind.clone()).collect()
}

/// Delver of Secrets: at its controller's upkeep, front face up, they
/// look at the top card of their library and are asked whether to
/// reveal it (with no card there, nothing to look at, nothing asked);
/// revealed and an instant or sorcery card, Delver transforms; not
/// revealed, or another card, nothing. Back face up, or gone, nothing.
///
/// Its controller, and the top card of their library (Lightning
/// Bolt, Lava Axe, the blank card, or none), are constants in each
/// harness: a card the solver chose is an index into the registry it
/// leaves open, a library made or not a list of a length it leaves
/// open, and a controller it chose an index into the players.
fn delver_of_secrets_transforms_on_a_revealed_instant_or_sorcery(case: u8) {
    let delver = isd::delver_of_secrets::DelverOfSecrets;
    assert!(triggers(&delver) == [(TriggerKind::Upkeep, None)] && back_triggers(&delver).is_empty());
    assert!(delver.step_trigger_scope(&TriggerKind::Upkeep, false) == TriggerScope::Your);
    let mut b = creature_of(Box::new(isd::delver_of_secrets::DelverOfSecrets), PlayerId(case / 4));
    // It transforms only as the trigger says, never on its own.
    assert!(!isd::delver_of_secrets::DelverOfSecrets.should_transform(&b.state, b.me, &b.registry));
    let bolt = b.registry.register(Box::new(crate::cards::lightning_bolt::LightningBolt));
    let axe = b.registry.register(Box::new(crate::cards::lava_axe::LavaAxe));
    let which = case % 4;
    if which < 3 {
        let card = match which { 0 => bolt, 1 => axe, _ => CardId(1) };
        let top = b.state.create_object(card, b.you, Zone::Library, None, None);
        if b.you == PlayerId(0) {
            b.state.players[0].library_order.push(top);
        } else {
            b.state.players[1].library_order.push(top);
        }
    }
    let front_up = b.front_up();
    let before = b.upkeep();
    if front_up && which < 3 {
        assert!(b.asked() && acted() == 0);
        let yes: bool = kani::any();
        b.answer(yes);
        if yes && which < 2 {
            assert!(asked_for(&[Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
    } else {
        assert!(!b.asked() && acted() == 0);
    }
    assert!(Snapshot::of(&b.state) == before);
    std::mem::forget(b);
}

case_by_case! { delver_of_secrets_transforms_on_a_revealed_instant_or_sorcery;
    delver_of_secrets_of_the_first_player_over_lightning_bolt: 0,
    delver_of_secrets_of_the_first_player_over_lava_axe: 1,
    delver_of_secrets_of_the_first_player_over_a_blank_card: 2,
    delver_of_secrets_of_the_first_player_over_nothing: 3,
    delver_of_secrets_of_the_second_player_over_lightning_bolt: 4,
    delver_of_secrets_of_the_second_player_over_lava_axe: 5,
    delver_of_secrets_of_the_second_player_over_a_blank_card: 6,
    delver_of_secrets_of_the_second_player_over_nothing: 7,
}

card_proof! {
    /// Cloistered Youth: at its controller's upkeep, front face up, they
    /// are asked whether to transform it, and it transforms on "yes";
    /// Unholy Fiend: at its controller's end step, they lose 1 life.
    fn cloistered_youth_may_transform_and_unholy_fiend_costs_a_life() {
        let youth = isd::cloistered_youth::CloisteredYouth;
        assert!(triggers(&youth) == [(TriggerKind::Upkeep, None)] && back_triggers(&youth) == [TriggerKind::EndStep]);
        for back in [false, true] {
            assert!(youth.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
            assert!(youth.step_trigger_scope(&TriggerKind::EndStep, back) == TriggerScope::Your);
        }
        let mut b = creature(Box::new(youth));
        let front_up = b.front_up();
        let before = b.upkeep();
        if front_up {
            assert!(b.asked() && acted() == 0);
            let yes: bool = kani::any();
            b.answer(yes);
            if yes {
                assert!(asked_for(&[Act::Transform(b.me)]));
            } else {
                assert!(acted() == 0);
            }
        } else {
            assert!(!b.asked() && acted() == 0);
            let card = b.card();
            b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[], &b.registry);
            assert!(asked_for(&[Act::Life(b.you, -1)]));
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Screeching Bat // Stalking Vampire: at its controller's upkeep, on
    /// the battlefield either face up, they are asked whether to pay
    /// {2}{B}{B} if they can (with no way to pay it the "may" has one
    /// answer, and nothing is asked); on "yes" they pay it and, if it was
    /// paid and the creature is still there, it transforms.
    fn screeching_bat_may_pay_to_transform_either_way() {
        let bat = isd::screeching_bat::ScreechingBat;
        assert!(triggers(&bat) == [(TriggerKind::Upkeep, None)] && back_triggers(&bat) == [TriggerKind::Upkeep]);
        for back in [false, true] {
            assert!(bat.step_trigger_scope(&TriggerKind::Upkeep, back) == TriggerScope::Your);
        }
        let mut b = creature(Box::new(bat));
        let on = b.state.get_object(b.me).expect("on the board").zone == Zone::Battlefield;
        let before = b.upkeep();
        if b.asked() {
            assert!(on && acted() == 0);
            let yes: bool = kani::any();
            b.answer(yes);
            let cost = crate::types::ManaCost::new(vec![
                crate::types::ManaSymbol::Generic(2),
                crate::types::ManaSymbol::Colored(crate::types::Color::Black),
                crate::types::ManaSymbol::Colored(crate::types::Color::Black),
            ]);
            match super::act(0) {
                None => {
                    assert!(!yes);
                }
                Some(Act::Pay(p, paid_cost, paid)) => {
                    assert!(yes && *p == b.you && *paid_cost == cost);
                    if *paid {
                        assert!(acted() == 2 && Some(&Act::Transform(b.me)) == super::act(1));
                    } else {
                        assert!(acted() == 1);
                    }
                }
                _ => panic!("pays first"),
            }
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Thraben Sentry // Thraben Militia: triggers, front face up, for
    /// another creature its controller controlled as it died, and for no
    /// other death (back face up, it has no such ability); its controller
    /// is asked whether to transform it, still there front face up, and it
    /// transforms on "yes".
    fn thraben_sentry_may_transform_when_another_creature_of_yours_dies() {
        let sentry = isd::thraben_sentry::ThrabenSentry;
        assert!(triggers(&sentry) == [(TriggerKind::AnyCreatureDies, None)] && back_triggers(&sentry).is_empty());
        let mut b = creature(Box::new(sentry));
        let transformed = b.state.get_object(b.me).expect("on the board").is_transformed;
        let dead_controller = PlayerId(u8::from(kani::any::<bool>()));
        let card = b.card();
        let fires = b.registry.get(card).expect("registered").should_trigger_on_creature_dies(
            &b.state, b.me, ObjectId(100), dead_controller, &[], 1, false, &[], &b.registry);
        assert!(fires == (!transformed && dead_controller == b.you));
        let front_up = b.front_up();
        let before = Snapshot::of(&b.state);
        b.registry.get(card).expect("registered").on_any_creature_dies(
            &mut b.state, b.me, ObjectId(100), dead_controller, &[], 1, false, &[], &b.registry);
        if front_up {
            assert!(b.asked() && acted() == 0);
            let yes: bool = kani::any();
            b.answer(yes);
            if yes {
                assert!(asked_for(&[Act::Transform(b.me)]));
            } else {
                assert!(acted() == 0);
            }
        } else {
            assert!(!b.asked() && acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

/// The card's activated abilities as the engine reads them, as the
/// activated-ability proofs pin them.
fn abilities(b: &Board) -> std::mem::ManuallyDrop<Vec<super::activated::Ability>> {
    let card = b.card();
    let defs = std::mem::ManuallyDrop::new(b.registry.get(card).expect("registered").activated_abilities(&b.state, b.me, &b.registry));
    std::mem::ManuallyDrop::new(defs.iter().enumerate().map(|(i, d)| {
        assert!(d.ability_index == i);
        super::activated::Ability::of(d)
    }).collect())
}

/// Puts a card in `you`'s `zone`: Grizzly Bears, a creature card, or the
/// blank card, which is none.
fn card_of_yours(b: &mut Board, bears: CardId, zone: Zone) -> ObjectId {
    let card = if kani::any() { bears } else { CardId(1) };
    let id = b.state.create_object(card, b.you, zone, None, None);
    if zone == Zone::Library {
        if b.you == PlayerId(0) {
            b.state.players[0].library_order.push(id);
        } else {
            b.state.players[1].library_order.push(id);
        }
    }
    id
}

card_proof! {
    /// Civilized Scholar // Homicidal Brute. Front face up on the
    /// battlefield, "{T}: Draw a card, then discard a card": its
    /// controller draws, then discards a card of their hand as it then is
    /// (the only one without being asked, else the one they choose); a
    /// creature card discarded, the Scholar untaps and then transforms.
    fn civilized_scholar_draws_discards_and_transforms_on_a_creature() {
        let mut b = creature(Box::new(isd::civilized_scholar::CivilizedScholar));
        let front_up = b.front_up();
        assert!(abilities(&b).len() == usize::from(front_up));
        if front_up {
            assert!(abilities(&b)[0] == super::activated::Ability { tap: true, ..super::activated::Ability::default() });
        }
        let bears = b.registry.register(Box::new(crate::cards::grizzly_bears::GrizzlyBears));
        let in_library: bool = kani::any();
        if in_library {
            card_of_yours(&mut b, bears, Zone::Library);
        }
        let in_hand: bool = kani::any();
        if in_hand {
            card_of_yours(&mut b, bears, Zone::Hand);
        }
        b.state.resolving_ability_activator = Some(b.you);
        let card = b.card();
        b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, 0, &[], &b.registry);
        let hand = picked(&b.state.objects_in_zone(Zone::Hand, b.you), |o| Some(o.id));
        let bears_in_hand = picked(&hand, |id| Some(b.state.get_object(*id).expect("on the board").card_id == bears));
        let creature_card = |id: ObjectId| hand.iter().zip(&bears_in_hand).any(|(h, is)| *h == id && *is);
        assert!(Some(&Act::Draw(b.you, 1)) == super::act(0));
        match hand.as_slice() {
            [] => {
                assert!(acted() == 1);
            }
            [only] if creature_card(*only) => {
                assert!(asked_for(&[Act::Draw(b.you, 1), Act::Discard(*only), Act::Untap(b.me), Act::Transform(b.me)]));
            }
            [only] => {
                assert!(asked_for(&[Act::Draw(b.you, 1), Act::Discard(*only)]));
            }
            _ => {
                let asked = matches!(&b.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
                    player, source, choice: ResolutionChoiceKind::ChooseCardFromHand { player: chooser, cards, discard_immediately: true, remaining: 1, .. },
                }) if *player == b.you && *source == b.me && *chooser == b.you && *cards == hand);
                assert!(asked && acted() == 1);
                let discarded = hand[usize::from(kani::any::<bool>())];
                b.state.awaiting_action = None;
                b.registry.get(card).expect("registered").on_discard_choice(&mut b.state, b.me, discarded, &b.registry);
                if creature_card(discarded) {
                    assert!(asked_for(&[Act::Draw(b.you, 1), Act::Untap(b.me), Act::Transform(b.me)]));
                } else {
                    assert!(acted() == 1);
                }
            }
        }
        std::mem::forget((b, hand, bears_in_hand));
    }
}

card_proof! {
    /// Homicidal Brute: triggers at its controller's end step (as the back
    /// face's ability, CR 712.8d), if it didn't attack this turn, checked
    /// as it triggers and as it resolves; tapped, then transformed.
    fn homicidal_brute_taps_and_transforms_if_it_did_not_attack() {
        let scholar = isd::civilized_scholar::CivilizedScholar;
        assert!(triggers(&scholar).is_empty() && back_triggers(&scholar) == [TriggerKind::EndStep]);
        assert!(scholar.step_trigger_scope(&TriggerKind::EndStep, true) == TriggerScope::Your);
        let mut b = creature(Box::new(scholar));
        let attacked: bool = kani::any();
        if attacked {
            b.state.get_object_mut(b.me).expect("on the board").attacked_on_turn = Some(b.state.turn_number);
        }
        let card = b.card();
        assert!(b.registry.get(card).expect("registered").should_trigger(&b.state, b.me, &TriggerKind::EndStep, &b.registry) == !attacked);
        let o = b.state.get_object(b.me).expect("on the board");
        let back_up = o.zone == Zone::Battlefield && o.is_transformed;
        let before = Snapshot::of(&b.state);
        b.registry.get(card).expect("registered").on_end_step(&mut b.state, b.me, &[], &b.registry);
        if back_up && !attacked {
            assert!(asked_for(&[Act::Tap(b.me), Act::Transform(b.me)]));
        } else {
            assert!(acted() == 0);
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget(b);
    }
}

card_proof! {
    /// Bloodline Keeper // Lord of Lineage: either face, "{T}: Create a 2/2
    /// black Vampire creature token with flying"; front face up, a card,
    /// and with five or more Vampires its controller controls, "{B}:
    /// Transform this creature"; back face up, "Other Vampire creatures you
    /// control get +2/+2". Over four tokens, each either player's and a
    /// Vampire or not.
    fn bloodline_keeper_makes_vampires_and_transforms_at_five() {
        let keeper = isd::bloodline_keeper::BloodlineKeeper;
        let back = std::mem::ManuallyDrop::new(keeper.back_face_data().expect("a back face"));
        assert!(matches!(back.continuous_effects.as_slice(), [crate::types::ContinuousEffect::ModifyPT {
            power: 2, toughness: 2, scope: crate::types::EffectScope::GlobalOther(f),
        }] if super::both(f, |f| matches!(f, crate::types::CreatureFilter::ControlledByYou), |f| super::has_subtype(f, "Vampire"))));
        let mut b = creature(Box::new(keeper));
        let mut vampires = 1;
        for _ in 0..4 {
            let controller = PlayerId(u8::from(kani::any::<bool>()));
            let id = b.state.create_object(CardId(0), controller, Zone::Battlefield, Some(2), Some(2));
            let o = b.state.get_object_mut(id).expect("just created");
            o.is_token = true;
            o.card_types = vec![crate::types::CardType::Creature];
            o.subtypes = Vec::with_capacity(1);
            if kani::any() {
                o.subtypes.push("Vampire".into());
                if controller == b.you {
                    vampires += 1;
                }
            }
        }
        let o = b.state.get_object(b.me).expect("on the board");
        let on = o.zone == Zone::Battlefield;
        let front_up = on && !o.is_transformed;
        let make = super::activated::Ability { tap: true, ..super::activated::Ability::default() };
        let transform = super::activated::Ability {
            cost: crate::types::ManaCost::new(vec![crate::types::ManaSymbol::Colored(crate::types::Color::Black)]),
            ..super::activated::Ability::default()
        };
        let got = abilities(&b);
        if !on {
            assert!(got.is_empty());
        } else if front_up && vampires >= 5 {
            assert!(got.len() == 2 && got[0] == make && got[1] == transform);
        } else {
            assert!(got.len() == 1 && got[0] == make);
        }
        b.state.resolving_ability_activator = Some(b.you);
        let before = Snapshot::of(&b.state);
        let index = usize::from(kani::any::<bool>());
        let card = b.card();
        b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, index, &[], &b.registry);
        if index == 0 {
            assert!(asked_for(&[Act::Tokens {
                n: 1, controller: b.you, power: 2, toughness: 2, colors: vec![crate::types::Color::Black],
                types: vec![crate::types::CardType::Creature], keywords: vec![crate::types::Keyword::Flying], subtypes: vec!["Vampire".into()],
            }]));
        } else {
            assert!(asked_for(&[Act::Transform(b.me)]));
        }
        assert!(Snapshot::of(&b.state) == before);
        std::mem::forget((b, got));
    }
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
        assert!(got.len() == 1 && got[0] == super::activated::Ability {
            cost: crate::types::ManaCost::new(vec![
                crate::types::ManaSymbol::Generic(1), crate::types::ManaSymbol::Colored(crate::types::Color::Blue),
            ]),
            ..super::activated::Ability::default()
        });
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

card_proof! { adding_counters
    /// Ludevic's Test Subject's ability resolving: a hatchling counter on
    /// it, and then, with five or more there, all of them removed and it
    /// transformed. Over any number of hatchling counters from none to five
    /// already on it.
    fn ludevics_test_subject_hatches_at_five_counters() {
        let mut b = creature(Box::new(isd::ludevics_test_subject::LudevicsTestSubject));
        let had: u32 = kani::any();
        kani::assume(had <= 5);
        if had > 0 {
            b.state.get_object_mut(b.me).expect("on the board").counters.insert(crate::types::CounterType::Hatchling, had);
        }
        let on = b.state.get_object(b.me).expect("on the board").zone == Zone::Battlefield;
        let card = b.card();
        b.registry.get(card).expect("registered").resolve_activated_ability(&mut b.state, b.me, 0, &[], &b.registry);
        let now = had + u32::from(on);
        let add = Act::Counters(b.me, crate::types::CounterType::Hatchling, 1);
        if now >= 5 {
            assert!(asked_for(&[add, Act::RemoveCounters(b.me, crate::types::CounterType::Hatchling, now), Act::Transform(b.me)]));
        } else {
            assert!(asked_for(&[add]));
        }
        std::mem::forget(b);
    }
}
